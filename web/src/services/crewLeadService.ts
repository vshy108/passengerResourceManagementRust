import { err, ok, type Result } from "../domain/errors";
import type { CrewLeadId } from "../domain/ids";
import type { AdminEvent, CrewLead } from "../domain/types";
import type { ManualClock } from "./clock";

// Mirrors application/crew_lead_service.rs (CL-R1..R6).
export class CrewLeadService {
  private leads: CrewLead[] = [];
  private nextAuditId = 1;
  private constructor(
    private readonly clock: ManualClock,
    private readonly emit: (e: AdminEvent) => void,
  ) {}

  // CL-R1 — exactly 3 distinct leads. Emits CrewLeadBootstrapped on success.
  static bootstrap(
    leads: CrewLead[],
    clock: ManualClock,
    emit: (e: AdminEvent) => void,
  ): Result<CrewLeadService> {
    if (leads.length !== 3) return err("CrewLeadBootstrapInvalid");
    for (let i = 0; i < leads.length; i++) {
      for (let j = i + 1; j < leads.length; j++) {
        if (leads[i]!.id === leads[j]!.id) return err("CrewLeadBootstrapInvalid");
      }
    }
    const svc = new CrewLeadService(clock, emit);
    svc.leads = [...leads];
    const actorId = svc.leads[0]!.id;
    svc.emit({
      id: svc.nextAuditId++,
      actorId,
      action: "CrewLeadBootstrapped",
      targetKind: "CrewLead",
      targetId: actorId,
      timestamp: clock.now(),
      details: `count=${svc.leads.length}`,
    });
    return ok(svc);
  }

  // CL-R2 — always rejected (cap = 3).
  add(_lead: CrewLead): Result<void> {
    return err("CrewLeadLimitReached");
  }
  // CL-R3 — always rejected (would breach minimum).
  remove(_id: CrewLeadId): Result<void> {
    return err("CrewLeadMinimumBreached");
  }

  // CL-R4 — atomic swap. Audited.
  replace(actorId: CrewLeadId, oldId: CrewLeadId, newLead: CrewLead): Result<void> {
    const slot = this.leads.findIndex((l) => l.id === oldId);
    if (slot === -1) return err("CrewLeadNotFound");
    if (this.leads.some((l, i) => i !== slot && l.id === newLead.id)) {
      return err("CrewLeadAlreadyExists");
    }
    this.leads[slot] = newLead;
    this.emit({
      id: this.nextAuditId++,
      actorId,
      action: "CrewLeadReplaced",
      targetKind: "CrewLead",
      targetId: newLead.id,
      timestamp: this.clock.now(),
      details: `replaced=${oldId}`,
    });
    return ok(undefined);
  }

  // CL-R6 — current Crew Leads (insertion order).
  list(): readonly CrewLead[] {
    return this.leads;
  }
}
