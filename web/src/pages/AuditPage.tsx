import { type JSX } from "react";
import { useData } from "../contexts/DataContext";
import { TierTag } from "../components/TierTag";

export function AuditPage(): JSX.Element {
  const { state } = useData();

  return (
    <section className="page">
      <h2>Audit Log</h2>

      <h3>Usage events (GET /usage)</h3>
      <div className="scroll">
        <table>
          <thead>
            <tr>
              <th>#</th>
              <th>passenger</th>
              <th>tier</th>
              <th>resource</th>
              <th>min</th>
              <th>result</th>
            </tr>
          </thead>
          <tbody>
            {[...state.usage].reverse().map((e) => (
              <tr key={e.id}>
                <td>{e.id}</td>
                <td>
                  <code>{e.passenger_id}</code>
                </td>
                <td>
                  <TierTag tier={e.tier_at_attempt} />
                </td>
                <td>
                  <code>{e.resource_id}</code>
                </td>
                <td>
                  <TierTag tier={e.min_tier_at_attempt} />
                </td>
                <td>
                  <span
                    className={`tag ${e.outcome === "Allowed" ? "allowed" : "denied"}`}
                  >
                    {e.outcome.toUpperCase()}
                  </span>
                </td>
              </tr>
            ))}
            {state.usage.length === 0 && (
              <tr>
                <td colSpan={6} className="muted">
                  no access attempts yet
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>

      <h3>Admin audit (GET /audit)</h3>
      <div className="scroll">
        <table>
          <thead>
            <tr>
              <th>#</th>
              <th>actor</th>
              <th>action</th>
              <th>target</th>
              <th>details</th>
            </tr>
          </thead>
          <tbody>
            {[...state.audit].reverse().map((e) => (
              <tr key={e.id}>
                <td>{e.id}</td>
                <td>
                  <code>{e.actor_id}</code>
                </td>
                <td>{e.action}</td>
                <td>
                  <code>
                    {e.target_kind}/{e.target_id}
                  </code>
                </td>
                <td className="muted">{e.details ?? ""}</td>
              </tr>
            ))}
            {state.audit.length === 0 && (
              <tr>
                <td colSpan={5} className="muted">
                  no admin events yet
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </section>
  );
}
