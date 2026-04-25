import { useStore } from "../state/useStore";

export function AuditLogPanel(): JSX.Element {
  const { world } = useStore();
  const events = world.adminEvents;
  return (
    <section className="panel">
      <header>
        <h2>Admin audit log (AU)</h2>
      </header>
      <div className="body">
        <div className="scroll">
          <table>
            <thead>
              <tr>
                <th>#</th>
                <th>t</th>
                <th>actor</th>
                <th>action</th>
                <th>target</th>
                <th>details</th>
              </tr>
            </thead>
            <tbody>
              {[...events].reverse().map((e) => (
                <tr key={e.id}>
                  <td>{e.id}</td>
                  <td>{e.timestamp}</td>
                  <td>
                    <code>{e.actorId}</code>
                  </td>
                  <td>{e.action}</td>
                  <td>
                    <code>
                      {e.targetKind}:{e.targetId}
                    </code>
                  </td>
                  <td className="muted">{e.details ?? ""}</td>
                </tr>
              ))}
              {events.length === 0 && (
                <tr>
                  <td colSpan={6} className="muted">
                    no admin events
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
