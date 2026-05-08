// Mirrors src/domain/tier.rs (TP-R1..R4, TP-E1).

export type Tier = "Silver" | "Gold" | "Diamond" | "Platinum";

// Single source of truth for tier ordering (TP-R1, TP-R4).
// To add a new tier: insert it here in rank order — rank(), canAccess(),
// parseTier(), and TierMatrix all derive from this array automatically.
export const ALL_TIERS: readonly Tier[] = ["Silver", "Gold", "Diamond", "Platinum"];

// TP-R1 — numeric rank derived from position in ALL_TIERS (1-based).
// No switch needed — adding a tier only requires updating ALL_TIERS above.
export function rank(tier: Tier): number {
  return ALL_TIERS.indexOf(tier) + 1;
}

// TP-R2 — passenger tier can access a resource whose minimum required tier is `min`.
export function canAccess(passenger: Tier, min: Tier): boolean {
  return rank(passenger) >= rank(min);
}

// TP-E1 — case-sensitive parse.
export function parseTier(value: string): Tier | null {
  return (ALL_TIERS as readonly string[]).includes(value)
    ? (value as Tier)
    : null;
}
