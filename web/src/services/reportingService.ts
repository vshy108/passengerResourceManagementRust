import type { PassengerId } from "../domain/ids";
import { ALL_TIERS, type Tier } from "../domain/tier";
import type { UsageEvent } from "../domain/types";
import type { AccessService } from "./accessService";

// Mirrors application/reporting_service.rs (RP-R1..R4).
export class ReportingService {
  constructor(private readonly access: AccessService) {}

  // RP-R1 — all events for a passenger in insertion order.
  personalHistory(id: PassengerId): UsageEvent[] {
    return this.access.history().filter((e) => e.passengerId === id);
  }

  // RP-R2/R3 — bucket attempts by the tier captured at the time of the
  // attempt (later tier changes don't reclassify past events).
  aggregateByTier(): Record<Tier, { allowed: number; denied: number }> {
    const buckets = Object.fromEntries(
      ALL_TIERS.map((t) => [t, { allowed: 0, denied: 0 }]),
    ) as Record<Tier, { allowed: number; denied: number }>;
    for (const e of this.access.history()) {
      buckets[e.passengerTier][e.allowed ? "allowed" : "denied"]++;
    }
    return buckets;
  }

  // RP-R4 — top N resources by allowed-attempt count, ties broken by
  // resource id ascending. Denied attempts are ignored.
  topResources(n: number): { resourceId: string; allowed: number }[] {
    if (n <= 0) return [];
    const counts = new Map<string, number>();
    for (const e of this.access.history()) {
      if (e.allowed) counts.set(e.resourceId, (counts.get(e.resourceId) ?? 0) + 1);
    }
    return [...counts.entries()]
      .map(([resourceId, allowed]) => ({ resourceId, allowed }))
      .sort((a, b) => b.allowed - a.allowed || a.resourceId.localeCompare(b.resourceId))
      .slice(0, n);
  }
}
