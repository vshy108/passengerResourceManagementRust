import { createContext } from "react";
import type { World } from "../services/world";

// The shape of the value exposed by StoreProvider through React context.
// Components read world state and trigger mutations through this API.
export interface StoreApi {
  // The single shared in-memory World (all services + seed data).
  // Read by every panel to derive their display state.
  world: World;
  // Bumped by 1 on every mutation so React sees a new value and
  // re-renders all components that call useStore(). Without this,
  // mutating the world in place would be invisible to React (objects
  // are compared by reference, not value).
  version: number;
  // Wrap any World mutation: applies `fn`, bumps `version`, and
  // returns the result. This is the ONLY way components should
  // write to the world — it guarantees re-renders happen.
  mutate: <T>(fn: (w: World) => T) => T;
}

// `null` as default forces a runtime error if a component is rendered
// outside a StoreProvider — better than silently using stale data.
export const StoreContext = createContext<StoreApi | null>(null);
