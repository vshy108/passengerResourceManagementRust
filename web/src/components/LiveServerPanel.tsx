import { useCallback, useEffect, useMemo, useState, type JSX } from "react";
import {
  api,
  type ApiAdminEvent,
  type ApiCrewLead,
  type ApiPassenger,
  type ApiResource,
  type ApiTierCount,
  type ApiTopResource,
  type ApiUsageEvent,
  type Tier,
} from "../services/api";
import { TierTag } from "./TierTag";

type Status = "idle" | "checking" | "online" | "offline";

const TIERS: Tier[] = ["Silver", "Gold", "Diamond", "Platinum"];

interface LiveState {
  crewLeads: ApiCrewLead[];
  passengers: ApiPassenger[];
  resources: ApiResource[];
  usage: ApiUsageEvent[];
  audit: ApiAdminEvent[];
  byTier: ApiTierCount[];
  topResources: ApiTopResource[];
  history: ApiUsageEvent[];
}

const EMPTY: LiveState = {
  crewLeads: [],
  passengers: [],
  resources: [],
  usage: [],
  audit: [],
  byTier: [],
  topResources: [],
  history: [],
};

export function LiveServerPanel(): JSX.Element {
  const [status, setStatus] = useState<Status>("idle");
  const [state, setState] = useState<LiveState>(EMPTY);
  const [actorId, setActorId] = useState<string>("");
  const [historyPid, setHistoryPid] = useState<string>("");
  const [topN, setTopN] = useState<number>(5);
  const [flash, setFlash] = useState<string>("");
  // FIX: track 401 Unauthorized responses centrally so any handler failure
  // surfaces a visible banner rather than silently failing per call-site.
  const [authError, setAuthError] = useState<boolean>(false);

  useEffect(() => {
    // Subscribe to 401 events emitted by the api layer. Clear on token change.
    return api.onUnauthorized(() => setAuthError(true));
  }, []);

  const refresh = useCallback(
    async (pidForHistory?: string): Promise<void> => {
      const [cl, pax, res, usage, audit, byTier, top] = await Promise.all([
        api.crewLeads(),
        api.passengers(),
        api.resources(),
        api.usage(),
        api.audit(),
        api.byTier(),
        api.topResources(topN),
      ]);
      const targetPid = pidForHistory ?? historyPid;
      const history =
        targetPid !== ""
          ? await api.history(targetPid)
          : { ok: true as const, value: [] };
      if (
        cl.ok &&
        pax.ok &&
        res.ok &&
        usage.ok &&
        audit.ok &&
        byTier.ok &&
        top.ok &&
        history.ok
      ) {
        setState({
          crewLeads: cl.value,
          passengers: pax.value,
          resources: res.value,
          usage: usage.value,
          audit: audit.value,
          byTier: byTier.value,
          topResources: top.value,
          history: history.value,
        });
        if (actorId === "" && cl.value[0]) {
          setActorId(cl.value[0].id);
          // FIX: actor identity now derived from bearer token, not request body.
          // In demo mode, token == actor-id (server started with matching PRMS_API_KEYS).
          api.setToken(cl.value[0].id);
        }
        if (historyPid === "" && pax.value[0]) setHistoryPid(pax.value[0].id);
      }
    },
    [actorId, historyPid, topN],
  );

  const ping = useCallback(async (): Promise<void> => {
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

  const announce = (msg: string): void => {
    setFlash(msg);
    window.setTimeout(() => setFlash(""), 4000);
  };

  return (
    <section className="panel" data-testid="live-panel">
      <header>
        <h2>Live Rust server (HTTP)</h2>
      </header>
      {authError && (
        <div
          className="tag denied"
          style={{ margin: "0.5rem 1rem", padding: "0.5rem 1rem", borderRadius: 4 }}
          data-testid="auth-error-banner"
        >
          ⚠️ 401 Unauthorized — check your API token (PRMS_API_KEYS).{" "}
          <button onClick={() => setAuthError(false)} style={{ marginLeft: "0.5rem" }}>
            Dismiss
          </button>
        </div>
      )}
      <div className="body">
        <div className="row">
          <span className="muted">
            base: <code>{api.base}</code>
          </span>
          <span className={`tag ${status === "online" ? "allowed" : "denied"}`} data-testid="server-status">
            {status.toUpperCase()}
          </span>
          <button onClick={() => void ping()} data-testid="btn-recheck" aria-label="Re-check server connection">Re-check</button>
          <button onClick={() => void refresh()} disabled={status !== "online"} data-testid="btn-refresh" aria-label="Refresh all data from server">
            Refresh all
          </button>
          <button
            onClick={async () => {
              if (
                !window.confirm(
                  "Reset live server state? All passengers, resources and audit history will be replaced with the seeded demo world.",
                )
              ) {
                return;
              }
              const r = await api.reset();
              announce(r.ok ? "Server state reset" : `reset failed: ${r.error}`);
              await refresh();
            }}
            disabled={status !== "online"}
          >
            Reset server state
          </button>
      {flash && (
        // aria-live="polite" announces flash messages to screen readers
        // without interrupting the current reading context (WCAG 4.1.3).
        <span className="muted" aria-live="polite" aria-atomic="true">→ {flash}</span>
      )}
        </div>

        {status === "offline" && (
          <p className="muted" data-testid="offline-msg">
            Server unreachable. Start it with{" "}
            <code>cargo run --features http --bin serve</code> from the repo root.
          </p>
        )}

        {status === "online" && (
          <>
            <div className="row">
              <label htmlFor="acting-crew-lead" className="muted">Acting Crew Lead:</label>
              <select
                id="acting-crew-lead"
                value={actorId}
                onChange={(e) => {
                  setActorId(e.target.value);
                  // FIX: update bearer token when crew-lead selection changes.
                  // Clear any stale 401 banner so the new token gets a clean slate.
                  api.setToken(e.target.value);
                  setAuthError(false);
                }}
                aria-label="Select acting crew lead"
              >
                {state.crewLeads.map((c) => (
                  <option key={c.id} value={c.id}>
                    {c.name} ({c.id})
                  </option>
                ))}
              </select>
            </div>

            <CrewLeadsSection
              crewLeads={state.crewLeads}
              onChange={(msg) => {
                announce(msg);
                void refresh();
              }}
            />

            <PassengersSection
              passengers={state.passengers}
              onChange={(msg) => {
                announce(msg);
                void refresh();
              }}
            />

            <ResourcesSection
              resources={state.resources}
              onChange={(msg) => {
                announce(msg);
                void refresh();
              }}
            />

            <AccessSection
              passengers={state.passengers}
              resources={state.resources}
              onResult={(msg) => {
                announce(msg);
                void refresh();
              }}
            />

            <AccessibleSection />
            <ReportsSection
              byTier={state.byTier}
              topResources={state.topResources}
              history={state.history}
              passengers={state.passengers}
              topN={topN}
              setTopN={(n) => {
                setTopN(n);
                void refresh();
              }}
              historyPid={historyPid}
              setHistoryPid={(pid) => {
                setHistoryPid(pid);
                void refresh(pid);
              }}
            />

            <UsageSection usage={state.usage} />
            <AuditSection audit={state.audit} />
          </>
        )}
      </div>
    </section>
  );
}

// ---------- Crew Leads -------------------------------------------------

function CrewLeadsSection({
  crewLeads,
  onChange,
}: {
  crewLeads: ApiCrewLead[];
  onChange: (msg: string) => void;
}): JSX.Element {
  const [oldId, setOldId] = useState<string>("");
  const [newId, setNewId] = useState<string>("");
  const [newName, setNewName] = useState<string>("");

  const submit = async (): Promise<void> => {
    if (!oldId || !newId || !newName) return;
    const r = await api.replaceCrewLead(oldId, { id: newId, name: newName });
    onChange(r.ok ? `Replaced ${oldId} → ${newId}` : `replace failed: ${r.error}`);
    if (r.ok) {
      setOldId("");
      setNewId("");
      setNewName("");
    }
  };

  return (
    <>
      <h3>Crew Leads (PUT /crew-leads/:old_id)</h3>
      <table>
        <thead>
          <tr>
            <th>id</th>
            <th>name</th>
          </tr>
        </thead>
        <tbody>
          {crewLeads.map((c) => (
            <tr key={c.id}>
              <td>
                <code>{c.id}</code>
              </td>
              <td>{c.name}</td>
            </tr>
          ))}
        </tbody>
      </table>
      <div className="row">
        <select value={oldId} onChange={(e) => setOldId(e.target.value)}>
          <option value="">replace…</option>
          {crewLeads.map((c) => (
            <option key={c.id} value={c.id}>
              {c.id}
            </option>
          ))}
        </select>
        <input
          placeholder="new id"
          value={newId}
          onChange={(e) => setNewId(e.target.value)}
        />
        <input
          placeholder="new name"
          value={newName}
          onChange={(e) => setNewName(e.target.value)}
        />
        <button onClick={() => void submit()} disabled={!oldId || !newId || !newName}>
          Replace
        </button>
      </div>
    </>
  );
}

// ---------- Passengers -------------------------------------------------

function PassengersSection({
  passengers,
  onChange,
}: {
  passengers: ApiPassenger[];
  onChange: (msg: string) => void;
}): JSX.Element {
  const [id, setId] = useState<string>("");
  const [name, setName] = useState<string>("");
  const [tier, setTier] = useState<Tier>("Silver");

  const create = async (): Promise<void> => {
    if (!id || !name) return;
    const r = await api.createPassenger(id, name, tier);
    onChange(r.ok ? `Created passenger ${id}` : `create failed: ${r.error}`);
    if (r.ok) {
      setId("");
      setName("");
    }
  };

  const changeTier = async (p: ApiPassenger, t: Tier): Promise<void> => {
    const r = await api.changePassengerTier(p.id, t, p.version);
    onChange(r.ok ? `${p.id} → ${t}` : `change failed: ${r.error}`);
  };

  const remove = async (p: ApiPassenger): Promise<void> => {
    const r = await api.softDeletePassenger(p.id, p.version);
    onChange(r.ok ? `Deleted ${p.id}` : `delete failed: ${r.error}`);
  };

  return (
    <>
      <h3>Passengers (POST/PATCH/DELETE /passengers)</h3>
      <table data-testid="passengers-table">
        <thead>
          <tr>
            <th>id</th>
            <th>name</th>
            <th>tier</th>
            <th>actions</th>
          </tr>
        </thead>
        <tbody>
          {passengers.map((p) => (
            <tr key={p.id}>
              <td>
                <code>{p.id}</code>
              </td>
              <td>{p.name}</td>
              <td>
                <TierTag tier={p.tier} />
              </td>
              <td>
                <select
                  value={p.tier}
                  aria-label={`Change tier for ${p.name}`}
                  onChange={(e) => void changeTier(p, e.target.value as Tier)}
                >
                  {TIERS.map((t) => (
                    <option key={t} value={t}>
                      → {t}
                    </option>
                  ))}
                </select>
                <button onClick={() => void remove(p)} aria-label={`Delete passenger ${p.name}`}>Delete</button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      <div className="row">
        <input placeholder="id" value={id} onChange={(e) => setId(e.target.value)} aria-label="Passenger ID" />
        <input
          placeholder="name"
          value={name}
          onChange={(e) => setName(e.target.value)}
          aria-label="Passenger name"
        />
        <select value={tier} onChange={(e) => setTier(e.target.value as Tier)} aria-label="Passenger tier">
          {TIERS.map((t) => (
            <option key={t} value={t}>
              {t}
            </option>
          ))}
        </select>
        <button onClick={() => void create()} disabled={!id || !name} data-testid="btn-create-passenger" aria-label="Create passenger">
          Create passenger
        </button>
      </div>
    </>
  );
}

// ---------- Resources --------------------------------------------------

function ResourcesSection({
  resources,
  onChange,
}: {
  resources: ApiResource[];
  onChange: (msg: string) => void;
}): JSX.Element {
  const [id, setId] = useState<string>("");
  const [name, setName] = useState<string>("");
  const [category, setCategory] = useState<string>("general");
  const [minTier, setMinTier] = useState<Tier>("Silver");

  const create = async (): Promise<void> => {
    if (!id || !name) return;
    const r = await api.createResource(id, name, category, minTier);
    onChange(r.ok ? `Created resource ${id}` : `create failed: ${r.error}`);
    if (r.ok) {
      setId("");
      setName("");
    }
  };

  const changeMin = async (resource: ApiResource, t: Tier): Promise<void> => {
    const r = await api.changeResourceMinTier(resource.id, t, resource.version);
    onChange(r.ok ? `${resource.id} min → ${t}` : `change failed: ${r.error}`);
  };

  const remove = async (resource: ApiResource): Promise<void> => {
    const r = await api.softDeleteResource(resource.id, resource.version);
    onChange(r.ok ? `Deleted ${resource.id}` : `delete failed: ${r.error}`);
  };

  return (
    <>
      <h3>Resources (POST/PATCH/DELETE /resources)</h3>
      <table data-testid="resources-table">
        <thead>
          <tr>
            <th>id</th>
            <th>name</th>
            <th>category</th>
            <th>min</th>
            <th>actions</th>
          </tr>
        </thead>
        <tbody>
          {resources.map((r) => (
            <tr key={r.id}>
              <td>
                <code>{r.id}</code>
              </td>
              <td>{r.name}</td>
              <td>{r.category}</td>
              <td>
                <TierTag tier={r.min_tier} />
              </td>
              <td>
                <select
                  value={r.min_tier}
                  aria-label={`Change min tier for ${r.name}`}
                  onChange={(e) => void changeMin(r, e.target.value as Tier)}
                >
                  {TIERS.map((t) => (
                    <option key={t} value={t}>
                      → {t}
                    </option>
                  ))}
                </select>
                <button onClick={() => void remove(r)} aria-label={`Delete resource ${r.name}`}>Delete</button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      <div className="row">
        <input placeholder="id" value={id} onChange={(e) => setId(e.target.value)} aria-label="Resource ID" />
        <input
          placeholder="name"
          value={name}
          onChange={(e) => setName(e.target.value)}
          aria-label="Resource name"
        />
        <input
          placeholder="category"
          value={category}
          onChange={(e) => setCategory(e.target.value)}
          aria-label="Resource category"
        />
        <select value={minTier} onChange={(e) => setMinTier(e.target.value as Tier)} aria-label="Minimum tier">
          {TIERS.map((t) => (
            <option key={t} value={t}>
              {t}
            </option>
          ))}
        </select>
        <button onClick={() => void create()} disabled={!id || !name} aria-label="Create resource">
          Create resource
        </button>
      </div>
    </>
  );
}

// ---------- Access -----------------------------------------------------

function AccessSection({
  passengers,
  resources,
  onResult,
}: {
  passengers: ApiPassenger[];
  resources: ApiResource[];
  onResult: (msg: string) => void;
}): JSX.Element {
  const [pid, setPid] = useState<string>("");
  const [rid, setRid] = useState<string>("");

  const validPid = useMemo<string>(
    () => passengers.find((p) => p.id === pid)?.id ?? passengers[0]?.id ?? "",
    [pid, passengers],
  );
  const validRid = useMemo<string>(
    () => resources.find((r) => r.id === rid)?.id ?? resources[0]?.id ?? "",
    [rid, resources],
  );

  const attempt = async (): Promise<void> => {
    if (!validPid || !validRid) return;
    // FIX: actor identity now derived from bearer token, not request body.
    // Temporarily set the token to the passenger ID (demo mode: token == id),
    // then restore the previous crew-lead token after the access attempt.
    const prevToken = api.getToken();
    api.setToken(validPid);
    const r = await api.useResource(validRid);
    api.setToken(prevToken);
    onResult(r.ok ? `Allowed (event #${r.value.id})` : r.error);
  };

  return (
    <>
      <h3>Access (POST /access)</h3>
      <div className="row">
        <select value={validPid} onChange={(e) => setPid(e.target.value)} aria-label="Select passenger">
          {passengers.map((p) => (
            <option key={p.id} value={p.id}>
              {p.name} ({p.tier})
            </option>
          ))}
        </select>
        <span className="muted" aria-hidden="true">→</span>
        <select value={validRid} onChange={(e) => setRid(e.target.value)} aria-label="Select resource">
          {resources.map((r) => (
            <option key={r.id} value={r.id}>
              {r.name} (min {r.min_tier})
            </option>
          ))}
        </select>
        <button onClick={() => void attempt()} disabled={!validPid || !validRid} data-testid="btn-attempt-access" aria-label="Attempt resource access">
          Attempt access
        </button>
      </div>
    </>
  );
}

// ---------- Reports ----------------------------------------------------

function ReportsSection({
  byTier,
  topResources,
  history,
  passengers,
  topN,
  setTopN,
  historyPid,
  setHistoryPid,
}: {
  byTier: ApiTierCount[];
  topResources: ApiTopResource[];
  history: ApiUsageEvent[];
  passengers: ApiPassenger[];
  topN: number;
  setTopN: (n: number) => void;
  historyPid: string;
  setHistoryPid: (pid: string) => void;
}): JSX.Element {
  return (
    <>
      <h3>Reports — aggregate by tier (GET /reports/by-tier)</h3>
      <table>
        <thead>
          <tr>
            <th>tier</th>
            <th>allowed</th>
            <th>denied</th>
          </tr>
        </thead>
        <tbody>
          {byTier.map((row) => (
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
          onChange={(e) => setTopN(Number(e.target.value) || 1)}
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
          {topResources.map((r) => (
            <tr key={r.resource_id}>
              <td>
                <code>{r.resource_id}</code>
              </td>
              <td>{r.allowed_count}</td>
            </tr>
          ))}
          {topResources.length === 0 && (
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
        <select value={historyPid} onChange={(e) => setHistoryPid(e.target.value)}>
          {passengers.map((p) => (
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
    </>
  );
}

// ---------- Usage + Audit ----------------------------------------------

function UsageSection({ usage }: { usage: ApiUsageEvent[] }): JSX.Element {
  return (
    <>
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
            {[...usage].reverse().map((e) => (
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
            {usage.length === 0 && (
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
  );
}

function AuditSection({ audit }: { audit: ApiAdminEvent[] }): JSX.Element {
  return (
    <>
      <h3>Audit log (GET /audit)</h3>
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
            {[...audit].reverse().map((e) => (
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
            {audit.length === 0 && (
              <tr>
                <td colSpan={5} className="muted">
                  no admin events yet
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </>
  );
}

function AccessibleSection(): JSX.Element {
  const [tier, setTier] = useState<Tier>("Silver");
  const [rows, setRows] = useState<ApiResource[] | null>(null);
  const [err, setErr] = useState<string | null>(null);

  const load = async (t: Tier): Promise<void> => {
    const r = await api.accessibleFor(t);
    if (r.ok) {
      setRows(r.value);
      setErr(null);
    } else {
      setErr(r.error);
      setRows(null);
    }
  };

  useEffect(() => {
    void load(tier);
  }, [tier]);

  return (
    <>
      <h4>Accessible resources for tier</h4>
      <div className="row">
        <select
          value={tier}
          onChange={(e) => setTier(e.target.value as Tier)}
          aria-label="Filter tier for accessible resources"
        >
          <option value="Silver">Silver</option>
          <option value="Gold">Gold</option>
          <option value="Diamond">Diamond</option>
          <option value="Platinum">Platinum</option>
        </select>
      </div>
      {err && <p className="error">{err}</p>}
      {rows && (
        <ul>
          {rows.map((r) => (
            <li key={r.id}>
              <code>{r.id}</code> — {r.name} (min{" "}
              <TierTag tier={r.min_tier} />)
            </li>
          ))}
          {rows.length === 0 && <li className="muted">none</li>}
        </ul>
      )}
    </>
  );
}
