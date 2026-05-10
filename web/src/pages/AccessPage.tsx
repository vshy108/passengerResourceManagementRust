import { useMemo, useState } from "react";
import { api } from "../services/api";
import { useData } from "../contexts/DataContext";

export function AccessPage(): JSX.Element {
  const { state, refresh } = useData();
  const [flash, setFlash] = useState("");
  const [pid, setPid] = useState("");
  const [rid, setRid] = useState("");

  const announce = (msg: string): void => {
    setFlash(msg);
    setTimeout(() => setFlash(""), 4000);
  };

  const validPid = useMemo(
    () =>
      state.passengers.find((p) => p.id === pid)?.id ??
      state.passengers[0]?.id ??
      "",
    [pid, state.passengers],
  );
  const validRid = useMemo(
    () =>
      state.resources.find((r) => r.id === rid)?.id ??
      state.resources[0]?.id ??
      "",
    [rid, state.resources],
  );

  const attempt = async (): Promise<void> => {
    if (!validPid || !validRid) return;
    // FIX: actor identity derived from bearer token, not request body.
    // Temporarily swap to the passenger's token (demo: token == passenger id),
    // then restore the previous token after the access attempt completes.
    const prevToken = api.getToken();
    api.setToken(validPid);
    const r = await api.useResource(validRid);
    api.setToken(prevToken);
    announce(r.ok ? `Allowed (event #${r.value.id})` : r.error);
    await refresh();
  };

  return (
    <section className="page">
      <h2>Access Check</h2>
      {flash && <p className="muted">→ {flash}</p>}

      <h3>Access (POST /access)</h3>
      <div className="row">
        <select
          value={validPid}
          onChange={(e) => setPid(e.target.value)}
          data-testid="access-passenger-select"
        >
          {state.passengers.map((p) => (
            <option key={p.id} value={p.id}>
              {p.name} ({p.tier})
            </option>
          ))}
        </select>
        <span className="muted">→</span>
        <select
          value={validRid}
          onChange={(e) => setRid(e.target.value)}
          data-testid="access-resource-select"
        >
          {state.resources.map((r) => (
            <option key={r.id} value={r.id}>
              {r.name} (min {r.min_tier})
            </option>
          ))}
        </select>
        <button
          onClick={() => void attempt()}
          disabled={!validPid || !validRid}
          data-testid="btn-attempt-access"
        >
          Attempt access
        </button>
      </div>
    </section>
  );
}
