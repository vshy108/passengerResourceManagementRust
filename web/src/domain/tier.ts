// Mirrors src/domain/tier.rs (TP-R1..R4, TP-E1).

export type Tier = "Silver" | "Gold" | "Platinum";

export const ALL_TIERS: readonly Tier[] = ["Silver", "Gold", "Platinum"];

// TP-R1 — numeric rank used for ordering.
export function rank(tier: Tier): 1 | 2 | 3 {
  switch (tier) {
    case "Silver":
      return 1;
    case "Gold":
      return 2;
    case "Platinum":
      return 3;
  }
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
