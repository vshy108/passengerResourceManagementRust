//! Access service. See `specs/05-access.md` (AC).

use crate::application::passenger_service::PassengerService;
use crate::application::ports::{Clock, UsageEventSink};
use crate::application::resource_service::ResourceService;
use crate::domain::actor::Actor;
use crate::domain::errors::DomainError;
use crate::domain::resource::ResourceId;
use crate::domain::usage_event::UsageEvent;

#[allow(dead_code)] // fields are populated and used once GREEN lands
pub struct AccessService<C: Clock, S: UsageEventSink> {
    clock: C,
    sink: S,
    next_event_id: u64,
}

impl<C: Clock, S: UsageEventSink> AccessService<C, S> {
    #[must_use]
    pub fn new(clock: C, sink: S) -> Self {
        Self {
            clock,
            sink,
            next_event_id: 1,
        }
    }

    #[must_use]
    pub fn sink(&self) -> &S {
        &self.sink
    }

    /// AC-R1..R7 — runtime permission check + audit emission.
    ///
    /// # Errors
    /// - `UnauthorizedActor` (AC-E1) if actor is not a Passenger.
    /// - `PassengerNotFound` (AC-E3) / `ResourceNotFound` (AC-E4) if
    ///   subject or target is missing or soft-deleted (no event).
    /// - `AccessDenied` (AC-E2) when tier is insufficient (event still
    ///   emitted with `Outcome::Denied`).
    pub fn use_resource<PC: Clock, RC: Clock>(
        &mut self,
        actor: &Actor,
        passengers: &PassengerService<PC>,
        resources: &ResourceService<RC>,
        resource_id: &ResourceId,
    ) -> Result<UsageEvent, DomainError> {
        let _ = (actor, passengers, resources, resource_id);
        todo!("AC-R1..R7: implement use_resource")
    }
}
