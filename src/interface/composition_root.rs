//! Composition root — wires concrete adapters into application
//! services and seeds the demo state. The HTTP server (and any future
//! adapter) consumes the resulting [`World`] inside an `Arc<Mutex<…>>`.

use crate::application::access_service::AccessService;
use crate::application::crew_lead_service::CrewLeadService;
use crate::application::passenger_service::PassengerService;
use crate::application::resource_service::ResourceService;
use crate::domain::actor::Actor;
use crate::domain::crew_lead::{CrewLead, CrewLeadId};
use crate::domain::errors::DomainError;
use crate::domain::passenger::PassengerId;
use crate::domain::resource::ResourceId;
use crate::domain::tier::Tier;
use crate::infrastructure::fake_clock::FakeClock;
use crate::infrastructure::in_memory_admin_event_sink::InMemoryAdminEventSink;
use crate::infrastructure::in_memory_usage_event_sink::InMemoryUsageEventSink;

/// In-process state owned by exactly one HTTP server. Held inside a
/// `Mutex` by the caller; not internally synchronised.
// Composition Root pattern (Dependency Injection): a single struct
// bundling the wired services. Built once at startup, shared via
// `Arc<Mutex<World>>` to handlers. Services know nothing about each
// other's wiring — only this file does.
pub struct World {
    pub crew_leads: CrewLeadService,
    pub passengers: PassengerService<FakeClock>,
    pub resources: ResourceService<FakeClock>,
    pub access: AccessService<FakeClock, InMemoryUsageEventSink>,
    /// Clone-able handle on the same shared admin-event buffer the
    /// services write to — exposed so reporting endpoints can read it.
    pub audit_sink: InMemoryAdminEventSink,
}

/// Build a fresh demo world: 3 seeded Crew Leads, 3 sample passengers,
/// 3 sample resources. Mirrors the TypeScript demo's `seedWorld`.
///
/// # Errors
/// Propagates any `DomainError` from the underlying services. With the
/// hard-coded seed data this should not happen in practice.
pub fn build_demo_world() -> Result<World, DomainError> {
    // Single audit sink. We `.clone()` the Arc-backed handle into each
    // service so they all write to the SAME underlying buffer. Cloning
    // the sink is cheap (one pointer bump) — see InMemoryAdminEventSink.
    let audit_sink = InMemoryAdminEventSink::new();

    let crew_leads = CrewLeadService::bootstrap_audited(
        // `vec![...]` macro builds a Vec from comma-separated literals.
        vec![
            CrewLead {
                id: CrewLeadId::from("cl-aria"),
                // `"Aria Vega".into()` -> String via `From<&str> for String`.
                name: "Aria Vega".into(),
            },
            CrewLead {
                id: CrewLeadId::from("cl-noor"),
                name: "Noor Hadid".into(),
            },
            CrewLead {
                id: CrewLeadId::from("cl-jun"),
                name: "Jun Park".into(),
            },
        ],
        // `Box::new(...)` heap-allocates so we can store as `Box<dyn Trait>`.
        Box::new(FakeClock::default()),
        Box::new(audit_sink.clone()),
    )?;

    // Builder chain: `new(...).with_audit(...)` — see PassengerService
    // for the with_audit details.
    let mut passengers =
        PassengerService::new(FakeClock::default()).with_audit(Box::new(audit_sink.clone()));
    let mut resources =
        ResourceService::new(FakeClock::default()).with_audit(Box::new(audit_sink.clone()));
    // AccessService doesn't emit ADMIN events (it emits USAGE events),
    // so its sink is the in-memory usage event sink — no audit handle.
    let access = AccessService::new(FakeClock::default(), InMemoryUsageEventSink::new());

    // Synthetic admin used for seeding. Not exposed externally.
    let admin: Actor = Actor::CrewLead(CrewLeadId::from("cl-aria"));

    passengers.create(
        &admin,
        PassengerId::from("ps-001"),
        "Mira Voss".into(),
        Tier::Silver,
    )?;
    passengers.create(
        &admin,
        PassengerId::from("ps-002"),
        "Kai Reeves".into(),
        Tier::Gold,
    )?;
    passengers.create(
        &admin,
        PassengerId::from("ps-003"),
        "Lena Ito".into(),
        Tier::Platinum,
    )?;

    resources.create(
        &admin,
        ResourceId::from("res-lounge"),
        "Stardeck Lounge".into(),
        "social".into(),
        Tier::Silver,
    )?;
    resources.create(
        &admin,
        ResourceId::from("res-spa"),
        "Zero-G Spa".into(),
        "wellness".into(),
        Tier::Gold,
    )?;
    resources.create(
        &admin,
        ResourceId::from("res-bridge"),
        "Bridge Tour".into(),
        "experience".into(),
        Tier::Platinum,
    )?;

    Ok(World {
        crew_leads,
        passengers,
        resources,
        access,
        audit_sink,
    })
}
