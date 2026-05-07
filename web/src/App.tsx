import { AccessPanel } from "./components/AccessPanel";
import { AuditLogPanel } from "./components/AuditLogPanel";
import { CrewLeadsPanel } from "./components/CrewLeadsPanel";
import { LiveServerPanel } from "./components/LiveServerPanel";
import { PassengersPanel } from "./components/PassengersPanel";
import { ReportsPanel } from "./components/ReportsPanel";
import { ResourcesPanel } from "./components/ResourcesPanel";
import { TierMatrix } from "./components/TierMatrix";

// Root layout. Two independent worlds run side-by-side:
//
// 1. In-browser TypeScript world (CrewLeads/Passengers/Resources/Access
//    /Reports/Audit panels) — state lives in StoreProvider (see
//    state/store.tsx), resets on page refresh, zero network calls.
//
// 2. Live Rust HTTP world (LiveServerPanel) — talks to the axum server
//    at localhost:8080. Mutations in one world are NOT reflected in
//    the other; they exist to prove the two implementations agree on
//    the same spec, not to share state.
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
        {/* Spec: 02-crew-lead.md */}
        <CrewLeadsPanel />
        {/* Static read-only table derived entirely from Tier.canAccess — no service call needed */}
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
        {/* Spec: 03-passenger.md */}
        <PassengersPanel />
        {/* Spec: 04-resource.md */}
        <ResourcesPanel />
        {/* Spec: 05-access.md — emits a UsageEvent on every attempt */}
        <AccessPanel />
        {/* Spec: 07-reporting.md */}
        <ReportsPanel />
        {/* Spec: 06-audit.md — shows AdminEvents from all crew-lead mutations */}
        <AuditLogPanel />
        <p className="muted">
          The panels above run a TypeScript port of the rules entirely in the
          browser. The panel below talks to the Rust <code>serve</code> binary
          over HTTP — the two keep <em>independent</em> state, so changes in
          one will not show up in the other.
        </p>
        {/* HTTP client panel — only functional when `cargo run --features http --bin serve` is running */}
        <LiveServerPanel />
      </main>
      <footer>
        See <code>AGENTS.md</code> and <code>specs/</code> for the canonical rules. Tests in
        the Rust crate keep this implementation honest.
      </footer>
    </>
  );
}
