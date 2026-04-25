import { createContext, useCallback, useContext, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import { buildWorld, type World } from "../services/world";

interface StoreApi {
  world: World;
  version: number;
  mutate: <T>(fn: (w: World) => T) => T;
}

const Ctx = createContext<StoreApi | null>(null);

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
  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
}

export function useStore(): StoreApi {
  const v = useContext(Ctx);
  if (!v) throw new Error("useStore must be used inside StoreProvider");
  return v;
}
