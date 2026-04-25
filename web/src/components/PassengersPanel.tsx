import { useState } from "react";
import { asCrewLead } from "../domain/actor";
import type { DomainError } from "../domain/errors";
import { passengerId } from "../domain/ids";
import { ALL_TIERS, type Tier } from "../domain/tier";
import { useStore } from "../state/useStore";
import { TierTag } from "./TierTag";

export function PassengersPanel(): JSX.Element {
  const { world, mutate } = useStore();
  const [id, setId] = useState("");
  const [name, setName] = useState("");
  const [tier, setTier] = useState<Tier>("Silver");
  const [error, setError] = useState<DomainError | null>(null);

  const actor = asCrewLead(world.crewLeads.list()[0]!.id);
  const list = world.passengers.list();

  const create = (): void => {
    if (!id || !name) return;
    const r = mutate((w) => w.passengers.create(actor, passengerId(id), name, tier));
    if (!r.ok) setError(r.error);
    else {
      setError(null);
      setId("");
      setName("");
    }
  };

  return (
    <section className="panel">
      <header>
        <h2>Passengers (PS)</h2>
      </header>
      <div className="body">
        <div className="row">
          <input placeholder="id" value={id} onChange={(e) => setId(e.target.value)} />
          <input placeholder="name" value={name} onChange={(e) => setName(e.target.value)} />
          <select value={tier} onChange={(e) => setTier(e.target.value as Tier)}>
            {ALL_TIERS.map((t) => (
              <option key={t} value={t}>
                {t}
              </option>
            ))}
          </select>
          <button onClick={create} disabled={!id || !name}>
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
                <th>tier</th>
                <th />
              </tr>
            </thead>
            <tbody>
              {list.map((p) => (
                <tr key={p.id}>
                  <td>
                    <code>{p.id}</code>
                  </td>
                  <td>{p.name}</td>
                  <td>
                    <select
                      value={p.tier}
                      onChange={(e) =>
                        mutate((w) =>
                          w.passengers.changeTier(actor, p.id, e.target.value as Tier),
                        )
                      }
                    >
                      {ALL_TIERS.map((t) => (
                        <option key={t} value={t}>
                          {t}
                        </option>
                      ))}
                    </select>{" "}
                    <TierTag tier={p.tier} />
                  </td>
                  <td>
                    <button
                      onClick={() => mutate((w) => w.passengers.softDelete(actor, p.id))}
                    >
                      Soft delete
                    </button>
                  </td>
                </tr>
              ))}
              {list.length === 0 && (
                <tr>
                  <td colSpan={4} className="muted">
                    no active passengers
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
