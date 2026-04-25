import { useState } from "react";
import { asCrewLead } from "../domain/actor";
import type { DomainError } from "../domain/errors";
import { resourceId } from "../domain/ids";
import { ALL_TIERS, type Tier } from "../domain/tier";
import { useStore } from "../state/useStore";
import { TierTag } from "./TierTag";

export function ResourcesPanel(): JSX.Element {
  const { world, mutate } = useStore();
  const [id, setId] = useState("");
  const [name, setName] = useState("");
  const [category, setCategory] = useState("");
  const [minTier, setMinTier] = useState<Tier>("Silver");
  const [error, setError] = useState<DomainError | null>(null);

  const actor = asCrewLead(world.crewLeads.list()[0]!.id);
  const list = world.resources.list();

  const create = (): void => {
    if (!id || !name || !category) return;
    const r = mutate((w) =>
      w.resources.create(actor, resourceId(id), name, category, minTier),
    );
    if (!r.ok) setError(r.error);
    else {
      setError(null);
      setId("");
      setName("");
      setCategory("");
    }
  };

  return (
    <section className="panel">
      <header>
        <h2>Resources (RS)</h2>
      </header>
      <div className="body">
        <div className="row">
          <input placeholder="id" value={id} onChange={(e) => setId(e.target.value)} />
          <input placeholder="name" value={name} onChange={(e) => setName(e.target.value)} />
          <input
            placeholder="category"
            value={category}
            onChange={(e) => setCategory(e.target.value)}
          />
          <select value={minTier} onChange={(e) => setMinTier(e.target.value as Tier)}>
            {ALL_TIERS.map((t) => (
              <option key={t} value={t}>
                min: {t}
              </option>
            ))}
          </select>
          <button onClick={create} disabled={!id || !name || !category}>
            Create
          </button>
        </div>
        {error && <div className="error-banner">{error}</div>}
        <div className="scroll">
          <table>
            <thead>
              <tr>
                <th>id</th>
                <th>name</th>
                <th>category</th>
                <th>min tier</th>
                <th />
              </tr>
            </thead>
            <tbody>
              {list.map((r) => (
                <tr key={r.id}>
                  <td>
                    <code>{r.id}</code>
                  </td>
                  <td>{r.name}</td>
                  <td>{r.category}</td>
                  <td>
                    <select
                      value={r.minTier}
                      onChange={(e) =>
                        mutate((w) =>
                          w.resources.changeMinTier(actor, r.id, e.target.value as Tier),
                        )
                      }
                    >
                      {ALL_TIERS.map((t) => (
                        <option key={t} value={t}>
                          {t}
                        </option>
                      ))}
                    </select>{" "}
                    <TierTag tier={r.minTier} />
                  </td>
                  <td>
                    <button
                      onClick={() => mutate((w) => w.resources.softDelete(actor, r.id))}
                    >
                      Soft delete
                    </button>
                  </td>
                </tr>
              ))}
              {list.length === 0 && (
                <tr>
                  <td colSpan={5} className="muted">
                    no active resources
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
