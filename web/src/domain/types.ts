import type { CrewLeadId, PassengerId, ResourceId } from "./ids";
import type { Tier } from "./tier";

// Monotonically increasing integer supplied by ManualClock.
// Using `number` (not Date) keeps the domain pure — no wall-clock calls.
export type Timestamp = number;

// Mirrors src/domain/passenger.rs.
export interface Passenger {
  id: PassengerId;
  name: string;
  tier: Tier;
  // null  → record is active.
  // number → soft-deleted at this timestamp; excluded from list() but
  //          still resolvable via get() for audit purposes (PS-R5).
  deletedAt: Timestamp | null;
}

// Mirrors src/domain/resource.rs.
export interface Resource {
  id: ResourceId;
  name: string;
  // Free-form group label (e.g. "food", "medical"). Not enumerated.
  category: string;
  // Minimum tier required to access this resource (TP-R2).
  minTier: Tier;
  // Same soft-delete semantics as Passenger.deletedAt (RS-R4).
  deletedAt: Timestamp | null;
}

// Mirrors src/domain/crew_lead.rs.
export interface CrewLead {
  id: CrewLeadId;
  name: string;
}

// AC-R5 — every access attempt emits a UsageEvent (allowed OR denied).
// Tier fields are snapshots: later tier changes do NOT reclassify past
// events (AC-R6 / RP-R3).
export interface UsageEvent {
  id: number;
  passengerId: PassengerId;
  passengerTier: Tier; // tier the passenger held AT THE TIME of the attempt
  resourceId: ResourceId;
  resourceMinTier: Tier; // min_tier the resource required AT THE TIME
  // true = Allowed, false = Denied — mirrors the Rust Outcome enum.
  allowed: boolean;
  timestamp: Timestamp;
}

// Closed set of admin mutations that emit an AdminEvent (AU-R3).
export type AdminAction =
  | "CrewLeadBootstrapped"
  | "CrewLeadReplaced"
  | "PassengerCreated"
  | "PassengerTierChanged"
  | "PassengerDeleted"
  | "ResourceCreated"
  | "ResourceMinTierChanged"
  | "ResourceDeleted";

// Identifies which entity type target_id refers to in an AdminEvent.
export type TargetKind = "CrewLead" | "Passenger" | "Resource";

// AU-R2 — append-only record of a successful admin mutation.
export interface AdminEvent {
  id: number;
  actorId: CrewLeadId;   // always a Crew Lead — only leads mutate state
  action: AdminAction;
  targetKind: TargetKind;
  targetId: string;       // string because it can be any of the three ID types
  timestamp: Timestamp;
  details: string | null; // optional free-form context, e.g. "tier Silver → Gold"
}
