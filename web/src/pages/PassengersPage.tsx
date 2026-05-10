import { useState } from "react";
import { api, type Tier } from "../services/api";
import { useData } from "../contexts/DataContext";
import { TierTag } from "../components/TierTag";

const TIERS: Tier[] = ["Silver", "Gold", "Diamond", "Platinum"];

export function PassengersPage(): JSX.Element {
  const { state, refresh } = useData();
  const [flash, setFlash] = useState("");
  const [id, setId] = useState("");
  const [name, setName] = useState("");
  const [tier, setTier] = useState<Tier>("Silver");

  const announce = (msg: string): void => {
    setFlash(msg);
    setTimeout(() => setFlash(""), 4000);
  };

  const create = async (): Promise<void> => {
    if (!id || !name) return;
    const r = await api.createPassenger(id, name, tier);
    announce(r.ok ? `Created passenger ${id}` : `create failed: ${r.error}`);
    if (r.ok) {
      setId("");
      setName("");
      await refresh();
    }
  };

  const changeTier = async (pid: string, t: Tier): Promise<void> => {
    const r = await api.changePassengerTier(pid, t);
    announce(r.ok ? `${pid} → ${t}` : `change failed: ${r.error}`);
    if (r.ok) await refresh();
  };

  const remove = async (pid: string): Promise<void> => {
    const r = await api.softDeletePassenger(pid);
    announce(r.ok ? `Deleted ${pid}` : `delete failed: ${r.error}`);
    if (r.ok) await refresh();
  };

  return (
    <section className="page">
      <h2>Passengers</h2>
      {flash && <p className="muted">→ {flash}</p>}

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
          {state.passengers.map((p) => (
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
                  onChange={(e) =>
                    void changeTier(p.id, e.target.value as Tier)
                  }
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
        <input
          placeholder="id"
          value={id}
          onChange={(e) => setId(e.target.value)}
        />
        <input
          placeholder="name"
          value={name}
          onChange={(e) => setName(e.target.value)}
        />
        <select
          value={tier}
          onChange={(e) => setTier(e.target.value as Tier)}
        >
          {TIERS.map((t) => (
            <option key={t} value={t}>
              {t}
            </option>
          ))}
        </select>
        <button
          onClick={() => void create()}
          disabled={!id || !name}
          data-testid="btn-create-passenger"
        >
          Create passenger
        </button>
      </div>
    </section>
  );
}
