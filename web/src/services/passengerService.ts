import type { Actor } from "../domain/actor";
import { err, ok, type Result } from "../domain/errors";
import type { CrewLeadId, PassengerId } from "../domain/ids";
import type { Tier } from "../domain/tier";
import type { AdminEvent, Passenger } from "../domain/types";
import type { ManualClock } from "./clock";
import { requireCrewLead } from "./guards";

// Mirrors application/passenger_service.rs (PS-R1..R9).
export class PassengerService {
  private active: Passenger[] = [];
  private deleted: Passenger[] = [];
  private nextAuditId = 1;
  constructor(
    private readonly clock: ManualClock,
    private readonly emit: (e: AdminEvent) => void,
  ) {}

  // PS-R1.
  create(actor: Actor, id: PassengerId, name: string, tier: Tier): Result<Passenger> {
    const guard = requireCrewLead(actor);
    if (!guard.ok) return guard;
    if (this.active.some((p) => p.id === id)) return err("PassengerAlreadyExists");
    const p: Passenger = { id, name, tier, deletedAt: null };
    this.active.push(p);
    this.emitEvent(guard.value, "PassengerCreated", p.id, `tier=${tier}`);
    return ok(p);
  }

  // PS-R3/R4 — Crew-Lead-only tier change (idempotent).
  changeTier(actor: Actor, id: PassengerId, newTier: Tier): Result<void> {
    const guard = requireCrewLead(actor);
    if (!guard.ok) return guard;
    const slot = this.active.find((p) => p.id === id);
    if (!slot) return err("PassengerNotFound");
    slot.tier = newTier;
    this.emitEvent(guard.value, "PassengerTierChanged", id, `tier=${newTier}`);
    return ok(undefined);
  }

  // PS-R5 — soft delete.
  softDelete(actor: Actor, id: PassengerId): Result<void> {
    const guard = requireCrewLead(actor);
    if (!guard.ok) return guard;
    const idx = this.active.findIndex((p) => p.id === id);
    if (idx === -1) return err("PassengerNotFound");
    const [p] = this.active.splice(idx, 1);
    p!.deletedAt = this.clock.now();
    this.deleted.push(p!);
    this.emitEvent(guard.value, "PassengerDeleted", id, null);
    return ok(undefined);
  }

  // PS-R8.
  list(): readonly Passenger[] {
    return this.active;
  }

  // PS-R9 — active first, else most-recent soft-deleted.
  get(id: PassengerId): Result<Passenger> {
    const live = this.active.find((p) => p.id === id);
    if (live) return ok(live);
    for (let i = this.deleted.length - 1; i >= 0; i--) {
      if (this.deleted[i]!.id === id) return ok(this.deleted[i]!);
    }
    return err("PassengerNotFound");
  }

  private emitEvent(
    actorId: CrewLeadId,
    action: "PassengerCreated" | "PassengerTierChanged" | "PassengerDeleted",
    targetId: PassengerId,
    details: string | null,
  ): void {
    this.emit({
      id: this.nextAuditId++,
      actorId,
      action,
      targetKind: "Passenger",
      targetId,
      timestamp: this.clock.now(),
      details,
    });
  }
}
