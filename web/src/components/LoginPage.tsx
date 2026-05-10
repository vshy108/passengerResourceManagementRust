import { useState, type JSX } from "react";
import { api } from "../services/api";
import { useAuth } from "../contexts/AuthContext";

export function LoginPage(): JSX.Element {
  const { login } = useAuth();
  const [token, setToken] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [checking, setChecking] = useState(false);

  const submit = async (): Promise<void> => {
    const t = token.trim();
    if (!t) return;
    setChecking(true);
    setError(null);
    // Temporarily apply the token so health() sends it.
    api.setToken(t);
    const h = await api.health();
    if (h.ok) {
      login(t);
      window.location.hash = "#/";
    } else {
      // Revert — health failed, token may be wrong or server is down.
      api.setToken(null);
      setError(
        "Could not connect to the server. Check that it is running and the token is valid.",
      );
    }
    setChecking(false);
  };

  return (
    <div className="login-container">
      <div className="login-card">
        <h1>Spaceship X26</h1>
        <p className="muted login-subtitle">Passenger Resource Management System</p>
        <div className="login-form">
          <label htmlFor="token-input" className="login-label">
            API Token
          </label>
          <input
            id="token-input"
            type="text"
            placeholder="e.g. cl-aria"
            value={token}
            onChange={(e) => setToken(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") void submit();
            }}
            autoFocus
          />
          {error && <p className="error-banner">{error}</p>}
          <button
            onClick={() => void submit()}
            disabled={!token.trim() || checking}
            className="login-btn"
          >
            {checking ? "Connecting…" : "Connect"}
          </button>
        </div>
        <p className="muted login-hint">
          Demo tokens: <code>cl-aria</code> (crew lead) · <code>ps-001</code>{" "}
          (passenger)
        </p>
      </div>
    </div>
  );
}
