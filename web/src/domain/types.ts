import type { CrewLeadId, PassengerId, ResourceId } from "./ids";
import type { Tier } from "./tier";

export type Timestamp = number;

export interface Passenger {
  id: PassengerId;
  name: string;
  tier: Tier;
  deletedAt: Timestamp | null;
}

export interface Resource {
  id: ResourceId;
  name: string;
  category: string;
  minTier: Tier;
  deletedAt: Timestamp | null;
}

export interface CrewLead {
  id: CrewLeadId;
  name: string;
}

// AC-R5 — every access attempt emits a UsageEvent (allowed OR denied).
export interface UsageEvent {
  id: number;
  passengerId: PassengerId;
  passengerTier: Tier; // tier captured at time of attempt (RP-R3)
  resourceId: ResourceId;
  resourceMinTier: Tier;
  allowed: boolean;
  timestamp: Timestamp;
}

export type AdminAction =
  | "CrewLeadBootstrapped"
  | "CrewLeadReplaced"
  | "PassengerCreated"
  | "PassengerTierChanged"
  | "PassengerDeleted"
  | "ResourceCreated"
  | "ResourceMinTierChanged"
  | "ResourceDeleted";

export type TargetKind = "CrewLead" | "Passenger" | "Resource";

export interface AdminEvent {
  id: number;
  actorId: CrewLeadId;
  action: AdminAction;
  targetKind: TargetKind;
  targetId: string;
  timestamp: Timestamp;
  details: string | null;
}
