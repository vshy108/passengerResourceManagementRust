import { useCallback, useEffect, useState } from "react";
import { api, type ApiPassenger, type ApiResource, type ApiUsageEvent } from "../services/api";
import { TierTag } from "./TierTag";

type Status = "idle" | "checking" | "online" | "offline";

interface LiveState {
  passengers: ApiPassenger[];
  resources: ApiResource[];
  usage: ApiUsageEvent[];
  byTier: { tier: string; allowed: number; denied: number }[];
}

export function LiveServerPanel(): JSX.Element {
  const [status, setStatus] = useState<Status>("idle");
  const [state, setState] = useState<LiveState>({
    passengers: [],
    resources: [],
    usage: [],
    byTier: [],
  });
  const [pid, setPid] = useState<string>("");
  const [rid, setRid] = useState<string>("");
  const [lastResult, setLastResult] = useState<string>("");

  const refresh = useCallback(async () => {
    const [pax, res, usage, byTier] = await Promise.all([
      api.passengers(),
      api.resources(),
      api.usage(),
      api.byTier(),
    ]);
    if (pax.ok && res.ok && usage.ok && byTier.ok) {
      setState({
        passengers: pax.value,
        resources: res.value,
        usage: usage.value,
        byTier: byTier.value,
      });
      if (!pid && pax.value[0]) setPid(pax.value[0].id);
      if (!rid && res.value[0]) setRid(res.value[0].id);
    }
  }, [pid, rid]);

  const ping = useCallback(async () => {
    setStatus("checking");
    const h = await api.health();
    if (h.ok) {
      setStatus("online");
      await refresh();
    } else {
      setStatus("offline");
    }
  }, [refresh]);

  useEffect(() => {
    void ping();
  }, [ping]);

  const attempt = async (): Promise<void> => {
    if (!pid || !rid) return;
    const r = await api.useResource(pid, rid);
    setLastResult(r.ok ? `Allowed (event #${r.value.id})` : r.error);
    await refresh();
  };

  return (
    <section className="panel">
      <header>
        <h2>Live Rust server (HTTP)</h2>
      </header>
      <div className="body">
        <div className="row">
          <span className="muted">
            base: <code>{api.base}</code>
          </span>
          <span className={`tag ${status === "online" ? "allowed" : "denied"}`}>
            {status.toUpperCase()}
          </span>
          <button onClick={() => void ping()}>Re-check</button>
          <button onClick={() => void refresh()} disabled={status !== "online"}>
            Refresh
          </button>
        </div>

        {status === "offline" && (
          <p className="muted">
            Server unreachable. Start it with{" "}
            <code>cargo run --features http --bin serve</code> from the repo root.
          </p>
        )}

        {status === "online" && (
          <>
            <div className="row">
              <select value={pid} onChange={(e) => setPid(e.target.value)}>
                {state.passengers.map((p) => (
                  <option key={p.id} value={p.id}>
                    {p.name} ({p.tier})
                  </option>
                ))}
              </select>
              <span className="muted">→</span>
              <select value={rid} onChange={(e) => setRid(e.target.value)}>
                {state.resources.map((r) => (
                  <option key={r.id} value={r.id}>
                    {r.name} (min {r.min_tier})
                  </option>
                ))}
              </select>
              <button onClick={() => void attempt()} disabled={!pid || !rid}>
                POST /access
              </button>
              {lastResult && <span className="muted">→ {lastResult}</span>}
            </div>

            <h3>Aggregate by tier (GET /reports/by-tier)</h3>
            <table>
              <thead>
                <tr>
                  <th>tier</th>
                  <th>allowed</th>
                  <th>denied</th>
                </tr>
              </thead>
              <tbody>
                {state.byTier.map((row) => (
                  <tr key={row.tier}>
                    <td>
                      <TierTag tier={row.tier as "Silver" | "Gold" | "Platinum"} />
                    </td>
                    <td>{row.allowed}</td>
                    <td>{row.denied}</td>
                  </tr>
                ))}
              </tbody>
            </table>

            <h3>Recent usage (GET /usage)</h3>
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
                        no attempts on the server yet
                      </td>
                    </tr>
                  )}
                </tbody>
              </table>
            </div>
          </>
        )}
      </div>
    </section>
  );
}
