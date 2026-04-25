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
        let _ = (actor, &id, &name, tier);
        todo!("PS-R1: implement create")
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
        let _ = (actor, id, new_tier);
        todo!("PS-R3: implement change_tier")
    }

    /// PS-R5 — soft delete (Crew-Lead-only). Stamps `deleted_at` from
    /// the clock and moves the record into the soft-deleted set.
    ///
    /// # Errors
    /// - `UnauthorizedActor` (PS-E1).
    /// - `PassengerNotFound` (PS-E3).
    pub fn soft_delete(&mut self, actor: &Actor, id: &PassengerId) -> Result<(), DomainError> {
        let _ = (actor, id);
        todo!("PS-R5: implement soft_delete")
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
        let _ = id;
        todo!("PS-R9: implement get")
    }
}
