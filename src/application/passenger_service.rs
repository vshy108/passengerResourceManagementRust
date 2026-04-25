//! Passenger application service. See `specs/03-passenger.md` (PS).

use crate::application::guards::require_crew_lead;
use crate::application::ports::Clock;
use crate::domain::actor::Actor;
use crate::domain::errors::DomainError;
use crate::domain::passenger::{Passenger, PassengerId};
use crate::domain::tier::Tier;

pub struct PassengerService<C: Clock> {
    /// Active passengers, in insertion order.
    active: Vec<Passenger>,
    /// Soft-deleted records (kept for audit lookups via `get`).
    deleted: Vec<Passenger>,
    clock: C,
}

impl<C: Clock> PassengerService<C> {
    #[must_use]
    pub fn new(clock: C) -> Self {
        Self {
            active: Vec::new(),
            deleted: Vec::new(),
            clock,
        }
    }

    /// PS-R1 — Crew-Lead-only create.
    ///
    /// # Errors
    /// - `UnauthorizedActor` (PS-E1) if actor is not a Crew Lead.
    /// - `PassengerAlreadyExists` (PS-E2) if an active passenger with
    ///   that id already exists.
    pub fn create(
        &mut self,
        actor: &Actor,
        id: PassengerId,
        name: String,
        tier: Tier,
    ) -> Result<Passenger, DomainError> {
        require_crew_lead(actor)?;
        if self.active.iter().any(|p| p.id == id) {
            return Err(DomainError::PassengerAlreadyExists);
        }
        let p = Passenger {
            id,
            name,
            tier,
            deleted_at: None,
        };
        self.active.push(p.clone());
        Ok(p)
    }

    /// PS-R3/R4 — Crew-Lead-only tier change. Idempotent.
    ///
    /// # Errors
    /// - `UnauthorizedActor` (PS-E1).
    /// - `PassengerNotFound` (PS-E3) if id is unknown or soft-deleted.
    pub fn change_tier(
        &mut self,
        actor: &Actor,
        id: &PassengerId,
        new_tier: Tier,
    ) -> Result<(), DomainError> {
        require_crew_lead(actor)?;
        let slot = self
            .active
            .iter_mut()
            .find(|p| p.id == *id)
            .ok_or(DomainError::PassengerNotFound)?;
        slot.tier = new_tier;
        Ok(())
    }

    /// PS-R5 — soft delete (Crew-Lead-only). Stamps `deleted_at` from
    /// the clock and moves the record into the soft-deleted set.
    ///
    /// # Errors
    /// - `UnauthorizedActor` (PS-E1).
    /// - `PassengerNotFound` (PS-E3).
    pub fn soft_delete(&mut self, actor: &Actor, id: &PassengerId) -> Result<(), DomainError> {
        require_crew_lead(actor)?;
        let pos = self
            .active
            .iter()
            .position(|p| p.id == *id)
            .ok_or(DomainError::PassengerNotFound)?;
        let mut p = self.active.remove(pos);
        p.deleted_at = Some(self.clock.now());
        self.deleted.push(p);
        Ok(())
    }

    /// PS-R8 — active passengers in insertion order.
    #[must_use]
    pub fn list(&self) -> &[Passenger] {
        &self.active
    }

    /// PS-R9 — return the most recent record for `id` (active first,
    /// otherwise the latest soft-deleted record).
    ///
    /// # Errors
    /// `PassengerNotFound` (PS-E3) if no record exists.
    pub fn get(&self, id: &PassengerId) -> Result<&Passenger, DomainError> {
        if let Some(p) = self.active.iter().find(|p| p.id == *id) {
            return Ok(p);
        }
        // Fall back to the most recently soft-deleted record matching id.
        self.deleted
            .iter()
            .rev()
            .find(|p| p.id == *id)
            .ok_or(DomainError::PassengerNotFound)
    }
}
