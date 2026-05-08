import { LiveServerPanel } from "./components/LiveServerPanel";

// Root layout. All content is fetched from the Rust backend (axum server at localhost:8080).
export function App(): JSX.Element {
  return (
    <>
      <header>
        <h1>Spaceship X26 — Passenger Resource Management</h1>
      </header>
      <main>
        <LiveServerPanel />
      </main>
      <footer>
        See <code>AGENTS.md</code> and <code>specs/</code> for the canonical rules. Tests in
        the Rust crate keep this implementation honest.
      </footer>
    </>
  );
}
