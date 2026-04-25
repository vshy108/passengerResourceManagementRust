import { useCallback, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import { buildWorld, type World } from "../services/world";
import { StoreContext, type StoreApi } from "./storeContext";

export function StoreProvider({ children }: { children: ReactNode }): JSX.Element {
  const worldRef = useRef<World>();
  if (!worldRef.current) worldRef.current = buildWorld();
  const [version, setVersion] = useState(0);

  const mutate = useCallback(<T,>(fn: (w: World) => T): T => {
    const result = fn(worldRef.current!);
    setVersion((v) => v + 1);
    return result;
  }, []);

  const value = useMemo<StoreApi>(
    () => ({ world: worldRef.current!, version, mutate }),
    [version, mutate],
  );
  return <StoreContext.Provider value={value}>{children}</StoreContext.Provider>;
}
