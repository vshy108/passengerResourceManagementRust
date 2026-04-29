//! Integration tests for `specs/06-audit.md` (AU-S1..S11).

// Two patterns to watch in this file:
//   1. `let sink = InMemoryAdminEventSink::new();` then `sink.clone()`
//      handed to the service — both handles point at the SAME inner
//      buffer (Arc<Mutex<...>>), so `sink.snapshot()` in the test
//      observes events written by the service.
//   2. `assert!(opt.as_deref().is_some_and(|s| s.contains("...")))` —
//      `.as_deref()` turns `Option<String>` into `Option<&str>`, then
//      `.is_some_and(predicate)` (Rust 1.70+) returns true iff the
//      Option is Some AND the predicate holds. Replaces the older
//      `.map(|s| s.contains("...")).unwrap_or(false)` idiom.

use passenger_resource_management::application::crew_lead_service::CrewLeadService;
use passenger_resource_management::application::passenger_service::PassengerService;
use passenger_resource_management::application::resource_service::ResourceService;
use passenger_resource_management::domain::actor::Actor;
use passenger_resource_management::domain::admin_event::{AdminAction, TargetKind};
use passenger_resource_management::domain::crew_lead::{CrewLead, CrewLeadId};
use passenger_resource_management::domain::passenger::PassengerId;
use passenger_resource_management::domain::resource::ResourceId;
use passenger_resource_management::domain::tier::Tier;
use passenger_resource_management::infrastructure::fake_clock::FakeClock;
use passenger_resource_management::infrastructure::in_memory_admin_event_sink::InMemoryAdminEventSink;

fn admin() -> Actor {
    Actor::CrewLead(CrewLeadId::from("a"))
}
fn pa(id: &str) -> Actor {
    Actor::Passenger(PassengerId::from(id))
}

// -- AU-S1..S3 (Passenger) --------------------------------------------

#[test]
fn au_s1_passenger_create_emits_passenger_created() {
    let sink = InMemoryAdminEventSink::new();
    let mut svc = PassengerService::new(FakeClock::default()).with_audit(Box::new(sink.clone()));
    svc.create(&admin(), PassengerId::from("p1"), "P1".into(), Tier::Silver)
        .unwrap();
    let events = sink.snapshot();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].action, AdminAction::PassengerCreated);
    assert_eq!(events[0].target_kind, TargetKind::Passenger);
    assert_eq!(events[0].target_id, "p1");
    assert_eq!(events[0].actor_id, CrewLeadId::from("a"));
}

#[test]
fn au_s2_passenger_change_tier_emits_with_details() {
    let sink = InMemoryAdminEventSink::new();
    let mut svc = PassengerService::new(FakeClock::default()).with_audit(Box::new(sink.clone()));
    svc.create(&admin(), PassengerId::from("p1"), "P1".into(), Tier::Silver)
        .unwrap();
    svc.change_tier(&admin(), &PassengerId::from("p1"), Tier::Gold)
        .unwrap();
    let events = sink.snapshot();
    assert_eq!(events.len(), 2);
    assert_eq!(events[1].action, AdminAction::PassengerTierChanged);
    assert!(
        events[1]
            .details
            .as_deref()
            .is_some_and(|s| s.contains("Gold"))
    );
}

#[test]
fn au_s3_passenger_soft_delete_emits_passenger_deleted() {
    let sink = InMemoryAdminEventSink::new();
    let mut svc = PassengerService::new(FakeClock::default()).with_audit(Box::new(sink.clone()));
    svc.create(&admin(), PassengerId::from("p1"), "P1".into(), Tier::Silver)
        .unwrap();
    svc.soft_delete(&admin(), &PassengerId::from("p1")).unwrap();
    let events = sink.snapshot();
    assert_eq!(events.last().unwrap().action, AdminAction::PassengerDeleted);
}

// -- AU-S4..S6 (Resource) ---------------------------------------------

#[test]
fn au_s4_resource_create_emits_resource_created() {
    let sink = InMemoryAdminEventSink::new();
    let mut svc = ResourceService::new(FakeClock::default()).with_audit(Box::new(sink.clone()));
    svc.create(
        &admin(),
        ResourceId::from("r1"),
        "R1".into(),
        "g".into(),
        Tier::Silver,
    )
    .unwrap();
    let events = sink.snapshot();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].action, AdminAction::ResourceCreated);
    assert_eq!(events[0].target_kind, TargetKind::Resource);
    assert_eq!(events[0].target_id, "r1");
}

