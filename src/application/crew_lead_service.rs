//! Crew Lead application service. See `specs/02-crew-lead.md` (CL).

use crate::domain::crew_lead::{CrewLead, CrewLeadId};
use crate::domain::errors::DomainError;

/// In-process service holding the canonical list of Crew Leads.
///
/// Exactly three Crew Leads exist after `bootstrap` (CL-I1).
pub struct CrewLeadService {
    leads: Vec<CrewLead>,
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
        Ok(Self { leads })
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
}
