//! Integration tests for `specs/05-access.md` (AC-S1..S10).

use passenger_resource_management::application::access_service::AccessService;
use passenger_resource_management::application::passenger_service::PassengerService;
use passenger_resource_management::application::ports::UsageEventSource;
use passenger_resource_management::application::resource_service::ResourceService;
use passenger_resource_management::domain::actor::Actor;
use passenger_resource_management::domain::crew_lead::CrewLeadId;
use passenger_resource_management::domain::errors::DomainError;
use passenger_resource_management::domain::passenger::PassengerId;
use passenger_resource_management::domain::resource::ResourceId;
use passenger_resource_management::domain::tier::Tier;
use passenger_resource_management::domain::usage_event::Outcome;
use passenger_resource_management::infrastructure::fake_clock::FakeClock;
use passenger_resource_management::infrastructure::in_memory_usage_event_sink::InMemoryUsageEventSink;

fn admin() -> Actor {
    Actor::CrewLead(CrewLeadId::from("a"))
}
fn pa(id: &str) -> Actor {
    Actor::Passenger(PassengerId::from(id))
}

struct World {
    passengers: PassengerService<FakeClock>,
    resources: ResourceService<FakeClock>,
    access: AccessService<FakeClock, InMemoryUsageEventSink>,
}

fn world() -> World {
    World {
        passengers: PassengerService::new(FakeClock::default()),
        resources: ResourceService::new(FakeClock::default()),
        access: AccessService::new(FakeClock::default(), InMemoryUsageEventSink::new()),
    }
}

fn world_with_clock_starting_at(start: i64) -> World {
    World {
        passengers: PassengerService::new(FakeClock::default()),
        resources: ResourceService::new(FakeClock::default()),
        access: AccessService::new(FakeClock::starting_at(start), InMemoryUsageEventSink::new()),
    }
}

fn seed_passenger(w: &mut World, id: &str, t: Tier) {
    w.passengers
        .create(&admin(), PassengerId::from(id), id.into(), t)
        .unwrap();
}

fn seed_resource(w: &mut World, id: &str, t: Tier) {
    w.resources
        .create(
            &admin(),
            ResourceId::from(id),
            id.into(),
            "general".into(),
            t,
        )
        .unwrap();
}

// -- AC-S1 -------------------------------------------------------------

#[test]
fn ac_s1_crew_lead_actor_unauthorized_no_event() {
    let mut w = world();
    seed_passenger(&mut w, "p1", Tier::Silver);
    seed_resource(&mut w, "r1", Tier::Silver);
    let res = w.access.use_resource(
        &admin(),
        &w.passengers,
        &w.resources,
        &ResourceId::from("r1"),
    );
    assert_eq!(res.err(), Some(DomainError::UnauthorizedActor));
    assert!(w.access.sink().list().is_empty());
}

// -- AC-S2..S5 ---------------------------------------------------------

#[test]
fn ac_s2_unknown_passenger_no_event() {
    let mut w = world();
    seed_resource(&mut w, "r1", Tier::Silver);
    let res = w.access.use_resource(
        &pa("ghost"),
        &w.passengers,
        &w.resources,
        &ResourceId::from("r1"),
    );
    assert_eq!(res.err(), Some(DomainError::PassengerNotFound));
    assert!(w.access.sink().list().is_empty());
}

#[test]
fn ac_s3_soft_deleted_passenger_no_event() {
    let mut w = world();
    seed_passenger(&mut w, "p1", Tier::Silver);
    seed_resource(&mut w, "r1", Tier::Silver);
    w.passengers
        .soft_delete(&admin(), &PassengerId::from("p1"))
        .unwrap();
    let res = w.access.use_resource(
        &pa("p1"),
        &w.passengers,
        &w.resources,
        &ResourceId::from("r1"),
    );
    assert_eq!(res.err(), Some(DomainError::PassengerNotFound));
    assert!(w.access.sink().list().is_empty());
}

#[test]
fn ac_s4_unknown_resource_no_event() {
    let mut w = world();
    seed_passenger(&mut w, "p1", Tier::Silver);
    let res = w.access.use_resource(
        &pa("p1"),
        &w.passengers,
        &w.resources,
        &ResourceId::from("ghost"),
    );
    assert_eq!(res.err(), Some(DomainError::ResourceNotFound));
    assert!(w.access.sink().list().is_empty());
}

