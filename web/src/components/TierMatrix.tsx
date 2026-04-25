import { ALL_TIERS, canAccess, type Tier } from "../domain/tier";

export function TierMatrix(): JSX.Element {
  return (
    <table className="matrix">
      <thead>
        <tr>
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
