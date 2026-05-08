import type { Tier } from "../services/api";

export function TierTag({ tier }: { tier: Tier }): JSX.Element {
  return <span className={`tag ${tier.toLowerCase()}`}>{tier}</span>;
}
