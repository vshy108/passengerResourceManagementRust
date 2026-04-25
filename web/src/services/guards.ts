import type { Actor } from "../domain/actor";
import { err, ok, type Result } from "../domain/errors";
import type { CrewLeadId } from "../domain/ids";

// Mirrors application/guards.rs — returns the inner CrewLeadId so callers
// can pass it to audit emission without re-pattern-matching.
export function requireCrewLead(actor: Actor): Result<CrewLeadId> {
  return actor.kind === "CrewLead" ? ok(actor.id) : err("UnauthorizedActor");
}
