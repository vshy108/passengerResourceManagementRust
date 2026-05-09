//! Resource application service. See `specs/04-resource.md` (RS).

use uuid::Uuid;

use crate::application::guards::require_crew_lead;
use crate::application::ports::{AdminEventSink, Clock};
use crate::domain::actor::Actor;
use crate::domain::admin_event::{AdminAction, AdminEvent, TargetKind};
use crate::domain::crew_lead::CrewLeadId;
use crate::domain::errors::DomainError;
use crate::domain::resource::{Resource, ResourceId};
use crate::domain::tier::Tier;

// Mirror of `PassengerService` for resources — same shape on purpose so
// the patterns are easy to learn once. See passenger_service.rs for
// detailed comments on these patterns; this file only highlights the
// differences specific to resources.
pub struct ResourceService<C: Clock> {
    active: Vec<Resource>,
    deleted: Vec<Resource>,
    clock: C,
    audit: Option<Box<dyn AdminEventSink>>,
}

impl<C: Clock> ResourceService<C> {
    #[must_use]
    pub fn new(clock: C) -> Self {
        Self {
            active: Vec::new(),
            deleted: Vec::new(),
            clock,
            audit: None,
        }
    }

    /// AU-R6 — opt in to admin audit emission.
    #[must_use]
    pub fn with_audit(mut self, sink: Box<dyn AdminEventSink>) -> Self {
        self.audit = Some(sink);
        self
    }

    /// Restore pre-existing resource records loaded from persistent storage.
    /// Does NOT emit audit events — the records already exist in the audit log.
    #[must_use]
    pub fn with_preloaded(mut self, active: Vec<Resource>, deleted: Vec<Resource>) -> Self {
        self.active = active;
        self.deleted = deleted;
        self
    }

    /// Soft-deleted resources (for persistence snapshots).
    #[must_use]
    pub fn deleted(&self) -> &[Resource] {
        &self.deleted
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
        let actor_id = require_crew_lead(actor)?.clone();
        if self.active.iter().any(|r| r.id == id) {
            return Err(DomainError::ResourceAlreadyExists);
        }
        let r = Resource {
            id,
            name,
            category,
            min_tier,
            deleted_at: None,
        };
        self.active.push(r.clone());
        self.emit(
            &actor_id,
            AdminAction::ResourceCreated,
            r.id.0.clone(),
            Some(format!("min_tier={min_tier:?}")),
        );
        Ok(r)
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
        let actor_id = require_crew_lead(actor)?.clone();
        let slot = self
            .active
            .iter_mut()
            .find(|r| r.id == *id)
            .ok_or(DomainError::ResourceNotFound)?;
        slot.min_tier = new_tier;
        self.emit(
            &actor_id,
            AdminAction::ResourceMinTierChanged,
            id.0.clone(),
            Some(format!("min_tier={new_tier:?}")),
        );
        Ok(())
    }

    /// RS-R4.
    ///
    /// # Errors
    /// `UnauthorizedActor` (RS-E1) or `ResourceNotFound` (RS-E3).
    pub fn soft_delete(&mut self, actor: &Actor, id: &ResourceId) -> Result<(), DomainError> {
        let actor_id = require_crew_lead(actor)?.clone();
        let pos = self
            .active
            .iter()
            .position(|r| r.id == *id)
            .ok_or(DomainError::ResourceNotFound)?;
        let mut r = self.active.remove(pos);
        r.deleted_at = Some(self.clock.now());
        self.deleted.push(r);
        self.emit(&actor_id, AdminAction::ResourceDeleted, id.0.clone(), None);
        Ok(())
    }

    /// RS-R6.
    #[must_use]
    pub fn list(&self) -> &[Resource] {
        &self.active
    }

    /// RS-R7.
    #[must_use]
    pub fn list_accessible_for(&self, tier: Tier) -> Vec<Resource> {
        self.active
            .iter()
            // `filter` keeps elements where the predicate returns true.
            .filter(|r| tier.can_access(r.min_tier))
            // `.cloned()` turns `Iterator<Item = &Resource>` into
            // `Iterator<Item = Resource>` by cloning each element.
            // Needed because we return owned Vec<Resource>.
            .cloned()
            // `.collect()` consumes the iterator into a collection. The
            // target type is inferred from the function's return type.
            .collect()
    }

    /// RS-R5 / RS-R9-equivalent: latest record (active or soft-deleted).
    ///
    /// # Errors
    /// `ResourceNotFound` (RS-E3).
    pub fn get(&self, id: &ResourceId) -> Result<&Resource, DomainError> {
        if let Some(r) = self.active.iter().find(|r| r.id == *id) {
            return Ok(r);
        }
        self.deleted
            .iter()
            .rev()
            .find(|r| r.id == *id)
            .ok_or(DomainError::ResourceNotFound)
    }

    fn emit(
        &mut self,
        actor_id: &CrewLeadId,
        action: AdminAction,
        target_id: String,
        details: Option<String>,
    ) {
        let Some(sink) = self.audit.as_mut() else {
            return;
        };
        let event = AdminEvent {
            // FIX: was u64 counter (reset on restart); UUID v4 is stable
            // once persisted.
            id: Uuid::new_v4().to_string(),
            actor_id: actor_id.clone(),
            action,
            target_kind: TargetKind::Resource,
            target_id,
            timestamp: self.clock.now(),
            details,
        };
        sink.append(event);
    }
}
