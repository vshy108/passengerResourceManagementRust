import { useState, type JSX } from "react";
import { api } from "../services/api";
import { useAuth } from "../contexts/AuthContext";
import { useData } from "../contexts/DataContext";

export function DashboardPage(): JSX.Element {
  const { status, state, refresh } = useData();
  const { token, login } = useAuth();
  const [flash, setFlash] = useState("");

  const announce = (msg: string): void => {
    setFlash(msg);
    setTimeout(() => setFlash(""), 4000);
  };

  const reset = async (): Promise<void> => {
    if (
      !window.confirm(
        "Reset server state? All passengers, resources and audit history will be replaced with the seeded demo world.",
      )
    )
      return;
    const r = await api.reset();
    announce(r.ok ? "Server state reset" : `reset failed: ${r.error}`);
    await refresh();
  };

  const online = status === "online";

  return (
    <section className="page">
      <h2>Dashboard</h2>

      {status === "offline" && (
        <p className="muted" data-testid="offline-msg">
          Server unreachable. Start it with{" "}
          <code>cargo run --features http --bin serve</code> from the repo root.
        </p>
      )}

      {online && (
        <>
          <div className="stats-row">
            <a href="#/crew-leads" className="stat-card">
              <div className="stat-num">{state.crewLeads.length}</div>
              <div className="stat-label">Crew Leads</div>
            </a>
            <a href="#/passengers" className="stat-card">
              <div className="stat-num">{state.passengers.length}</div>
              <div className="stat-label">Passengers</div>
            </a>
            <a href="#/resources" className="stat-card">
              <div className="stat-num">{state.resources.length}</div>
              <div className="stat-label">Resources</div>
            </a>
            <a href="#/audit" className="stat-card">
              <div className="stat-num">{state.usage.length}</div>
              <div className="stat-label">Access Events</div>
            </a>
          </div>

          <div className="row" style={{ marginTop: 16 }}>
            <label className="muted">Acting as:</label>
            <select
              value={token ?? ""}
              onChange={(e) => {
                // FIX: update both the bearer token and auth context when
                // switching crew-lead identity from the dashboard picker.
                login(e.target.value);
              }}
            >
              {state.crewLeads.map((c) => (
                <option key={c.id} value={c.id}>
                  {c.name} ({c.id})
                </option>
              ))}
            </select>
          </div>

          <div className="row" style={{ marginTop: 8 }}>
            <button onClick={() => void reset()}>Reset server state</button>
            {flash && <span className="muted">→ {flash}</span>}
          </div>
        </>
      )}
    </section>
  );
}
