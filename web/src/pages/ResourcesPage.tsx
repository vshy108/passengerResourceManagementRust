import { useEffect, useState, type JSX } from "react";
import { api, type ApiResource, type Tier } from "../services/api";
import { useData } from "../contexts/DataContext";
import { TierTag } from "../components/TierTag";

const TIERS: Tier[] = ["Silver", "Gold", "Diamond", "Platinum"];

export function ResourcesPage(): JSX.Element {
  const { state, refresh } = useData();
  const [flash, setFlash] = useState("");
  const [id, setId] = useState("");
  const [name, setName] = useState("");
  const [category, setCategory] = useState("general");
  const [minTier, setMinTier] = useState<Tier>("Silver");

  const [checkTier, setCheckTier] = useState<Tier>("Silver");
  const [accessible, setAccessible] = useState<ApiResource[] | null>(null);

  const announce = (msg: string): void => {
    setFlash(msg);
    setTimeout(() => setFlash(""), 4000);
  };

  const create = async (): Promise<void> => {
    if (!id || !name) return;
    const r = await api.createResource(id, name, category, minTier);
    announce(r.ok ? `Created resource ${id}` : `create failed: ${r.error}`);
    if (r.ok) {
      setId("");
      setName("");
      await refresh();
    }
  };

  const changeMin = async (resource: ApiResource, t: Tier): Promise<void> => {
    const r = await api.changeResourceMinTier(resource.id, t, resource.version);
    announce(r.ok ? `${resource.id} min → ${t}` : `change failed: ${r.error}`);
    if (r.ok) await refresh();
  };

  const remove = async (resource: ApiResource): Promise<void> => {
    const r = await api.softDeleteResource(resource.id, resource.version);
    announce(r.ok ? `Deleted ${resource.id}` : `delete failed: ${r.error}`);
    if (r.ok) await refresh();
  };

  // Refresh accessible list when tier filter or resources change.
  useEffect(() => {
    void api.accessibleFor(checkTier).then((r) => {
      if (r.ok) setAccessible(r.value);
    });
  }, [checkTier, state.resources]);

  return (
    <section className="page">
      <h2>Resources</h2>
      {flash && <p className="muted">→ {flash}</p>}

      <table data-testid="resources-table">
        <thead>
          <tr>
            <th>id</th>
            <th>name</th>
            <th>category</th>
            <th>min tier</th>
            <th>actions</th>
          </tr>
        </thead>
        <tbody>
          {state.resources.map((r) => (
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
                  onChange={(e) =>
                    void changeMin(r, e.target.value as Tier)
                  }
                >
                  {TIERS.map((t) => (
                    <option key={t} value={t}>
                      → {t}
                    </option>
                  ))}
                </select>
                <button onClick={() => void remove(r)}>Delete</button>
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
        <input
          placeholder="category"
          value={category}
          onChange={(e) => setCategory(e.target.value)}
        />
        <select
          value={minTier}
          onChange={(e) => setMinTier(e.target.value as Tier)}
        >
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

      <h3>Accessible resources for tier</h3>
      <div className="row">
        <select
          value={checkTier}
          onChange={(e) => setCheckTier(e.target.value as Tier)}
        >
          {TIERS.map((t) => (
            <option key={t} value={t}>
              {t}
            </option>
          ))}
        </select>
      </div>
      {accessible !== null && (
        <ul>
          {accessible.map((r) => (
            <li key={r.id}>
              <code>{r.id}</code> — {r.name} (min <TierTag tier={r.min_tier} />
              )
            </li>
          ))}
          {accessible.length === 0 && (
            <li className="muted">none accessible for this tier</li>
          )}
        </ul>
      )}
    </section>
  );
}
