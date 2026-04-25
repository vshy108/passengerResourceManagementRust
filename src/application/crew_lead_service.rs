//! Crew Lead application service. See `specs/02-crew-lead.md` (CL).

use crate::application::ports::{AdminEventSink, Clock};
use crate::domain::admin_event::{AdminAction, AdminEvent, TargetKind};
use crate::domain::crew_lead::{CrewLead, CrewLeadId};
use crate::domain::errors::DomainError;

/// In-process service holding the canonical list of Crew Leads.
///
/// Exactly three Crew Leads exist after `bootstrap` (CL-I1).
pub struct CrewLeadService {
    leads: Vec<CrewLead>,
    audit: Option<AuditCfg>,
}

struct AuditCfg {
    clock: Box<dyn Clock>,
    sink: Box<dyn AdminEventSink>,
    next_id: u64,
}

impl CrewLeadService {
    /// CL-R1 — seed the system with exactly three distinct leads.
    ///
    /// # Errors
    /// Returns `DomainError::CrewLeadBootstrapInvalid` (CL-E5) if the
    /// input count is not 3 or contains duplicate ids.
    pub fn bootstrap(leads: Vec<CrewLead>) -> Result<Self, DomainError> {
        if leads.len() != 3 {
            return Err(DomainError::CrewLeadBootstrapInvalid);
        }
        // CL-I2 — reject duplicate ids.
        for i in 0..leads.len() {
            for j in (i + 1)..leads.len() {
                if leads[i].id == leads[j].id {
                    return Err(DomainError::CrewLeadBootstrapInvalid);
                }
            }
        }
        Ok(Self { leads, audit: None })
    }

    /// CL-R2 — always rejected because the cap is already at 3.
    ///
    /// # Errors
    /// Returns `DomainError::CrewLeadLimitReached` (CL-E1).
    pub fn add(&mut self, _lead: CrewLead) -> Result<(), DomainError> {
        Err(DomainError::CrewLeadLimitReached)
    }

    /// CL-R3 — always rejected because removal would breach the
    /// exactly-3 invariant. Use `replace` to rotate a lead instead.
    ///
    /// # Errors
    /// Returns `DomainError::CrewLeadMinimumBreached` (CL-E2).
    pub fn remove(&mut self, _id: &CrewLeadId) -> Result<(), DomainError> {
        Err(DomainError::CrewLeadMinimumBreached)
    }

    /// CL-R4 — atomically swap `old_id` for `new_lead`.
    ///
    /// # Errors
    /// - `DomainError::CrewLeadNotFound` (CL-E4) if `old_id` is unknown.
    /// - `DomainError::CrewLeadAlreadyExists` (CL-E3) if `new_lead.id`
    ///   matches a different existing lead.
    pub fn replace(&mut self, old_id: &CrewLeadId, new_lead: CrewLead) -> Result<(), DomainError> {
        let Some(slot) = self.leads.iter().position(|l| l.id == *old_id) else {
            return Err(DomainError::CrewLeadNotFound);
        };
        // CL-E3 — reject if `new_lead.id` collides with a *different*
        // existing lead. Replacing in place (same id) is allowed.
        if self
            .leads
            .iter()
            .enumerate()
            .any(|(i, l)| i != slot && l.id == new_lead.id)
        {
            return Err(DomainError::CrewLeadAlreadyExists);
        }
        self.leads[slot] = new_lead;
        Ok(())
    }

    /// CL-R6 — current Crew Leads in insertion order.
    #[must_use]
    pub fn list(&self) -> &[CrewLead] {
        &self.leads
    }

    /// AU-S7 — audit-aware bootstrap. Emits one
    /// `AdminAction::CrewLeadBootstrapped` event on success. The actor
    /// id is taken from the first lead in the input list.
    ///
    /// # Errors
    /// See [`bootstrap`].
    pub fn bootstrap_audited(
        leads: Vec<CrewLead>,
        clock: Box<dyn Clock>,
        sink: Box<dyn AdminEventSink>,
    ) -> Result<Self, DomainError> {
        let mut svc = Self::bootstrap(leads)?;
        let mut audit = AuditCfg {
            clock,
            sink,
            next_id: 1,
        };
        // Use the first lead as the acting Crew Lead for the bootstrap
        // event; this is a synthetic but stable choice (AU-R2 only
        // requires `actor_id` be a Crew Lead id).
        let actor_id = svc.leads[0].id.clone();
        let target_id = actor_id.0.clone();
        let event = AdminEvent {
            id: audit.next_id,
            actor_id,
            action: AdminAction::CrewLeadBootstrapped,
            target_kind: TargetKind::CrewLead,
            target_id,
            timestamp: audit.clock.now(),
            details: Some(format!("count={}", svc.leads.len())),
        };
        audit.next_id += 1;
        audit.sink.append(event);
        svc.audit = Some(audit);
        Ok(svc)
    }

    /// AU-S8 — audit-aware replace. Emits one
    /// `AdminAction::CrewLeadReplaced` event on success.
    ///
    /// # Errors
    /// See [`replace`].
    pub fn replace_audited(
        &mut self,
        actor_id: &CrewLeadId,
        old_id: &CrewLeadId,
        new_lead: CrewLead,
    ) -> Result<(), DomainError> {
        let new_id = new_lead.id.clone();
        self.replace(old_id, new_lead)?;
        if let Some(audit) = self.audit.as_mut() {
            let event = AdminEvent {
                id: audit.next_id,
                actor_id: actor_id.clone(),
                action: AdminAction::CrewLeadReplaced,
                target_kind: TargetKind::CrewLead,
                target_id: new_id.0,
                timestamp: audit.clock.now(),
                details: Some(format!("replaced={}", old_id.0)),
            };
            audit.next_id += 1;
            audit.sink.append(event);
        }
        Ok(())
    }
}
