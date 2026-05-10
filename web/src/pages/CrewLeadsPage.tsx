import { useState } from "react";
import { api } from "../services/api";
import { useData } from "../contexts/DataContext";

export function CrewLeadsPage(): JSX.Element {
  const { state, refresh } = useData();
  const [flash, setFlash] = useState("");

  const [oldId, setOldId] = useState("");
  const [newId, setNewId] = useState("");
  const [newName, setNewName] = useState("");

  const [addId, setAddId] = useState("");
  const [addName, setAddName] = useState("");

  const announce = (msg: string): void => {
    setFlash(msg);
    setTimeout(() => setFlash(""), 4000);
  };

  const add = async (): Promise<void> => {
    if (!addId || !addName) return;
    const r = await api.addCrewLead({ id: addId, name: addName });
    announce(r.ok ? `Added ${addId}` : `add failed: ${r.error}`);
    if (r.ok) {
      setAddId("");
      setAddName("");
      await refresh();
    }
  };

  const replace = async (): Promise<void> => {
    if (!oldId || !newId || !newName) return;
    const r = await api.replaceCrewLead(oldId, { id: newId, name: newName });
    announce(r.ok ? `Replaced ${oldId} → ${newId}` : `replace failed: ${r.error}`);
    if (r.ok) {
      setOldId("");
      setNewId("");
      setNewName("");
      await refresh();
    }
  };

  const remove = async (id: string): Promise<void> => {
    const r = await api.removeCrewLead(id);
    announce(r.ok ? `Removed ${id}` : `remove failed: ${r.error}`);
    if (r.ok) await refresh();
  };

  return (
    <section className="page">
      <h2>Crew Leads</h2>
      {flash && <p className="muted">→ {flash}</p>}

      <table>
        <thead>
          <tr>
            <th>id</th>
            <th>name</th>
            <th>actions</th>
          </tr>
        </thead>
        <tbody>
          {state.crewLeads.map((c) => (
            <tr key={c.id}>
              <td>
                <code>{c.id}</code>
              </td>
              <td>{c.name}</td>
              <td>
                <button onClick={() => void remove(c.id)}>Remove</button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>

      <h3>Add crew lead</h3>
      <div className="row">
        <input
          placeholder="id"
          value={addId}
          onChange={(e) => setAddId(e.target.value)}
        />
        <input
          placeholder="name"
          value={addName}
          onChange={(e) => setAddName(e.target.value)}
        />
        <button onClick={() => void add()} disabled={!addId || !addName}>
          Add
        </button>
      </div>

      <h3>Replace crew lead</h3>
      <div className="row">
        <select value={oldId} onChange={(e) => setOldId(e.target.value)}>
          <option value="">replace…</option>
          {state.crewLeads.map((c) => (
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
        <button
          onClick={() => void replace()}
          disabled={!oldId || !newId || !newName}
        >
          Replace
        </button>
      </div>
    </section>
  );
}
