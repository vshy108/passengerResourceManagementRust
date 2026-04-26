import { useCallback, useMemo, useState } from "react";
import type { ReactNode } from "react";
import { buildWorld, type World } from "../services/world";
import { StoreContext, type StoreApi } from "./storeContext";

export function StoreProvider({ children }: { children: ReactNode }): JSX.Element {
  // Lazy-initialise the world once. The reference is stable across renders;
  // we mutate it in place and bump `version` to trigger re-renders. Using
  // useState (instead of useRef) keeps render side-effect-free, which the
  // react-hooks/exhaustive-deps + react-hooks rules in v7 enforce.
  const [world] = useState<World>(() => buildWorld());
  const [version, setVersion] = useState(0);

  const mutate = useCallback(<T,>(fn: (w: World) => T): T => {
    const result = fn(world);
    setVersion((v) => v + 1);
    return result;
  }, [world]);

  const value = useMemo<StoreApi>(
    () => ({ world, version, mutate }),
    [world, version, mutate],
  );
  return <StoreContext.Provider value={value}>{children}</StoreContext.Provider>;
}
