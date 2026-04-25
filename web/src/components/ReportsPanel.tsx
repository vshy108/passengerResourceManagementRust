import { useState } from "react";
import { passengerId } from "../domain/ids";
import { ALL_TIERS } from "../domain/tier";
import { useStore } from "../state/store";
import { TierTag } from "./TierTag";

export function ReportsPanel(): JSX.Element {
  const { world } = useStore();
  const passengers = world.passengers.list();
  const [topN, setTopN] = useState(3);
  const [pid, setPid] = useState<string>(passengers[0]?.id ?? "");

  const buckets = world.reporting.aggregateByTier();
  const top = world.reporting.topResources(topN);
  const personal = pid ? world.reporting.personalHistory(passengerId(pid)) : [];

  return (
    <section className="panel">
      <header>
        <h2>Reports (RP)</h2>
      </header>
      <div className="body">
        <h3 style={{ margin: 0, fontSize: 13 }}>Aggregate by tier (at time of attempt)</h3>
        <table>
          <thead>
            <tr>
              <th>tier</th>
              <th>allowed</th>
              <th>denied</th>
            </tr>
          </thead>
          <tbody>
            {ALL_TIERS.map((t) => (
              <tr key={t}>
                <td>
                  <TierTag tier={t} />
                </td>
                <td>{buckets[t].allowed}</td>
                <td>{buckets[t].denied}</td>
              </tr>
            ))}
          </tbody>
        </table>

        <div className="row">
          <h3 style={{ margin: 0, fontSize: 13 }}>Top resources</h3>
          <input
            type="number"
            min={0}
            max={20}
            value={topN}
            onChange={(e) => setTopN(Math.max(0, Number(e.target.value)))}
            style={{ width: 70 }}
          />
        </div>
        <table>
          <thead>
            <tr>
              <th>resource</th>
              <th>allowed</th>
            </tr>
          </thead>
          <tbody>
            {top.map((r) => (
              <tr key={r.resourceId}>
                <td>
                  <code>{r.resourceId}</code>
                </td>
                <td>{r.allowed}</td>
              </tr>
            ))}
            {top.length === 0 && (
              <tr>
                <td colSpan={2} className="muted">
                  no allowed attempts yet
                </td>
              </tr>
            )}
          </tbody>
        </table>

        <div className="row">
          <h3 style={{ margin: 0, fontSize: 13 }}>Personal history</h3>
          <select value={pid} onChange={(e) => setPid(e.target.value)}>
            {passengers.map((p) => (
              <option key={p.id} value={p.id}>
                {p.name}
              </option>
            ))}
          </select>
        </div>
        <div className="scroll">
          <table>
            <thead>
              <tr>
                <th>#</th>
                <th>resource</th>
                <th>tier@time</th>
                <th>result</th>
              </tr>
            </thead>
            <tbody>
              {personal.map((e) => (
                <tr key={e.id}>
                  <td>{e.id}</td>
                  <td>
                    <code>{e.resourceId}</code>
                  </td>
                  <td>
                    <TierTag tier={e.passengerTier} />
                  </td>
                  <td>
                    <span className={`tag ${e.allowed ? "allowed" : "denied"}`}>
                      {e.allowed ? "ALLOWED" : "DENIED"}
                    </span>
                  </td>
                </tr>
              ))}
              {personal.length === 0 && (
                <tr>
                  <td colSpan={4} className="muted">
                    no events
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      </div>
    </section>
  );
}
