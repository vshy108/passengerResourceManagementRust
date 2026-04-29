//! End-to-end lifecycle test — exercises the full happy + sad path
//! across all aggregates in one scenario, complementing the per-rule
//! tests in `tests/{access,passenger,resource,…}.rs`.

// Per-rule tests prove ONE thing each (good for pinpointing failures);
// this lifecycle test proves the WHOLE FLOW composes correctly. Both
// styles complement each other — keep both.

use passenger_resource_management::application::access_service::AccessService;
use passenger_resource_management::application::passenger_service::PassengerService;
use passenger_resource_management::application::ports::UsageEventSource;
use passenger_resource_management::application::reporting_service::ReportingService;
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

#[test]
fn lifecycle_create_deny_upgrade_allow_softdelete_deny() {
    let admin = Actor::CrewLead(CrewLeadId::from("cl-aria"));
    let mut passengers = PassengerService::new(FakeClock::default());
    let mut resources = ResourceService::new(FakeClock::default());
    let mut access = AccessService::new(FakeClock::default(), InMemoryUsageEventSink::new());

    // 1. Crew lead seeds a Silver passenger and a Gold-min resource.
    passengers
        .create(&admin, PassengerId::from("p"), "P".into(), Tier::Silver)
        .unwrap();
    resources
        .create(
            &admin,
            ResourceId::from("r"),
            "R".into(),
            "spa".into(),
            Tier::Gold,
        )
        .unwrap();

    // 2. Silver passenger attempts the Gold-min resource → Denied.
    let actor_p = Actor::Passenger(PassengerId::from("p"));
    let res = access.use_resource(&actor_p, &passengers, &resources, &ResourceId::from("r"));
    assert_eq!(res.err(), Some(DomainError::AccessDenied));
    let evts = access.sink().list();
    assert_eq!(evts.len(), 1);
    assert_eq!(evts[0].outcome, Outcome::Denied);
    assert_eq!(evts[0].tier_at_attempt, Tier::Silver);

    // 3. Crew lead upgrades the passenger to Platinum.
    passengers
        .change_tier(&admin, &PassengerId::from("p"), Tier::Platinum)
        .unwrap();

    // 4. Same passenger retries → Allowed, with current tier captured.
    let event = access
        .use_resource(&actor_p, &passengers, &resources, &ResourceId::from("r"))
        .unwrap();
    assert_eq!(event.outcome, Outcome::Allowed);
    assert_eq!(event.tier_at_attempt, Tier::Platinum);

    // 5. Reporting reflects the two events for this passenger.
    let reporting = ReportingService::new(access.sink());
    let history = reporting.personal_history(&PassengerId::from("p"));
    assert_eq!(history.len(), 2);
    let by_tier = reporting.aggregate_by_tier();
    assert_eq!(by_tier[&Tier::Silver].denied, 1);
    assert_eq!(by_tier[&Tier::Platinum].allowed, 1);

    // 6. Soft-delete the resource; further attempts deny on not-found path.
    resources
        .soft_delete(&admin, &ResourceId::from("r"))
        .unwrap();
    let res = access.use_resource(&actor_p, &passengers, &resources, &ResourceId::from("r"));
    assert_eq!(res.err(), Some(DomainError::ResourceNotFound));
    // No new usage event for not-found (AC contract).
    assert_eq!(access.sink().list().len(), 2);
}
