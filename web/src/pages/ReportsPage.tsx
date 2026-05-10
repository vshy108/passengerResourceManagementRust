import { useCallback, useEffect, useState, type JSX } from "react";
import { api, type ApiUsageEvent } from "../services/api";
import { useData } from "../contexts/DataContext";
import { TierTag } from "../components/TierTag";

export function ReportsPage(): JSX.Element {
  const { state, topN, setTopN, refresh } = useData();
  const firstPid = state.passengers[0]?.id ?? "";
  const [historyPid, setHistoryPid] = useState(firstPid);
  const [history, setHistory] = useState<ApiUsageEvent[]>([]);

  const loadHistory = useCallback(async (pid: string): Promise<void> => {
    if (!pid) return;
    const r = await api.history(pid);
    if (r.ok) setHistory(r.value);
  }, []);

  // Load history whenever the selected passenger or the global data refreshes.
  useEffect(() => {
    const pid = historyPid || firstPid;
    if (pid) void loadHistory(pid);
  }, [historyPid, firstPid, loadHistory]);

  return (
    <section className="page">
      <h2>Reports</h2>

      <h3>By tier (GET /reports/by-tier)</h3>
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
                <TierTag tier={row.tier} />
              </td>
              <td>{row.allowed}</td>
              <td>{row.denied}</td>
            </tr>
          ))}
        </tbody>
      </table>

      <h3>Top resources (GET /reports/top-resources?n=…)</h3>
      <div className="row">
        <label className="muted">n:</label>
        <input
          type="number"
          min={1}
          max={50}
          value={topN}
          onChange={(e) => {
            setTopN(Number(e.target.value) || 1);
            void refresh();
          }}
          style={{ width: 60 }}
        />
      </div>
      <table>
        <thead>
          <tr>
            <th>resource</th>
            <th>allowed count</th>
          </tr>
        </thead>
        <tbody>
          {state.topResources.map((r) => (
            <tr key={r.resource_id}>
              <td>
                <code>{r.resource_id}</code>
              </td>
              <td>{r.allowed_count}</td>
            </tr>
          ))}
          {state.topResources.length === 0 && (
            <tr>
              <td colSpan={2} className="muted">
                no allowed events yet
              </td>
            </tr>
          )}
        </tbody>
      </table>

      <h3>Personal history (GET /reports/history/:passenger_id)</h3>
      <div className="row">
        <select
          value={historyPid || firstPid}
          onChange={(e) => setHistoryPid(e.target.value)}
        >
          {state.passengers.map((p) => (
            <option key={p.id} value={p.id}>
              {p.name} ({p.id})
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
              <th>tier</th>
              <th>min</th>
              <th>result</th>
            </tr>
          </thead>
          <tbody>
            {[...history].reverse().map((e) => (
              <tr key={e.id}>
                <td>{e.id}</td>
                <td>
                  <code>{e.resource_id}</code>
                </td>
                <td>
                  <TierTag tier={e.tier_at_attempt} />
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
            {history.length === 0 && (
              <tr>
                <td colSpan={5} className="muted">
                  no events for this passenger
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </section>
  );
}
