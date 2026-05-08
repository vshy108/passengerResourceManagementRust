//! Crew Lead application service. See `specs/02-crew-lead.md` (CL).

use uuid::Uuid;

use crate::application::ports::{AdminEventSink, Clock};
use crate::domain::admin_event::{AdminAction, AdminEvent, TargetKind};
use crate::domain::crew_lead::{CrewLead, CrewLeadId};
use crate::domain::errors::DomainError;

/// In-process service holding the canonical list of Crew Leads.
///
/// Exactly three Crew Leads exist after `bootstrap` (CL-I1).
pub struct CrewLeadService {
    // `Vec<T>` is Rust's growable heap-allocated array (like
    // `ArrayList`/`std::vector`). Iteration order = insertion order.
    leads: Vec<CrewLead>,
    // Audit support is OPT-IN: `None` for the plain ctor, `Some(...)`
    // when constructed via `bootstrap_audited`. Keeping it Option means
    // callers that don't need audit don't pay for it.
    audit: Option<AuditCfg>,
}

// `struct` (no `pub`) = visible only inside this module — implementation
// detail of the service.
struct AuditCfg {
    clock: Box<dyn Clock>,
    sink: Box<dyn AdminEventSink>,
}

impl CrewLeadService {
    /// CL-R1 — seed the system with exactly three distinct leads.
    ///
    /// # Errors
    /// Returns `DomainError::CrewLeadBootstrapInvalid` (CL-E5) if the
    /// input count is not 3 or contains duplicate ids.
    // Associated function (no `self`) — called as `CrewLeadService::bootstrap(...)`.
    // Acts as a *named constructor*.
    pub fn bootstrap(leads: Vec<CrewLead>) -> Result<Self, DomainError> {
        if leads.len() != 3 {
            return Err(DomainError::CrewLeadBootstrapInvalid);
        }
        // CL-I2 — reject duplicate ids. O(n²) is fine for n=3.
        // `0..leads.len()` is a half-open range (0,1,2); `(i+1)..len`
        // skips already-compared pairs.
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
    // Leading underscore on `_lead` tells the compiler "I'm intentionally
    // ignoring this parameter" — silences the unused-variable warning.
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
        // `position` returns `Option<usize>` — index of the first match.
        // `let-else` bails out cleanly on None; on Some, `slot` is bound.
        let Some(slot) = self.leads.iter().position(|l| l.id == *old_id) else {
            return Err(DomainError::CrewLeadNotFound);
        };
        // CL-E3 — reject if `new_lead.id` collides with a *different*
        // existing lead. Replacing in place (same id) is allowed.
        // `enumerate()` pairs each item with its index.
        // `any(|...| ...)` short-circuits on first match.
        if self
            .leads
            .iter()
            .enumerate()
            .any(|(i, l)| i != slot && l.id == new_lead.id)
        {
            return Err(DomainError::CrewLeadAlreadyExists);
        }
        // Index assignment — replaces the element at `slot` in place.
        self.leads[slot] = new_lead;
        Ok(())
    }

    /// CL-R6 — current Crew Leads in insertion order.
    #[must_use]
    pub fn list(&self) -> &[CrewLead] {
        // `&self.leads` (a `&Vec<CrewLead>`) auto-coerces to `&[CrewLead]`
        // (a slice). Slice is a smaller, more general API — callers that
        // only need to read benefit.
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
        // The `?` operator: if `bootstrap` returns Err, the whole
        // function returns that Err immediately. Otherwise the Ok value
        // is unwrapped into `svc`.
        let mut svc = Self::bootstrap(leads)?;
        let mut audit = AuditCfg {
            clock,
            sink,
        };
        // Use the first lead as the acting Crew Lead for the bootstrap
        // event; this is a synthetic but stable choice (AU-R2 only
        // requires `actor_id` be a Crew Lead id).
        let actor_id = svc.leads[0].id.clone();
        // `.0` accesses the inner `String` of the `CrewLeadId(pub String)`
        // tuple struct.
        let target_id = actor_id.0.clone();
        let event = AdminEvent {
            // FIX: was u64 counter (reset on restart); UUID v4 is stable
            // once persisted.
            id: Uuid::new_v4().to_string(),
            actor_id,
            action: AdminAction::CrewLeadBootstrapped,
            target_kind: TargetKind::CrewLead,
            target_id,
            timestamp: audit.clock.now(),
            // `format!` is `println!` that returns a String instead of
            // printing. `{}` formats with Display, `{:?}` with Debug.
            details: Some(format!("count={}", svc.leads.len())),
        };
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
        // Clone BEFORE the call because `new_lead` is moved into
        // `replace` and we still need its id afterwards.
        let new_id = new_lead.id.clone();
        self.replace(old_id, new_lead)?;
        // `if let Some(...)` = run this block only when audit is enabled.
        // `.as_mut()` converts `&mut Option<T>` -> `Option<&mut T>`,
        // letting us mutate the inner value without taking ownership.
        if let Some(audit) = self.audit.as_mut() {
            let event = AdminEvent {
                id: Uuid::new_v4().to_string(),
                actor_id: actor_id.clone(),
                action: AdminAction::CrewLeadReplaced,
                target_kind: TargetKind::CrewLead,
                target_id: new_id.0,
                timestamp: audit.clock.now(),
                details: Some(format!("replaced={}", old_id.0)),
            };
            audit.sink.append(event);
        }
        Ok(())
    }
}
