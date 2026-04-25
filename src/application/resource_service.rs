//! Resource application service. See `specs/04-resource.md` (RS).

use crate::application::guards::require_crew_lead;
use crate::application::ports::Clock;
use crate::domain::actor::Actor;
use crate::domain::errors::DomainError;
use crate::domain::resource::{Resource, ResourceId};
use crate::domain::tier::Tier;

pub struct ResourceService<C: Clock> {
    active: Vec<Resource>,
    deleted: Vec<Resource>,
    clock: C,
}

impl<C: Clock> ResourceService<C> {
    #[must_use]
    pub fn new(clock: C) -> Self {
        Self {
            active: Vec::new(),
            deleted: Vec::new(),
            clock,
        }
    }

    /// RS-R1.
    ///
    /// # Errors
    /// `UnauthorizedActor` (RS-E1) or `ResourceAlreadyExists` (RS-E2).
    pub fn create(
        &mut self,
        actor: &Actor,
        id: ResourceId,
        name: String,
        category: String,
        min_tier: Tier,
    ) -> Result<Resource, DomainError> {
        let _ = (actor, &id, &name, &category, min_tier);
        todo!("RS-R1: implement create")
    }

    /// RS-R3.
    ///
    /// # Errors
    /// `UnauthorizedActor` (RS-E1) or `ResourceNotFound` (RS-E3).
    pub fn change_min_tier(
        &mut self,
        actor: &Actor,
        id: &ResourceId,
        new_tier: Tier,
    ) -> Result<(), DomainError> {
        let _ = (actor, id, new_tier);
        todo!("RS-R3: implement change_min_tier")
    }

    /// RS-R4.
    ///
    /// # Errors
    /// `UnauthorizedActor` (RS-E1) or `ResourceNotFound` (RS-E3).
    pub fn soft_delete(&mut self, actor: &Actor, id: &ResourceId) -> Result<(), DomainError> {
        let _ = (actor, id);
        todo!("RS-R4: implement soft_delete")
    }

    /// RS-R6.
    #[must_use]
    pub fn list(&self) -> &[Resource] {
        &self.active
    }

    /// RS-R7.
    #[must_use]
    pub fn list_accessible_for(&self, tier: Tier) -> Vec<Resource> {
        let _ = tier;
        todo!("RS-R7: implement list_accessible_for")
    }

    /// RS-R5 / RS-R9-equivalent: latest record (active or soft-deleted).
    ///
    /// # Errors
    /// `ResourceNotFound` (RS-E3).
    pub fn get(&self, id: &ResourceId) -> Result<&Resource, DomainError> {
        let _ = id;
        todo!("RS-R5: implement get")
    }
}
