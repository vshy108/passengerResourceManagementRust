//! Passenger application service. See `specs/03-passenger.md` (PS).

use uuid::Uuid;

use crate::application::guards::require_crew_lead;
use crate::application::ports::{AdminEventSink, Clock};
use crate::domain::actor::Actor;
use crate::domain::admin_event::{AdminAction, AdminEvent, TargetKind};
use crate::domain::crew_lead::CrewLeadId;
use crate::domain::errors::DomainError;
use crate::domain::passenger::{Passenger, PassengerId};
use crate::domain::tier::Tier;

pub struct PassengerService<C: Clock> {
    /// Active passengers, in insertion order.
    active: Vec<Passenger>,
    /// Soft-deleted records (kept for audit lookups via `get`).
    // Two Vecs (active + deleted) instead of one Vec with a flag —
    // simpler iteration on the hot `list()` path, no need to filter
    // every read. PS-R5 just moves between the two.
    deleted: Vec<Passenger>,
    clock: C,
    // Audit sink is generic via trait object — same trick as the
    // crew-lead service. `Option<...>` because audit is opt-in.
    audit: Option<Box<dyn AdminEventSink>>,
}

impl<C: Clock> PassengerService<C> {
    #[must_use]
    pub fn new(clock: C) -> Self {
        Self {
            // `Vec::new()` is the canonical empty Vec — no allocation
            // happens until you push. Cheap.
            active: Vec::new(),
            deleted: Vec::new(),
            clock,
            audit: None,
        }
    }

    /// AU-R6 — opt in to admin audit emission.
    // *Builder pattern*: takes `self` BY VALUE (consumes the service),
    // mutates it, returns it. Lets callers chain:
    //   `PassengerService::new(clock).with_audit(sink)`.
    // `mut self` makes the consumed value mutable inside the function.
    #[must_use]
    pub fn with_audit(mut self, sink: Box<dyn AdminEventSink>) -> Self {
        self.audit = Some(sink);
        self
    }

    /// Restore pre-existing passenger records loaded from persistent storage.
    /// Does NOT emit audit events — the records already exist in the audit log.
    #[must_use]
    pub fn with_preloaded(mut self, active: Vec<Passenger>, deleted: Vec<Passenger>) -> Self {
        self.active = active;
        self.deleted = deleted;
        self
    }

    /// Soft-deleted passengers (for persistence snapshots).
    #[must_use]
    pub fn deleted(&self) -> &[Passenger] {
        &self.deleted
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
        // Taking owned values (`PassengerId`, `String`) instead of
        // borrows lets us move them straight into the new Passenger
        // without cloning. Caller is expected to hand them over.
        id: PassengerId,
        name: String,
        tier: Tier,
    ) -> Result<Passenger, DomainError> {
        // Guard returns `&CrewLeadId`; we `clone()` because we'll need
        // an owned copy after `?` propagates the borrow lifetime out.
        let actor_id = require_crew_lead(actor)?.clone();
        // `any` short-circuits on the first match.
        if self.active.iter().any(|p| p.id == id) {
            return Err(DomainError::PassengerAlreadyExists);
        }
        let p = Passenger {
            id,
            name,
            tier,
            deleted_at: None,
            version: 0,
        };
        // Push a clone so we can also return `p` to the caller.
        self.active.push(p.clone());
        self.emit(
            &actor_id,
            AdminAction::PassengerCreated,
            p.id.0.clone(),
            // `format!("tier={tier:?}")` is the new (Rust 2021+) inline
            // format-args syntax — equivalent to `format!("tier={:?}", tier)`.
            Some(format!("tier={tier:?}")),
        );
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
        let actor_id = require_crew_lead(actor)?.clone();
        // `iter_mut()` yields `&mut Passenger` so we can mutate in place.
        let slot = self
            .active
            .iter_mut()
            .find(|p| p.id == *id)
            .ok_or(DomainError::PassengerNotFound)?;
        slot.tier = new_tier;
        // Increment version so concurrent If-Match checks on the old version fail.
        slot.version += 1;
        self.emit(
            &actor_id,
            AdminAction::PassengerTierChanged,
            id.0.clone(),
            Some(format!("tier={new_tier:?}")),
        );
        Ok(())
    }

    /// PS-R5 — soft delete (Crew-Lead-only). Stamps `deleted_at` from
    /// the clock and moves the record into the soft-deleted set.
    ///
    /// # Errors
    /// - `UnauthorizedActor` (PS-E1).
    /// - `PassengerNotFound` (PS-E3).
    pub fn soft_delete(&mut self, actor: &Actor, id: &PassengerId) -> Result<(), DomainError> {
        let actor_id = require_crew_lead(actor)?.clone();
        // We compute the index first (immutable borrow), then call
        // `remove` (mutable borrow). Splitting like this keeps the borrow
        // checker happy — only one borrow active at a time.
        let pos = self
            .active
            .iter()
            .position(|p| p.id == *id)
            .ok_or(DomainError::PassengerNotFound)?;
        // `Vec::remove` shifts later elements down — O(n). Acceptable
        // for a small in-memory roster.
        let mut p = self.active.remove(pos);
        p.deleted_at = Some(self.clock.now());
        // Increment version so any in-flight If-Match request with the old version fails.
        p.version += 1;
        self.deleted.push(p);
        self.emit(&actor_id, AdminAction::PassengerDeleted, id.0.clone(), None);
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
        // `if let` is `match` with one named arm and an implicit catch-all
        // that does nothing. Common shorthand for "do X iff Some/Ok".
        if let Some(p) = self.active.iter().find(|p| p.id == *id) {
            return Ok(p);
        }
        // Fall back to the most recently soft-deleted record matching id.
        // `.rev()` walks the iterator backwards — last item first.
        // (Possible because slice iterators are DoubleEndedIterators.)
        self.deleted
            .iter()
            .rev()
            .find(|p| p.id == *id)
            .ok_or(DomainError::PassengerNotFound)
    }

    /// Emit an audit event when a sink is configured. Caller passes a
    /// `CrewLeadId` obtained from `require_crew_lead`, so no further
    /// authorisation pattern-matching is needed here.
    // No `pub` -> private helper, only callable from inside this impl.
    fn emit(
        &mut self,
        actor_id: &CrewLeadId,
        action: AdminAction,
        target_id: String,
        details: Option<String>,
    ) {
        // Early-return idiom using `let-else`: keeps the happy path
        // unindented, exits silently when audit is off.
        let Some(sink) = self.audit.as_mut() else {
            return;
        };
        let event = AdminEvent {
            // FIX: was u64 counter (reset on restart); UUID v4 is stable
            // once persisted.
            id: Uuid::new_v4().to_string(),
            actor_id: actor_id.clone(),
            action,
            target_kind: TargetKind::Passenger,
            target_id,
            timestamp: self.clock.now(),
            details,
        };
        sink.append(event);
    }
}
