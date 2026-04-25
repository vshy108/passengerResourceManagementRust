import type { CrewLeadId, PassengerId } from "./ids";

// Mirrors src/domain/actor.rs.
export type Actor =
  | { kind: "CrewLead"; id: CrewLeadId }
  | { kind: "Passenger"; id: PassengerId };

export const asCrewLead = (id: CrewLeadId): Actor => ({ kind: "CrewLead", id });
export const asPassenger = (id: PassengerId): Actor => ({
  kind: "Passenger",
  id,
});
