import { ALL_TIERS, canAccess, type Tier } from "../domain/tier";

// Visual proof of TP-R2: renders every (passenger tier, resource min_tier)
// combination and marks it ✓ (allowed) or ✗ (denied) using canAccess().
// No state or side effects — purely derived from the domain logic.
export function TierMatrix(): JSX.Element {
  return (
    <table className="matrix">
      <thead>
        <tr>
          {/* Row headers are passenger tiers; column headers are resource min tiers */}
          <th>passenger \\ resource</th>
          {ALL_TIERS.map((t) => (
            <th key={t}>{t}</th>
          ))}
        </tr>
      </thead>
      <tbody>
        {ALL_TIERS.map((p) => (
          <tr key={p}>
            <th>{p}</th>
            {ALL_TIERS.map((r: Tier) => (
              // CSS class drives the green/red colouring in styles.css
              <td key={r} className={canAccess(p, r) ? "yes" : "no"}>
                {canAccess(p, r) ? "✓" : "✗"}
              </td>
            ))}
          </tr>
        ))}
      </tbody>
    </table>
  );
}
