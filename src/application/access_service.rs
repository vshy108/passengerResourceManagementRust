//! Access service. See `specs/05-access.md` (AC).

use crate::application::passenger_service::PassengerService;
use crate::application::ports::{Clock, UsageEventSink};
use crate::application::resource_service::ResourceService;
use crate::domain::actor::Actor;
use crate::domain::errors::DomainError;
use crate::domain::resource::ResourceId;
use crate::domain::usage_event::{Outcome, UsageEvent};

// `<C: Clock, S: UsageEventSink>` are *generic type parameters with
// trait bounds* — `C` can be ANY type that implements `Clock`. The
// compiler generates a specialised version of this struct per concrete
// pair (monomorphisation) → zero runtime cost vs. trait objects.
//
// We could instead use `Box<dyn Clock>` (dynamic dispatch) but generics
// were chosen for performance and to keep the API ergonomic in tests.
pub struct AccessService<C: Clock, S: UsageEventSink> {
    // Fields are PRIVATE by default. No `pub` keyword = only this module
    // can read or write them — the rest of the world goes through
    // methods. This is the only encapsulation Rust offers.
    clock: C,
    sink: S,
    // Monotonic counter for event ids, never resets, never reused.
    next_event_id: u64,
}

// `impl<C, S>` block adds methods. The bounds must be repeated here.
impl<C: Clock, S: UsageEventSink> AccessService<C, S> {
    // `#[must_use]` -> warn if the caller drops the constructed value.
    // Constructors are commonly annotated to catch silly mistakes.
    #[must_use]
    pub fn new(clock: C, sink: S) -> Self {
        // Struct literal syntax. Field shorthand `clock` is shorthand
        // for `clock: clock` (param name == field name).
        Self {
            clock,
            sink,
            next_event_id: 1,
        }
    }

    #[must_use]
    pub fn sink(&self) -> &S {
        // Returning `&S` borrows the inner sink. The lifetime is tied
        // (elided) to `&self`, so callers can't outlive the service.
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
    // Extra generics `<PC, RC>` let the caller pass services that use
    // *different* clock types than ours. In practice tests typically use
    // the same FakeClock everywhere, but the API doesn't force that.
    //
    // `&mut self` because we bump `next_event_id` and append to the sink.
    pub fn use_resource<PC: Clock, RC: Clock>(
        &mut self,
        actor: &Actor,
        passengers: &PassengerService<PC>,
        resources: &ResourceService<RC>,
        resource_id: &ResourceId,
    ) -> Result<UsageEvent, DomainError> {
        // AC-R1.
        // `let-else` (Rust 1.65+): if the pattern matches, `passenger_id`
        // is bound for the rest of the function; if it doesn't match,
        // the `else` block MUST diverge (return / panic / continue / etc.).
        // Cleaner than `match` when only one arm is "happy path".
        let Actor::Passenger(passenger_id) = actor else {
            return Err(DomainError::UnauthorizedActor);
        };
        // AC-R2 — only active passengers can attempt access.
        let passenger = passengers
            .list()
            // `.iter()` -> iterator yielding `&Passenger`.
            .iter()
            // `find` returns `Option<&Passenger>`. `|p| ...` is a closure;
            // `p` is `&&Passenger` here (iterator yields refs, find adds
            // another). We compare via `*passenger_id` to deref the &Id.
            .find(|p| p.id == *passenger_id)
            // `.ok_or(err)` converts `Option<T>` -> `Result<T, E>`:
            //   Some(t) -> Ok(t), None -> Err(err).
            // The trailing `?` propagates an Err early-return — saves us
            // writing `match ... { Err(e) => return Err(e), Ok(v) => v }`.
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
            // `.clone()` is required for non-Copy fields — we keep our
            // borrowed reference but the event needs OWNED ids so it can
            // be moved into the sink and returned to the caller.
            passenger_id: passenger.id.clone(),
            resource_id: resource.id.clone(),
            // `Tier` is Copy → no `.clone()` needed.
            tier_at_attempt: passenger.tier,
            min_tier_at_attempt: resource.min_tier,
            timestamp: self.clock.now(),
            // `if`/`else` is an *expression* in Rust: it yields a value.
            outcome: if allowed {
                Outcome::Allowed
            } else {
                Outcome::Denied
            },
        };
        self.next_event_id += 1;
        // `.clone()` because `append` takes ownership of the event but we
        // also want to return it to the caller below. Cheap-ish since
        // UsageEvent only has String ids inside it.
        self.sink.append(event.clone());

        if allowed {
            Ok(event)
        } else {
            // Note we still emitted the event above (audit trail records
            // BOTH allowed and denied attempts — AC-R7).
            Err(DomainError::AccessDenied)
        }
    }
}
