import type { Tier } from "../domain/tier";

export function TierTag({ tier }: { tier: Tier }): JSX.Element {
  return <span className={`tag ${tier.toLowerCase()}`}>{tier}</span>;
}
