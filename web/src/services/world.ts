import type { AdminEvent } from "../domain/types";
import { ManualClock } from "./clock";
import { CrewLeadService } from "./crewLeadService";
import { PassengerService } from "./passengerService";
import { ResourceService } from "./resourceService";
import { AccessService } from "./accessService";
import { ReportingService } from "./reportingService";
import { crewLeadId, passengerId, resourceId } from "../domain/ids";
import type { Result } from "../domain/errors";

// Composition root — mirrors src/interface/composition_root.rs.
export interface World {
  clock: ManualClock;
  crewLeads: CrewLeadService;
  passengers: PassengerService;
  resources: ResourceService;
  access: AccessService;
  reporting: ReportingService;
  adminEvents: AdminEvent[];
}

export function buildWorld(): World {
  const clock = new ManualClock();
  const adminEvents: AdminEvent[] = [];
  const emit = (e: AdminEvent): void => {
    adminEvents.push(e);
  };

  const bootstrap: Result<CrewLeadService> = CrewLeadService.bootstrap(
    [
      { id: crewLeadId("cl-aria"), name: "Aria Vance" },
      { id: crewLeadId("cl-bren"), name: "Bren Okafor" },
      { id: crewLeadId("cl-cyra"), name: "Cyra Lin" },
    ],
    clock,
    emit,
  );
  if (!bootstrap.ok) {
    throw new Error(`bootstrap failed: ${bootstrap.error}`);
  }
  const crewLeads = bootstrap.value;
  const passengers = new PassengerService(clock, emit);
  const resources = new ResourceService(clock, emit);
  const access = new AccessService(clock, passengers, resources);
  const reporting = new ReportingService(access);

  // Seed a few passengers + resources so the demo is non-empty.
  const aria = { kind: "CrewLead" as const, id: crewLeadId("cl-aria") };
  passengers.create(aria, passengerId("p-001"), "Iris Kade", "Silver");
  passengers.create(aria, passengerId("p-002"), "Jonas Reed", "Gold");
  passengers.create(aria, passengerId("p-003"), "Lila Soren", "Platinum");

  resources.create(
    aria,
    resourceId("r-pool"),
    "Hydro Lounge",
    "wellness",
    "Silver",
  );
  resources.create(
    aria,
    resourceId("r-deck"),
    "Observation Deck",
    "leisure",
    "Gold",
  );
  resources.create(
    aria,
    resourceId("r-vault"),
    "Captain's Vault",
    "exclusive",
    "Platinum",
  );

  return {
    clock,
    crewLeads,
    passengers,
    resources,
    access,
    reporting,
    adminEvents,
  };
}
