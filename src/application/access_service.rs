//! Access service. See `specs/05-access.md` (AC).

use crate::application::passenger_service::PassengerService;
use crate::application::ports::{Clock, UsageEventSink};
use crate::application::resource_service::ResourceService;
use crate::domain::actor::Actor;
use crate::domain::errors::DomainError;
use crate::domain::resource::ResourceId;
use crate::domain::usage_event::{Outcome, UsageEvent};

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
        // AC-R1.
        let Actor::Passenger(passenger_id) = actor else {
            return Err(DomainError::UnauthorizedActor);
        };
        // AC-R2 — only active passengers can attempt access.
        let passenger = passengers
            .list()
            .iter()
            .find(|p| p.id == *passenger_id)
            .ok_or(DomainError::PassengerNotFound)?;
        // AC-R3 — only active resources are valid targets.
        let resource = resources
            .list()
            .iter()
            .find(|r| r.id == *resource_id)
            .ok_or(DomainError::ResourceNotFound)?;

        // AC-R4 / AC-R5 / AC-R7 — emit event before returning.
        let allowed = passenger.tier.can_access(resource.min_tier);
        let event = UsageEvent {
            id: self.next_event_id,
            passenger_id: passenger.id.clone(),
            resource_id: resource.id.clone(),
            tier_at_attempt: passenger.tier,
            min_tier_at_attempt: resource.min_tier,
            timestamp: self.clock.now(),
            outcome: if allowed {
                Outcome::Allowed
            } else {
                Outcome::Denied
            },
        };
        self.next_event_id += 1;
        self.sink.append(event.clone());

        if allowed {
            Ok(event)
        } else {
            Err(DomainError::AccessDenied)
        }
    }
}
