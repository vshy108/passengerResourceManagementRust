//! Integration tests for `specs/03-passenger.md` (PS-S1..S10).

use passenger_resource_management::application::passenger_service::PassengerService;
use passenger_resource_management::domain::actor::Actor;
use passenger_resource_management::domain::crew_lead::CrewLeadId;
use passenger_resource_management::domain::errors::DomainError;
use passenger_resource_management::domain::passenger::PassengerId;
use passenger_resource_management::domain::tier::Tier;
use passenger_resource_management::infrastructure::fake_clock::FakeClock;

fn admin() -> Actor {
    Actor::CrewLead(CrewLeadId::from("a"))
}

fn passenger_actor() -> Actor {
    Actor::Passenger(PassengerId::from("p1"))
}

fn svc() -> PassengerService<FakeClock> {
    PassengerService::new(FakeClock::default())
}

// -- Create (PS-S1..S3) -----------------------------------------------

#[test]
fn ps_s1_crew_lead_can_create_passenger() {
    let mut svc = svc();
    svc.create(
        &admin(),
        PassengerId::from("p1"),
        "Alice".into(),
        Tier::Silver,
    )
    .expect("PS-S1");
    assert_eq!(svc.list().len(), 1);
    assert_eq!(svc.list()[0].tier, Tier::Silver);
}

#[test]
fn ps_s2_passenger_actor_cannot_create() {
    let mut svc = svc();
    let res = svc.create(
        &passenger_actor(),
        PassengerId::from("p1"),
        "Alice".into(),
        Tier::Silver,
    );
    assert_eq!(res.err(), Some(DomainError::UnauthorizedActor));
    assert!(svc.list().is_empty());
}

#[test]
fn ps_s3_create_with_existing_active_id_fails() {
    let mut svc = svc();
    svc.create(
        &admin(),
        PassengerId::from("p1"),
        "Alice".into(),
        Tier::Silver,
    )
    .unwrap();
    let res = svc.create(
        &admin(),
        PassengerId::from("p1"),
        "Alice II".into(),
        Tier::Gold,
    );
    assert_eq!(res.err(), Some(DomainError::PassengerAlreadyExists));
}

// -- Change tier (PS-S4..S7) ------------------------------------------

#[test]
fn ps_s4_crew_lead_can_change_tier() {
    let mut svc = svc();
    svc.create(
        &admin(),
        PassengerId::from("p1"),
        "Alice".into(),
        Tier::Silver,
    )
    .unwrap();
    svc.change_tier(&admin(), &PassengerId::from("p1"), Tier::Platinum)
        .expect("PS-S4");
    assert_eq!(
        svc.get(&PassengerId::from("p1")).unwrap().tier,
        Tier::Platinum
    );
}

#[test]
fn ps_s5_passenger_actor_cannot_change_tier() {
    let mut svc = svc();
    svc.create(
        &admin(),
        PassengerId::from("p1"),
        "Alice".into(),
        Tier::Silver,
    )
    .unwrap();
    let res = svc.change_tier(&passenger_actor(), &PassengerId::from("p1"), Tier::Gold);
    assert_eq!(res, Err(DomainError::UnauthorizedActor));
    assert_eq!(svc.get(&PassengerId::from("p1")).unwrap().tier, Tier::Silver);
}

#[test]
fn ps_s6_change_tier_unknown_id_returns_not_found() {
    let mut svc = svc();
    let res = svc.change_tier(&admin(), &PassengerId::from("zz"), Tier::Gold);
    assert_eq!(res, Err(DomainError::PassengerNotFound));
}

#[test]
fn ps_s7_change_tier_to_same_tier_is_idempotent() {
    let mut svc = svc();
    svc.create(
        &admin(),
        PassengerId::from("p1"),
        "Alice".into(),
        Tier::Gold,
    )
    .unwrap();
    svc.change_tier(&admin(), &PassengerId::from("p1"), Tier::Gold)
        .expect("PS-S7");
    assert_eq!(svc.get(&PassengerId::from("p1")).unwrap().tier, Tier::Gold);
}

// -- Soft delete (PS-S8..S9) ------------------------------------------

#[test]
fn ps_s8_soft_delete_excludes_from_list_but_get_still_resolves() {
    let mut svc = svc();
    svc.create(
        &admin(),
        PassengerId::from("p1"),
        "Alice".into(),
        Tier::Silver,
    )
    .unwrap();
    svc.soft_delete(&admin(), &PassengerId::from("p1"))
        .expect("PS-S8");
    assert!(svc.list().is_empty());
    let got = svc.get(&PassengerId::from("p1")).unwrap();
    assert!(got.deleted_at.is_some());
}

#[test]
fn ps_s9_can_recreate_id_after_soft_delete() {
    let mut svc = svc();
    svc.create(
        &admin(),
        PassengerId::from("p1"),
        "Alice".into(),
        Tier::Silver,
    )
    .unwrap();
    svc.soft_delete(&admin(), &PassengerId::from("p1")).unwrap();
    svc.create(
        &admin(),
        PassengerId::from("p1"),
        "Alice II".into(),
        Tier::Gold,
    )
    .expect("PS-S9");
    assert_eq!(svc.list().len(), 1);
    assert_eq!(svc.get(&PassengerId::from("p1")).unwrap().tier, Tier::Gold);
}

// -- Listing (PS-S10) -------------------------------------------------

#[test]
fn ps_s10_list_preserves_insertion_order() {
    let mut svc = svc();
    for (id, name) in [("p1", "Alice"), ("p2", "Bob"), ("p3", "Cara")] {
        svc.create(&admin(), PassengerId::from(id), name.into(), Tier::Silver)
            .unwrap();
    }
    let ids: Vec<&str> = svc.list().iter().map(|p| p.id.0.as_str()).collect();
    assert_eq!(ids, vec!["p1", "p2", "p3"]);
}
