import { describe, expect, test } from "vitest";
import { buildWorld } from "../world";
import { passengerId, resourceId } from "../../domain/ids";

describe("buildWorld (composition root)", () => {
  test("seeds 3 crew leads, 3 passengers, 3 resources", () => {
    const w = buildWorld();
    expect(w.crewLeads.list().length).toBe(3);
    expect(w.passengers.list().length).toBe(3);
    expect(w.resources.list().length).toBe(3);
  });

  test("emits one bootstrap admin event up front", () => {
    const w = buildWorld();
    const bootstrap = w.adminEvents.filter(
      (e) => e.action === "CrewLeadBootstrapped",
    );
    expect(bootstrap.length).toBe(1);
  });

  test("silver passenger denied access to platinum resource", () => {
    const w = buildWorld();
    const actor = { kind: "Passenger" as const, id: passengerId("p-001") };
    const r = w.access.useResource(
      actor,
      passengerId("p-001"),
      resourceId("r-vault"),
    );
    expect(r.ok).toBe(true);
    if (r.ok) {
      expect(r.value.allowed).toBe(false);
    }
  });

  test("platinum passenger allowed on platinum resource", () => {
    const w = buildWorld();
    const actor = { kind: "Passenger" as const, id: passengerId("p-003") };
    const r = w.access.useResource(
      actor,
      passengerId("p-003"),
      resourceId("r-vault"),
    );
    expect(r.ok).toBe(true);
    if (r.ok) {
      expect(r.value.allowed).toBe(true);
    }
  });

  test("crew lead actor cannot use a resource (AC-S1)", () => {
    const w = buildWorld();
    const actor = w.crewLeads.list()[0]!;
    const r = w.access.useResource(
      { kind: "CrewLead", id: actor.id },
      passengerId("p-001"),
      resourceId("r-pool"),
    );
    expect(r.ok).toBe(false);
    if (!r.ok) expect(r.error).toBe("UnauthorizedActor");
  });
});
