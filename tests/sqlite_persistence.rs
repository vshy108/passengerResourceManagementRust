// SQLite entity persistence tests — verifies that entity state (passengers,
// resources, crew leads) survives a simulated restart when using a SQLite
// database. Uses an in-memory SQLite database (":memory:" gives a fresh
// schema per open_db call; we use a temporary file to simulate persistence).
//
// Spec ref: P10 — entity persistence.

// All items in this file require the `http` feature (build_world_with_sqlite
// is gated behind it). The `use` declarations are inside the cfg block so
// `cargo nextest run` (no features) does not fail with E0432.
#[cfg(feature = "http")]
use passenger_resource_management::domain::actor::Actor;
#[cfg(feature = "http")]
use passenger_resource_management::domain::crew_lead::CrewLeadId;
#[cfg(feature = "http")]
use passenger_resource_management::domain::passenger::PassengerId;
#[cfg(feature = "http")]
use passenger_resource_management::domain::resource::ResourceId;
#[cfg(feature = "http")]
use passenger_resource_management::domain::tier::Tier;
#[cfg(feature = "http")]
use passenger_resource_management::interface::composition_root::build_world_with_sqlite;

#[cfg(feature = "http")]
#[test]
fn entity_state_survives_restart_via_sqlite() {
    // Write a temporary file so "run 1" and "run 2" share the same data.
    let dir = std::env::temp_dir();
    let db_path = dir.join(format!(
        "prms_test_{}.db",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    let db_str = db_path.to_str().expect("tempdir path is UTF-8");

    // ── Run 1: first boot — creates and persists demo entities ─────────────
    {
        let mut world = build_world_with_sqlite(db_str).expect("run 1 world build failed");
        // Verify seeded demo data is present.
        assert_eq!(
            world.passengers.list().len(),
            3,
            "run 1 should have 3 passengers"
        );

        // Mutate: change tier of ps-001.
        let admin = Actor::CrewLead(CrewLeadId("cl-aria".into()));
        world
            .passengers
            .change_tier(&admin, &PassengerId("ps-001".into()), Tier::Gold)
            .expect("tier change failed");

        // Mutate: soft-delete res-lounge.
        world
            .resources
            .soft_delete(&admin, &ResourceId("res-lounge".into()))
            .expect("soft delete failed");

        // The World::flush_to_db() is called inside the HTTP handlers,
        // but here we are in a unit-style test driving the World directly.
        // Call flush manually to simulate what the handler would do.
        world.flush_to_db();
    } // `world` dropped; simulated restart.

    // ── Run 2: subsequent boot — restores state, no re-seeding ─────────────
    {
        let world2 = build_world_with_sqlite(db_str).expect("run 2 world build failed");

        // Crew leads restored (3 unchanged).
        assert_eq!(
            world2.crew_leads.list().len(),
            3,
            "run 2 should still have 3 crew leads"
        );

        // Passengers restored with updated tier.
        let pax = world2.passengers.list();
        assert_eq!(pax.len(), 3, "active passenger count unchanged");
        let ps001 = pax
            .iter()
            .find(|p| p.id.0 == "ps-001")
            .expect("ps-001 missing");
        assert_eq!(
            ps001.tier,
            Tier::Gold,
            "tier change should have been persisted"
        );

        // Resources: res-lounge was soft-deleted, so active list has 2.
        assert_eq!(
            world2.resources.list().len(),
            2,
            "soft-deleted resource excluded from active list"
        );
        // Soft-deleted is still resolvable.
        world2
            .resources
            .get(&ResourceId("res-lounge".into()))
            .expect("soft-deleted resource should still be retrievable via get()");
    }

    // Clean up temp file.
    let _ = std::fs::remove_file(&db_path);
    // WAL sidecar files.
    let _ = std::fs::remove_file(db_path.with_extension("db-wal"));
    let _ = std::fs::remove_file(db_path.with_extension("db-shm"));
}
