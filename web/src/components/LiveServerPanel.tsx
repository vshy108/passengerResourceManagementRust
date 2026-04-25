import { useCallback, useEffect, useMemo, useState } from "react";
import {
  api,
  type ApiAdminEvent,
  type ApiCrewLead,
  type ApiPassenger,
  type ApiResource,
  type ApiTierCount,
  type ApiTopResource,
  type ApiUsageEvent,
} from "../services/api";
import type { Tier } from "../domain/tier";
import { TierTag } from "./TierTag";

type Status = "idle" | "checking" | "online" | "offline";

const TIERS: Tier[] = ["Silver", "Gold", "Platinum"];

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
        if (actorId === "" && cl.value[0]) setActorId(cl.value[0].id);
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
            Refresh all
          </button>
          {flash && <span className="muted">→ {flash}</span>}
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
              <label className="muted">Acting Crew Lead:</label>
              <select value={actorId} onChange={(e) => setActorId(e.target.value)}>
                {state.crewLeads.map((c) => (
                  <option key={c.id} value={c.id}>
                    {c.name} ({c.id})
                  </option>
                ))}
              </select>
            </div>

            <CrewLeadsSection
              crewLeads={state.crewLeads}
              actorId={actorId}
              onChange={(msg) => {
                announce(msg);
                void refresh();
              }}
            />

            <PassengersSection
              passengers={state.passengers}
              actorId={actorId}
              onChange={(msg) => {
                announce(msg);
                void refresh();
              }}
            />

            <ResourcesSection
              resources={state.resources}
              actorId={actorId}
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
  actorId,
  onChange,
}: {
  crewLeads: ApiCrewLead[];
  actorId: string;
  onChange: (msg: string) => void;
}): JSX.Element {
  const [oldId, setOldId] = useState<string>("");
  const [newId, setNewId] = useState<string>("");
  const [newName, setNewName] = useState<string>("");

  const submit = async (): Promise<void> => {
    if (!actorId || !oldId || !newId || !newName) return;
    const r = await api.replaceCrewLead(actorId, oldId, { id: newId, name: newName });
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
  actorId,
  onChange,
}: {
  passengers: ApiPassenger[];
  actorId: string;
  onChange: (msg: string) => void;
}): JSX.Element {
  const [id, setId] = useState<string>("");
  const [name, setName] = useState<string>("");
  const [tier, setTier] = useState<Tier>("Silver");

  const create = async (): Promise<void> => {
    if (!actorId || !id || !name) return;
    const r = await api.createPassenger(actorId, id, name, tier);
    onChange(r.ok ? `Created passenger ${id}` : `create failed: ${r.error}`);
    if (r.ok) {
      setId("");
      setName("");
    }
  };

  const changeTier = async (pid: string, t: Tier): Promise<void> => {
    if (!actorId) return;
    const r = await api.changePassengerTier(actorId, pid, t);
    onChange(r.ok ? `${pid} → ${t}` : `change failed: ${r.error}`);
  };

  const remove = async (pid: string): Promise<void> => {
    if (!actorId) return;
    const r = await api.softDeletePassenger(actorId, pid);
    onChange(r.ok ? `Deleted ${pid}` : `delete failed: ${r.error}`);
  };

  return (
    <>
      <h3>Passengers (POST/PATCH/DELETE /passengers)</h3>
      <table>
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
                  onChange={(e) => void changeTier(p.id, e.target.value as Tier)}
                >
                  {TIERS.map((t) => (
                    <option key={t} value={t}>
                      → {t}
                    </option>
                  ))}
                </select>
                <button onClick={() => void remove(p.id)}>Delete</button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      <div className="row">
        <input placeholder="id" value={id} onChange={(e) => setId(e.target.value)} />
        <input
          placeholder="name"
          value={name}
          onChange={(e) => setName(e.target.value)}
        />
        <select value={tier} onChange={(e) => setTier(e.target.value as Tier)}>
          {TIERS.map((t) => (
            <option key={t} value={t}>
              {t}
            </option>
          ))}
        </select>
        <button onClick={() => void create()} disabled={!id || !name}>
          Create passenger
        </button>
      </div>
    </>
  );
}

// ---------- Resources --------------------------------------------------

function ResourcesSection({
  resources,
  actorId,
  onChange,
}: {
  resources: ApiResource[];
  actorId: string;
  onChange: (msg: string) => void;
}): JSX.Element {
  const [id, setId] = useState<string>("");
  const [name, setName] = useState<string>("");
  const [category, setCategory] = useState<string>("general");
  const [minTier, setMinTier] = useState<Tier>("Silver");

  const create = async (): Promise<void> => {
    if (!actorId || !id || !name) return;
    const r = await api.createResource(actorId, id, name, category, minTier);
    onChange(r.ok ? `Created resource ${id}` : `create failed: ${r.error}`);
    if (r.ok) {
      setId("");
      setName("");
    }
  };

  const changeMin = async (rid: string, t: Tier): Promise<void> => {
    if (!actorId) return;
    const r = await api.changeResourceMinTier(actorId, rid, t);
    onChange(r.ok ? `${rid} min → ${t}` : `change failed: ${r.error}`);
  };

  const remove = async (rid: string): Promise<void> => {
    if (!actorId) return;
    const r = await api.softDeleteResource(actorId, rid);
    onChange(r.ok ? `Deleted ${rid}` : `delete failed: ${r.error}`);
  };

  return (
    <>
      <h3>Resources (POST/PATCH/DELETE /resources)</h3>
      <table>
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
                  onChange={(e) => void changeMin(r.id, e.target.value as Tier)}
                >
                  {TIERS.map((t) => (
                    <option key={t} value={t}>
                      → {t}
                    </option>
                  ))}
                </select>
                <button onClick={() => void remove(r.id)}>Delete</button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      <div className="row">
        <input placeholder="id" value={id} onChange={(e) => setId(e.target.value)} />
        <input
          placeholder="name"
          value={name}
          onChange={(e) => setName(e.target.value)}
        />
        <input
          placeholder="category"
          value={category}
          onChange={(e) => setCategory(e.target.value)}
        />
        <select value={minTier} onChange={(e) => setMinTier(e.target.value as Tier)}>
          {TIERS.map((t) => (
            <option key={t} value={t}>
              {t}
            </option>
          ))}
        </select>
        <button onClick={() => void create()} disabled={!id || !name}>
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
    const r = await api.useResource(validPid, validRid);
    onResult(r.ok ? `Allowed (event #${r.value.id})` : r.error);
  };

  return (
    <>
      <h3>Access (POST /access)</h3>
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
              {r.name} (min {r.min_tier})
            </option>
          ))}
        </select>
        <button onClick={() => void attempt()} disabled={!validPid || !validRid}>
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
