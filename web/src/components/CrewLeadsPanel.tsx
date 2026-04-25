import { useState } from "react";
import { crewLeadId } from "../domain/ids";
import { useStore } from "../state/useStore";
import type { DomainError } from "../domain/errors";

export function CrewLeadsPanel(): JSX.Element {
  const { world, mutate } = useStore();
  const [oldId, setOldId] = useState("");
  const [newId, setNewId] = useState("");
  const [newName, setNewName] = useState("");
  const [error, setError] = useState<DomainError | null>(null);

  const leads = world.crewLeads.list();
  const actor = leads[0]!.id;

  const submit = (): void => {
    if (!oldId || !newId || !newName) return;
    const result = mutate((w) =>
      w.crewLeads.replace(actor, crewLeadId(oldId), {
        id: crewLeadId(newId),
        name: newName,
      }),
    );
    if (!result.ok) {
      setError(result.error);
    } else {
      setError(null);
      setOldId("");
      setNewId("");
      setNewName("");
    }
  };

  return (
    <section className="panel">
      <header>
        <h2>Crew Leads (CL — exactly 3)</h2>
      </header>
      <div className="body">
        <table>
          <thead>
            <tr>
              <th>id</th>
              <th>name</th>
            </tr>
          </thead>
          <tbody>
            {leads.map((l) => (
              <tr key={l.id}>
                <td>
                  <code>{l.id}</code>
                </td>
                <td>{l.name}</td>
              </tr>
            ))}
          </tbody>
        </table>
        <p className="muted">
          CL-R2/R3 reject add &amp; remove unconditionally. Use replace to rotate a lead.
        </p>
        <div className="row">
          <select value={oldId} onChange={(e) => setOldId(e.target.value)}>
            <option value="">replace…</option>
            {leads.map((l) => (
              <option key={l.id} value={l.id}>
                {l.id}
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
          <button onClick={submit} disabled={!oldId || !newId || !newName}>
            Replace
          </button>
        </div>
        {error && <div className="error-banner">{error}</div>}
      </div>
    </section>
  );
}
