//! Integration tests for `specs/07-reporting.md` (RP-S1..S10).

// Watch the `attempt()` helper: it ignores the Result via `let _ = ...`.
// The reporting service cares about EVENTS (success and failure both
// emit one), not the boolean outcome — so swallowing the Result is
// correct here. In production code, `let _ = result;` would be
// forbidden by AGENTS.md §5 — tests get more leeway.

use passenger_resource_management::application::access_service::AccessService;
use passenger_resource_management::application::passenger_service::PassengerService;
use passenger_resource_management::application::reporting_service::{ReportingService, TierCounts};
use passenger_resource_management::application::resource_service::ResourceService;
use passenger_resource_management::domain::actor::Actor;
use passenger_resource_management::domain::crew_lead::CrewLeadId;
use passenger_resource_management::domain::passenger::PassengerId;
use passenger_resource_management::domain::resource::ResourceId;
use passenger_resource_management::domain::tier::Tier;
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

fn seed_passenger(w: &mut World, id: &str, t: Tier) {
    w.passengers
        .create(&admin(), PassengerId::from(id), id.into(), t)
        .unwrap();
}

fn seed_resource(w: &mut World, id: &str, t: Tier) {
    w.resources
        .create(&admin(), ResourceId::from(id), id.into(), "g".into(), t)
        .unwrap();
}

fn attempt(w: &mut World, passenger: &str, resource: &str) {
    let _ = w.access.use_resource(
        &pa(passenger),
        &w.passengers,
        &w.resources,
        &ResourceId::from(resource),
    );
}

// -- RP-S1..S3 (personal history) -------------------------------------

#[test]
fn rp_s1_personal_history_returns_all_events_in_insertion_order() {
    let mut w = world();
    seed_passenger(&mut w, "p1", Tier::Silver);
    seed_resource(&mut w, "r1", Tier::Silver);
    seed_resource(&mut w, "r2", Tier::Gold); // denies p1
    attempt(&mut w, "p1", "r1");
    attempt(&mut w, "p1", "r2");
    attempt(&mut w, "p1", "r1");
    let rep = ReportingService::new(w.access.sink());
    let hist = rep.personal_history(&PassengerId::from("p1"));
    assert_eq!(hist.len(), 3);
    assert_eq!(hist[0].resource_id, ResourceId::from("r1"));
    assert_eq!(hist[1].resource_id, ResourceId::from("r2"));
    assert_eq!(hist[2].resource_id, ResourceId::from("r1"));
}

#[test]
fn rp_s2_personal_history_unknown_passenger_returns_empty() {
    let w = world();
    let rep = ReportingService::new(w.access.sink());
    assert!(rep.personal_history(&PassengerId::from("ghost")).is_empty());
}

#[test]
fn rp_s3_personal_history_excludes_other_passengers() {
    let mut w = world();
    seed_passenger(&mut w, "p1", Tier::Silver);
    seed_passenger(&mut w, "p2", Tier::Silver);
    seed_resource(&mut w, "r1", Tier::Silver);
    attempt(&mut w, "p1", "r1");
    attempt(&mut w, "p2", "r1");
    attempt(&mut w, "p1", "r1");
    let rep = ReportingService::new(w.access.sink());
    let hist = rep.personal_history(&PassengerId::from("p1"));
    assert_eq!(hist.len(), 2);
    assert!(
        hist.iter()
            .all(|e| e.passenger_id == PassengerId::from("p1"))
    );
}

// -- RP-S4..S6 (aggregate by tier) ------------------------------------

#[test]
fn rp_s4_aggregate_by_tier_silver_2_allowed_1_denied() {
    let mut w = world();
    seed_passenger(&mut w, "p1", Tier::Silver);
    seed_resource(&mut w, "r1", Tier::Silver);
    seed_resource(&mut w, "r2", Tier::Gold);
    attempt(&mut w, "p1", "r1"); // allowed
    attempt(&mut w, "p1", "r1"); // allowed
    attempt(&mut w, "p1", "r2"); // denied
    let rep = ReportingService::new(w.access.sink());
    let agg = rep.aggregate_by_tier();
    assert_eq!(
        agg[&Tier::Silver],
        TierCounts {
            allowed: 2,
            denied: 1,
        }
    );
}

