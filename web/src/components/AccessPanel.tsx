import { useState } from "react";
import { asPassenger } from "../domain/actor";
import { useStore } from "../state/useStore";
import { TierTag } from "./TierTag";

export function AccessPanel(): JSX.Element {
  const { world, mutate } = useStore();
  const passengers = world.passengers.list();
  const resources = world.resources.list();
  const [pid, setPid] = useState<string>(passengers[0]?.id ?? "");
  const [rid, setRid] = useState<string>(resources[0]?.id ?? "");

  // Keep selections valid as the lists change.
  // If the stored id was deleted from the list, fall back to the first
  // available option so the dropdowns never show an empty/stale value.
  const validPid = passengers.find((p) => p.id === pid)?.id ?? passengers[0]?.id ?? "";
  const validRid = resources.find((r) => r.id === rid)?.id ?? resources[0]?.id ?? "";

  const attempt = (): void => {
    const passenger = passengers.find((p) => p.id === validPid);
    const resource = resources.find((r) => r.id === validRid);
    if (!passenger || !resource) return;
    mutate((w) => w.access.useResource(asPassenger(passenger.id), passenger.id, resource.id));
  };

  const history = world.access.history();

  return (
    <section className="panel">
      <header>
        <h2>Access attempts (AC)</h2>
      </header>
      <div className="body">
        <div className="row">
          <select value={validPid} onChange={(e) => setPid(e.target.value)}>
            {passengers.map((p) => (
              <option key={p.id} value={p.id}>
                {p.name} ({p.tier})
              </option>
            ))}
          </select>
          <span className="muted">→</span>
          <select value={validRid} onChange={(e) => setRid(e.target.value)}>
            {resources.map((r) => (
              <option key={r.id} value={r.id}>
                {r.name} (min {r.minTier})
              </option>
            ))}
          </select>
          <button onClick={attempt} disabled={!validPid || !validRid}>
            Attempt access
          </button>
        </div>
        <div className="scroll">
          <table>
            <thead>
              <tr>
                <th>#</th>
                <th>passenger</th>
                <th>tier@time</th>
                <th>resource</th>
                <th>min</th>
                <th>result</th>
              </tr>
            </thead>
            <tbody>
              {[...history].reverse().map((e) => (
                <tr key={e.id}>
                  <td>{e.id}</td>
                  <td>
                    <code>{e.passengerId}</code>
                  </td>
                  <td>
                    <TierTag tier={e.passengerTier} />
                  </td>
                  <td>
                    <code>{e.resourceId}</code>
                  </td>
                  <td>
                    <TierTag tier={e.resourceMinTier} />
                  </td>
                  <td>
                    <span className={`tag ${e.allowed ? "allowed" : "denied"}`}>
                      {e.allowed ? "ALLOWED" : "DENIED"}
                    </span>
                  </td>
                </tr>
              ))}
              {history.length === 0 && (
                <tr>
                  <td colSpan={6} className="muted">
                    no attempts yet
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
