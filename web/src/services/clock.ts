import type { Timestamp } from "../domain/types";

// Manual clock — mirrors infrastructure/fake_clock.rs. The UI advances
// time on each mutation so timestamps strictly increase.
export class ManualClock {
  private current: Timestamp = 0;
  now(): Timestamp {
    this.current += 1;
    return this.current;
  }
  peek(): Timestamp {
    return this.current;
  }
}