#[test]
fn ac_s5_soft_deleted_resource_no_event() {
    let mut w = world();
    seed_passenger(&mut w, "p1", Tier::Silver);
    seed_resource(&mut w, "r1", Tier::Silver);
    w.resources
        .soft_delete(&admin(), &ResourceId::from("r1"))
        .unwrap();
    let res = w.access.use_resource(
        &pa("p1"),
        &w.passengers,
        &w.resources,
        &ResourceId::from("r1"),
    );
    assert_eq!(res.err(), Some(DomainError::ResourceNotFound));
    assert!(w.access.sink().list().is_empty());
}

// -- AC-S6 / AC-S7 ----------------------------------------------------

#[test]
fn ac_s6_platinum_on_silver_allowed_emits_one_allowed_event() {
    let mut w = world();
    seed_passenger(&mut w, "p1", Tier::Platinum);
    seed_resource(&mut w, "r1", Tier::Silver);
    let ev = w
        .access
        .use_resource(
            &pa("p1"),
            &w.passengers,
            &w.resources,
            &ResourceId::from("r1"),
        )
        .expect("AC-S6");
    assert_eq!(ev.outcome, Outcome::Allowed);
    assert_eq!(w.access.sink().list().len(), 1);
    assert_eq!(w.access.sink().list()[0].outcome, Outcome::Allowed);
}

#[test]
fn ac_s7_silver_on_gold_denied_emits_one_denied_event() {
    let mut w = world();
    seed_passenger(&mut w, "p1", Tier::Silver);
    seed_resource(&mut w, "r1", Tier::Gold);
    let res = w.access.use_resource(
        &pa("p1"),
        &w.passengers,
        &w.resources,
        &ResourceId::from("r1"),
    );
    assert_eq!(res.err(), Some(DomainError::AccessDenied));
    assert_eq!(w.access.sink().list().len(), 1);
    assert_eq!(w.access.sink().list()[0].outcome, Outcome::Denied);
}

// -- AC-S8 / AC-S9 (snapshots) ----------------------------------------

#[test]
fn ac_s8_snapshot_tier_at_attempt_not_rewritten_on_later_upgrade() {
    let mut w = world();
    seed_passenger(&mut w, "p1", Tier::Silver);
    seed_resource(&mut w, "r1", Tier::Silver);
    w.access
        .use_resource(
            &pa("p1"),
            &w.passengers,
            &w.resources,
            &ResourceId::from("r1"),
        )
        .unwrap();
    w.passengers
        .change_tier(&admin(), &PassengerId::from("p1"), Tier::Platinum)
        .unwrap();
    assert_eq!(w.access.sink().list()[0].tier_at_attempt, Tier::Silver);
}

#[test]
fn ac_s9_old_denied_stays_denied_after_upgrade() {
    let mut w = world();
    seed_passenger(&mut w, "p1", Tier::Silver);
    seed_resource(&mut w, "r1", Tier::Gold);
    let _ = w.access.use_resource(
        &pa("p1"),
        &w.passengers,
        &w.resources,
        &ResourceId::from("r1"),
    );
    w.passengers
        .change_tier(&admin(), &PassengerId::from("p1"), Tier::Gold)
        .unwrap();
    let later = w
        .access
        .use_resource(
            &pa("p1"),
            &w.passengers,
            &w.resources,
            &ResourceId::from("r1"),
        )
        .expect("upgrade should allow");
    assert_eq!(later.outcome, Outcome::Allowed);
    assert_eq!(w.access.sink().list().len(), 2);
    assert_eq!(w.access.sink().list()[0].outcome, Outcome::Denied);
    assert_eq!(w.access.sink().list()[1].outcome, Outcome::Allowed);
}

// -- AC-S10 (clock) ---------------------------------------------------

#[test]
fn ac_s10_uses_injected_clock_for_timestamp() {
    let mut w = world_with_clock_starting_at(42);
    seed_passenger(&mut w, "p1", Tier::Silver);
    seed_resource(&mut w, "r1", Tier::Silver);
    let ev = w
        .access
        .use_resource(
            &pa("p1"),
            &w.passengers,
            &w.resources,
            &ResourceId::from("r1"),
        )
        .unwrap();
    assert_eq!(ev.timestamp.0, 42);
}
