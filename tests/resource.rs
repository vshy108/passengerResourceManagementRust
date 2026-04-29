//! Integration tests for `specs/04-resource.md` (RS-S1..S11).

// Mirrors `tests/passenger.rs` line-for-line — same shape, different
// aggregate. The duplication is INTENTIONAL: it keeps each test file
// self-contained and individually grokkable, more important than DRY
// in test code.

use passenger_resource_management::application::resource_service::ResourceService;
use passenger_resource_management::domain::actor::Actor;
use passenger_resource_management::domain::crew_lead::CrewLeadId;
use passenger_resource_management::domain::errors::DomainError;
use passenger_resource_management::domain::passenger::PassengerId;
use passenger_resource_management::domain::resource::ResourceId;
use passenger_resource_management::domain::tier::Tier;
use passenger_resource_management::infrastructure::fake_clock::FakeClock;

fn admin() -> Actor {
    Actor::CrewLead(CrewLeadId::from("a"))
}

fn passenger_actor() -> Actor {
    Actor::Passenger(PassengerId::from("p1"))
}

fn svc() -> ResourceService<FakeClock> {
    ResourceService::new(FakeClock::default())
}

fn create(svc: &mut ResourceService<FakeClock>, id: &str, t: Tier) {
    svc.create(
        &admin(),
        ResourceId::from(id),
        format!("name-{id}"),
        "general".into(),
        t,
    )
    .unwrap();
}

// -- Create (RS-S1..S3) -----------------------------------------------

#[test]
fn rs_s1_crew_lead_can_create_resource() {
    let mut svc = svc();
    svc.create(
        &admin(),
        ResourceId::from("r1"),
        "Food Station".into(),
        "food".into(),
        Tier::Silver,
    )
    .expect("RS-S1");
    assert_eq!(svc.list().len(), 1);
}

#[test]
fn rs_s2_passenger_actor_cannot_create() {
    let mut svc = svc();
    let res = svc.create(
        &passenger_actor(),
        ResourceId::from("r1"),
        "x".into(),
        "y".into(),
        Tier::Silver,
    );
    assert_eq!(res.err(), Some(DomainError::UnauthorizedActor));
}

#[test]
fn rs_s3_create_with_existing_active_id_fails() {
    let mut svc = svc();
    create(&mut svc, "r1", Tier::Silver);
    let res = svc.create(
        &admin(),
        ResourceId::from("r1"),
        "x".into(),
        "y".into(),
        Tier::Gold,
    );
    assert_eq!(res.err(), Some(DomainError::ResourceAlreadyExists));
}

// -- Change min tier (RS-S4..S6) --------------------------------------

#[test]
fn rs_s4_crew_lead_can_change_min_tier() {
    let mut svc = svc();
    create(&mut svc, "r1", Tier::Silver);
    svc.change_min_tier(&admin(), &ResourceId::from("r1"), Tier::Platinum)
        .expect("RS-S4");
    assert_eq!(
        svc.get(&ResourceId::from("r1")).unwrap().min_tier,
        Tier::Platinum
    );
}

#[test]
fn rs_s5_passenger_actor_cannot_change_min_tier() {
    let mut svc = svc();
    create(&mut svc, "r1", Tier::Silver);
    let res = svc.change_min_tier(&passenger_actor(), &ResourceId::from("r1"), Tier::Gold);
    assert_eq!(res, Err(DomainError::UnauthorizedActor));
}

#[test]
fn rs_s6_change_min_tier_unknown_id_returns_not_found() {
    let mut svc = svc();
    let res = svc.change_min_tier(&admin(), &ResourceId::from("zz"), Tier::Gold);
    assert_eq!(res, Err(DomainError::ResourceNotFound));
}

// -- Soft delete (RS-S7..S8) ------------------------------------------

#[test]
fn rs_s7_soft_delete_excludes_from_list_but_get_resolves() {
    let mut svc = svc();
    create(&mut svc, "r1", Tier::Silver);
    svc.soft_delete(&admin(), &ResourceId::from("r1"))
        .expect("RS-S7");
    assert!(svc.list().is_empty());
    assert!(
        svc.get(&ResourceId::from("r1"))
            .unwrap()
            .deleted_at
            .is_some()
    );
}

#[test]
fn rs_s8_change_min_tier_on_soft_deleted_returns_not_found() {
    let mut svc = svc();
    create(&mut svc, "r1", Tier::Silver);
    svc.soft_delete(&admin(), &ResourceId::from("r1")).unwrap();
    let res = svc.change_min_tier(&admin(), &ResourceId::from("r1"), Tier::Gold);
    assert_eq!(res, Err(DomainError::ResourceNotFound));
}

// -- Listing (RS-S9..S11) ---------------------------------------------

#[test]
fn rs_s9_list_preserves_insertion_order() {
    let mut svc = svc();
    for id in ["r1", "r2", "r3"] {
        create(&mut svc, id, Tier::Silver);
    }
    let ids: Vec<&str> = svc.list().iter().map(|r| r.id.0.as_str()).collect();
    assert_eq!(ids, vec!["r1", "r2", "r3"]);
}

#[test]
fn rs_s10_list_accessible_for_gold_returns_silver_and_gold() {
    let mut svc = svc();
    create(&mut svc, "s1", Tier::Silver);
    create(&mut svc, "g1", Tier::Gold);
    create(&mut svc, "p1", Tier::Platinum);
    let ids: Vec<String> = svc
        .list_accessible_for(Tier::Gold)
        .iter()
        .map(|r| r.id.0.clone())
        .collect();
    assert_eq!(ids, vec!["s1".to_string(), "g1".to_string()]);
}

#[test]
fn rs_s11_list_accessible_for_excludes_soft_deleted() {
    let mut svc = svc();
    create(&mut svc, "s1", Tier::Silver);
    svc.soft_delete(&admin(), &ResourceId::from("s1")).unwrap();
    assert!(svc.list_accessible_for(Tier::Platinum).is_empty());
}