#[test]
fn rp_s5_aggregate_by_tier_includes_zero_buckets() {
    let w = world();
    let rep = ReportingService::new(w.access.sink());
    let agg = rep.aggregate_by_tier();
    for t in [Tier::Silver, Tier::Gold, Tier::Platinum] {
        assert_eq!(
            agg[&t],
            TierCounts {
                allowed: 0,
                denied: 0,
            }
        );
    }
}

#[test]
fn rp_s6_tier_change_does_not_reclassify_past_events() {
    let mut w = world();
    seed_passenger(&mut w, "p1", Tier::Silver);
    seed_resource(&mut w, "r1", Tier::Silver);
    attempt(&mut w, "p1", "r1"); // allowed @ Silver
    w.passengers
        .change_tier(&admin(), &PassengerId::from("p1"), Tier::Platinum)
        .unwrap();
    attempt(&mut w, "p1", "r1"); // allowed @ Platinum
    let rep = ReportingService::new(w.access.sink());
    let agg = rep.aggregate_by_tier();
    assert_eq!(agg[&Tier::Silver].allowed, 1);
    assert_eq!(agg[&Tier::Platinum].allowed, 1);
}

// -- RP-S7..S10 (top resources) ---------------------------------------

#[test]
fn rp_s7_top_resources_returns_top_n_by_allowed_count() {
    let mut w = world();
    seed_passenger(&mut w, "p1", Tier::Platinum);
    seed_resource(&mut w, "r1", Tier::Silver);
    seed_resource(&mut w, "r2", Tier::Silver);
    seed_resource(&mut w, "r3", Tier::Silver);
    for _ in 0..3 {
        attempt(&mut w, "p1", "r1");
    }
    attempt(&mut w, "p1", "r2");
    for _ in 0..2 {
        attempt(&mut w, "p1", "r3");
    }
    let rep = ReportingService::new(w.access.sink());
    let top = rep.top_resources(2);
    assert_eq!(
        top,
        vec![(ResourceId::from("r1"), 3), (ResourceId::from("r3"), 2),]
    );
}

#[test]
fn rp_s8_top_resources_ignores_denied() {
    let mut w = world();
    seed_passenger(&mut w, "p1", Tier::Silver);
    seed_resource(&mut w, "r1", Tier::Gold); // denies p1
    seed_resource(&mut w, "r2", Tier::Silver);
    for _ in 0..5 {
        attempt(&mut w, "p1", "r1");
    }
    attempt(&mut w, "p1", "r2");
    let rep = ReportingService::new(w.access.sink());
    assert_eq!(rep.top_resources(1), vec![(ResourceId::from("r2"), 1)]);
}

#[test]
fn rp_s9_top_resources_ties_broken_by_resource_id_ascending() {
    let mut w = world();
    seed_passenger(&mut w, "p1", Tier::Platinum);
    seed_resource(&mut w, "rb", Tier::Silver);
    seed_resource(&mut w, "ra", Tier::Silver);
    attempt(&mut w, "p1", "rb");
    attempt(&mut w, "p1", "ra");
    let rep = ReportingService::new(w.access.sink());
    let top = rep.top_resources(2);
    assert_eq!(
        top,
        vec![(ResourceId::from("ra"), 1), (ResourceId::from("rb"), 1),]
    );
}

#[test]
fn rp_s10_top_resources_zero_returns_empty() {
    let mut w = world();
    seed_passenger(&mut w, "p1", Tier::Silver);
    seed_resource(&mut w, "r1", Tier::Silver);
    attempt(&mut w, "p1", "r1");
    let rep = ReportingService::new(w.access.sink());
    assert!(rep.top_resources(0).is_empty());
}
