import type { Actor } from "../domain/actor";
import { err, ok, type Result } from "../domain/errors";
import type { PassengerId, ResourceId } from "../domain/ids";
import { canAccess } from "../domain/tier";
import type { UsageEvent } from "../domain/types";
import type { ManualClock } from "./clock";
import type { PassengerService } from "./passengerService";
import type { ResourceService } from "./resourceService";

// Mirrors application/access_service.rs (AC-R1..R7).
//
// AC-R5 — every access attempt emits a UsageEvent (allowed OR denied).
// AC-R6 — the actor must be the same passenger as the subject.
export class AccessService {
  private events: UsageEvent[] = [];
  private nextId = 1;
  constructor(
    private readonly clock: ManualClock,
    private readonly passengers: PassengerService,
    private readonly resources: ResourceService,
  ) {}

  useResource(
    actor: Actor,
    passengerId: PassengerId,
    resourceId: ResourceId,
  ): Result<UsageEvent> {
    // AC-R6 — passenger acting on behalf of themselves only.
    if (actor.kind !== "Passenger" || actor.id !== passengerId) {
      return err("UnauthorizedActor");
    }
    const passenger = this.passengers.get(passengerId);
    if (!passenger.ok) return passenger;
    if (passenger.value.deletedAt !== null) return err("PassengerNotFound");

    const resource = this.resources.get(resourceId);
    if (!resource.ok) return resource;
    if (resource.value.deletedAt !== null) return err("ResourceNotFound");

    const allowed = canAccess(passenger.value.tier, resource.value.minTier);
    const event: UsageEvent = {
      id: this.nextId++,
      passengerId,
      passengerTier: passenger.value.tier,
      resourceId,
      resourceMinTier: resource.value.minTier,
      allowed,
      timestamp: this.clock.now(),
    };
    this.events.push(event);
    return ok(event);
  }

  history(): readonly UsageEvent[] {
    return this.events;
  }
}
