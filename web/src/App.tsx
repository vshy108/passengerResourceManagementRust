import { AccessPanel } from "./components/AccessPanel";
import { AuditLogPanel } from "./components/AuditLogPanel";
import { CrewLeadsPanel } from "./components/CrewLeadsPanel";
import { LiveServerPanel } from "./components/LiveServerPanel";
import { PassengersPanel } from "./components/PassengersPanel";
import { ReportsPanel } from "./components/ReportsPanel";
import { ResourcesPanel } from "./components/ResourcesPanel";
import { TierMatrix } from "./components/TierMatrix";

export function App(): JSX.Element {
  return (
    <>
      <header>
        <div>
          <h1>Spaceship X26 — Passenger Resource Management</h1>
          <p>
            Browser demo. The TypeScript services in <code>web/src/services/</code> mirror the
            Rust specs in <code>specs/</code> 1:1.
          </p>
        </div>
        <p>state lives in memory · refresh to reset</p>
      </header>
      <main>
        <CrewLeadsPanel />
        <section className="panel">
          <header>
            <h2>Tier policy matrix (TP)</h2>
          </header>
          <div className="body">
            <TierMatrix />
            <p className="muted">
              Higher rank inherits access to all lower-rank resources (TP-R2).
            </p>
          </div>
        </section>
        <PassengersPanel />
        <ResourcesPanel />
        <AccessPanel />
        <ReportsPanel />
        <AuditLogPanel />
        <LiveServerPanel />
      </main>
      <footer>
        See <code>AGENTS.md</code> and <code>specs/</code> for the canonical rules. Tests in
        the Rust crate keep this implementation honest.
      </footer>
    </>
  );
}
