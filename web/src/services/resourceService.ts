import type { Actor } from "../domain/actor";
import { err, ok, type Result } from "../domain/errors";
import type { CrewLeadId, ResourceId } from "../domain/ids";
import { canAccess, type Tier } from "../domain/tier";
import type { AdminEvent, Resource } from "../domain/types";
import type { ManualClock } from "./clock";
import { requireCrewLead } from "./guards";

// Mirrors application/resource_service.rs (RS-R1..R7).
export class ResourceService {
  private active: Resource[] = [];
  private deleted: Resource[] = [];
  private nextAuditId = 1;
  constructor(
    private readonly clock: ManualClock,
    private readonly emit: (e: AdminEvent) => void,
  ) {}

  create(
    actor: Actor,
    id: ResourceId,
    name: string,
    category: string,
    minTier: Tier,
  ): Result<Resource> {
    const guard = requireCrewLead(actor);
    if (!guard.ok) return guard;
    if (this.active.some((r) => r.id === id))
      return err("ResourceAlreadyExists");
    const r: Resource = { id, name, category, minTier, deletedAt: null };
    this.active.push(r);
    this.audit(guard.value, "ResourceCreated", id, `min_tier=${minTier}`);
    return ok(r);
  }

  changeMinTier(actor: Actor, id: ResourceId, newTier: Tier): Result<void> {
    const guard = requireCrewLead(actor);
    if (!guard.ok) return guard;
    const slot = this.active.find((r) => r.id === id);
    if (!slot) return err("ResourceNotFound");
    slot.minTier = newTier;
    this.audit(
      guard.value,
      "ResourceMinTierChanged",
      id,
      `min_tier=${newTier}`,
    );
    return ok(undefined);
  }

  softDelete(actor: Actor, id: ResourceId): Result<void> {
    const guard = requireCrewLead(actor);
    if (!guard.ok) return guard;
    const idx = this.active.findIndex((r) => r.id === id);
    if (idx === -1) return err("ResourceNotFound");
    const [r] = this.active.splice(idx, 1);
    r!.deletedAt = this.clock.now();
    this.deleted.push(r!);
    this.audit(guard.value, "ResourceDeleted", id, null);
    return ok(undefined);
  }

  list(): readonly Resource[] {
    return this.active;
  }

  // RS-R7 — resources accessible by a given tier.
  listAccessibleFor(tier: Tier): Resource[] {
    return this.active.filter((r) => canAccess(tier, r.minTier));
  }

  get(id: ResourceId): Result<Resource> {
    const live = this.active.find((r) => r.id === id);
    if (live) return ok(live);
    for (let i = this.deleted.length - 1; i >= 0; i--) {
      if (this.deleted[i]!.id === id) return ok(this.deleted[i]!);
    }
    return err("ResourceNotFound");
  }

  private audit(
    actorId: CrewLeadId,
    action: "ResourceCreated" | "ResourceMinTierChanged" | "ResourceDeleted",
    targetId: ResourceId,
    details: string | null,
  ): void {
    this.emit({
      id: this.nextAuditId++,
      actorId,
      action,
      targetKind: "Resource",
      targetId,
      timestamp: this.clock.now(),
      details,
    });
  }
}
