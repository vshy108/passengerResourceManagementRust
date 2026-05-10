import { type JSX } from "react";
import { useAuth } from "../contexts/AuthContext";
import { useData } from "../contexts/DataContext";
import { useHash } from "../hooks/useHash";
import { DashboardPage } from "../pages/DashboardPage";
import { CrewLeadsPage } from "../pages/CrewLeadsPage";
import { PassengersPage } from "../pages/PassengersPage";
import { ResourcesPage } from "../pages/ResourcesPage";
import { AccessPage } from "../pages/AccessPage";
import { ReportsPage } from "../pages/ReportsPage";
import { AuditPage } from "../pages/AuditPage";

const NAV_LINKS = [
  { path: "#/", label: "Dashboard" },
  { path: "#/crew-leads", label: "Crew Leads" },
  { path: "#/passengers", label: "Passengers" },
  { path: "#/resources", label: "Resources" },
  { path: "#/access", label: "Access" },
  { path: "#/reports", label: "Reports" },
  { path: "#/audit", label: "Audit" },
] as const;

function Breadcrumb({ hash }: { hash: string }): JSX.Element {
  const found = NAV_LINKS.find((l) => l.path === hash);
  const label = found?.label ?? "Dashboard";
  if (label === "Dashboard") {
    return (
      <nav className="breadcrumb" aria-label="breadcrumb">
        <span>Home</span>
      </nav>
    );
  }
  return (
    <nav className="breadcrumb" aria-label="breadcrumb">
      <a href="#/">Home</a>
      <span className="breadcrumb-sep" aria-hidden>›</span>
      <span>{label}</span>
    </nav>
  );
}

export function AppShell(): JSX.Element {
  const { token, logout } = useAuth();
  const { status, ping, refresh } = useData();
  const hash = useHash();

  let page: JSX.Element;
  if (hash === "#/crew-leads") page = <CrewLeadsPage />;
  else if (hash === "#/passengers") page = <PassengersPage />;
  else if (hash === "#/resources") page = <ResourcesPage />;
  else if (hash === "#/access") page = <AccessPage />;
  else if (hash === "#/reports") page = <ReportsPage />;
  else if (hash === "#/audit") page = <AuditPage />;
  else page = <DashboardPage />;

  return (
    <>
      <header>
        <div className="header-left">
          <h1>Spaceship X26 — PRMS</h1>
          <nav className="nav-links" aria-label="main navigation">
            {NAV_LINKS.map((l) => (
              <a
                key={l.path}
                href={l.path}
                className={`nav-link${hash === l.path ? " active" : ""}`}
              >
                {l.label}
              </a>
            ))}
          </nav>
        </div>
        <div className="header-right">
          <span
            className={`tag ${status === "online" ? "allowed" : "denied"}`}
            data-testid="server-status"
          >
            {status.toUpperCase()}
          </span>
          <button onClick={() => void ping()} data-testid="btn-recheck">
            Re-check
          </button>
          <button
            onClick={() => void refresh()}
            disabled={status !== "online"}
            data-testid="btn-refresh"
          >
            Refresh
          </button>
          <span className="muted header-token">{token}</span>
          <button onClick={logout} className="btn-logout">
            Logout
          </button>
        </div>
      </header>
      <div className="breadcrumb-bar">
        <Breadcrumb hash={hash} />
      </div>
      <main data-testid="live-panel">
        {page}
      </main>
      <footer>
        See <code>AGENTS.md</code> and <code>specs/</code> for the canonical rules.
      </footer>
    </>
  );
}