#[test]
fn au_s5_resource_change_min_tier_emits_with_details() {
    let sink = InMemoryAdminEventSink::new();
    let mut svc = ResourceService::new(FakeClock::default()).with_audit(Box::new(sink.clone()));
    svc.create(
        &admin(),
        ResourceId::from("r1"),
        "R1".into(),
        "g".into(),
        Tier::Silver,
    )
    .unwrap();
    svc.change_min_tier(&admin(), &ResourceId::from("r1"), Tier::Platinum)
        .unwrap();
    let events = sink.snapshot();
    assert_eq!(events[1].action, AdminAction::ResourceMinTierChanged);
    assert!(
        events[1]
            .details
            .as_deref()
            .is_some_and(|s| s.contains("Platinum"))
    );
}

#[test]
fn au_s6_resource_soft_delete_emits_resource_deleted() {
    let sink = InMemoryAdminEventSink::new();
    let mut svc = ResourceService::new(FakeClock::default()).with_audit(Box::new(sink.clone()));
    svc.create(
        &admin(),
        ResourceId::from("r1"),
        "R1".into(),
        "g".into(),
        Tier::Silver,
    )
    .unwrap();
    svc.soft_delete(&admin(), &ResourceId::from("r1")).unwrap();
    let events = sink.snapshot();
    assert_eq!(events.last().unwrap().action, AdminAction::ResourceDeleted);
}

// -- AU-S7 / AU-S8 (CrewLead) -----------------------------------------

#[test]
fn au_s7_bootstrap_audited_emits_crewleadbootstrapped() {
    let sink = InMemoryAdminEventSink::new();
    let leads = vec![
        CrewLead {
            id: CrewLeadId::from("a"),
            name: "A".into(),
        },
        CrewLead {
            id: CrewLeadId::from("b"),
            name: "B".into(),
        },
        CrewLead {
            id: CrewLeadId::from("c"),
            name: "C".into(),
        },
    ];
    let _svc = CrewLeadService::bootstrap_audited(
        leads,
        Box::new(FakeClock::default()),
        Box::new(sink.clone()),
    )
    .unwrap();
    let events = sink.snapshot();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].action, AdminAction::CrewLeadBootstrapped);
    assert_eq!(events[0].target_kind, TargetKind::CrewLead);
}

#[test]
fn au_s8_replace_audited_emits_crewleadreplaced() {
    let sink = InMemoryAdminEventSink::new();
    let leads = vec![
        CrewLead {
            id: CrewLeadId::from("a"),
            name: "A".into(),
        },
        CrewLead {
            id: CrewLeadId::from("b"),
            name: "B".into(),
        },
        CrewLead {
            id: CrewLeadId::from("c"),
            name: "C".into(),
        },
    ];
    let mut svc = CrewLeadService::bootstrap_audited(
        leads,
        Box::new(FakeClock::default()),
        Box::new(sink.clone()),
    )
    .unwrap();
    svc.replace_audited(
        &CrewLeadId::from("a"),
        &CrewLeadId::from("a"),
        CrewLead {
            id: CrewLeadId::from("d"),
            name: "D".into(),
        },
    )
    .unwrap();
    let events = sink.snapshot();
    assert_eq!(events.len(), 2);
    assert_eq!(events[1].action, AdminAction::CrewLeadReplaced);
    assert_eq!(events[1].target_id, "d");
}

// -- AU-S9 / AU-S10 (no event on failure) -----------------------------

#[test]
fn au_s9_unauthorized_create_emits_no_event() {
    let sink = InMemoryAdminEventSink::new();
    let mut svc = PassengerService::new(FakeClock::default()).with_audit(Box::new(sink.clone()));
    let _ = svc.create(
        &pa("p1"),
        PassengerId::from("p1"),
        "P1".into(),
        Tier::Silver,
    );
    assert!(sink.is_empty());
}

#[test]
fn au_s10_change_tier_unknown_passenger_emits_no_event() {
    let sink = InMemoryAdminEventSink::new();
    let mut svc = PassengerService::new(FakeClock::default()).with_audit(Box::new(sink.clone()));
    let _ = svc.change_tier(&admin(), &PassengerId::from("ghost"), Tier::Gold);
    assert!(sink.is_empty());
}

// -- AU-S11 (ordering) ------------------------------------------------

#[test]
fn au_s11_create_change_delete_recorded_in_order() {
    let sink = InMemoryAdminEventSink::new();
    let mut svc = PassengerService::new(FakeClock::default()).with_audit(Box::new(sink.clone()));
    svc.create(&admin(), PassengerId::from("p1"), "P1".into(), Tier::Silver)
        .unwrap();
    svc.change_tier(&admin(), &PassengerId::from("p1"), Tier::Gold)
        .unwrap();
    svc.soft_delete(&admin(), &PassengerId::from("p1")).unwrap();
    let actions: Vec<_> = sink.snapshot().into_iter().map(|e| e.action).collect();
    assert_eq!(
        actions,
        vec![
            AdminAction::PassengerCreated,
            AdminAction::PassengerTierChanged,
            AdminAction::PassengerDeleted,
        ]
    );
}
