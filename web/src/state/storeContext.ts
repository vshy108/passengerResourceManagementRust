import { createContext } from "react";
import type { World } from "../services/world";

export interface StoreApi {
  world: World;
  version: number;
  mutate: <T>(fn: (w: World) => T) => T;
}

export const StoreContext = createContext<StoreApi | null>(null);
