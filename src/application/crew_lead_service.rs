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
        let _ = leads;
        todo!("CL-R1: implement bootstrap")
    }

    /// CL-R2 — always rejected because the cap is already at 3.
    ///
    /// # Errors
    /// Returns `DomainError::CrewLeadLimitReached` (CL-E1).
    pub fn add(&mut self, lead: CrewLead) -> Result<(), DomainError> {
        let _ = lead;
        todo!("CL-R2: implement add")
    }

    /// CL-R3 — always rejected because removal would breach the
    /// exactly-3 invariant. Use `replace` to rotate a lead instead.
    ///
    /// # Errors
    /// Returns `DomainError::CrewLeadMinimumBreached` (CL-E2).
    pub fn remove(&mut self, id: &CrewLeadId) -> Result<(), DomainError> {
        let _ = id;
        todo!("CL-R3: implement remove")
    }

    /// CL-R4 — atomically swap `old_id` for `new_lead`.
    ///
    /// # Errors
    /// - `DomainError::CrewLeadNotFound` (CL-E4) if `old_id` is unknown.
    /// - `DomainError::CrewLeadAlreadyExists` (CL-E3) if `new_lead.id`
    ///   matches a different existing lead.
    pub fn replace(&mut self, old_id: &CrewLeadId, new_lead: CrewLead) -> Result<(), DomainError> {
        let _ = (old_id, new_lead);
        todo!("CL-R4: implement replace")
    }

    /// CL-R6 — current Crew Leads in insertion order.
    #[must_use]
    pub fn list(&self) -> &[CrewLead] {
        &self.leads
    }
}
