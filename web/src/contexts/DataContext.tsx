import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
  type ReactNode,
} from "react";
import {
  api,
  type ApiAdminEvent,
  type ApiCrewLead,
  type ApiPassenger,
  type ApiResource,
  type ApiTierCount,
  type ApiTopResource,
  type ApiUsageEvent,
} from "../services/api";

type Status = "idle" | "checking" | "online" | "offline";

export interface LiveState {
  crewLeads: ApiCrewLead[];
  passengers: ApiPassenger[];
  resources: ApiResource[];
  usage: ApiUsageEvent[];
  audit: ApiAdminEvent[];
  byTier: ApiTierCount[];
  topResources: ApiTopResource[];
}

const EMPTY: LiveState = {
  crewLeads: [],
  passengers: [],
  resources: [],
  usage: [],
  audit: [],
  byTier: [],
  topResources: [],
};

interface DataCtx {
  status: Status;
  state: LiveState;
  topN: number;
  setTopN: (n: number) => void;
  refresh: () => Promise<void>;
  ping: () => Promise<void>;
}

const DataContext = createContext<DataCtx>({
  status: "idle",
  state: EMPTY,
  topN: 5,
  setTopN: () => {},
  refresh: async () => {},
  ping: async () => {},
});

export function DataProvider({ children }: { children: ReactNode }): JSX.Element {
  const [status, setStatus] = useState<Status>("idle");
  const [state, setState] = useState<LiveState>(EMPTY);
  const [topN, setTopN] = useState(5);
  // Ref keeps topN available inside the stable refresh() callback without
  // making it a dependency (which would recreate ping() on every change).
  // FIX: update the ref inside a useEffect rather than during render to
  // satisfy react-hooks/refs — refs must not be mutated in render phase.
  const topNRef = useRef(5);
  useEffect(() => {
    topNRef.current = topN;
  }, [topN]);

  const refresh = useCallback(async (): Promise<void> => {
    const [cl, pax, res, usage, audit, byTier, top] = await Promise.all([
      api.crewLeads(),
      api.passengers(),
      api.resources(),
      api.usage(),
      api.audit(),
      api.byTier(),
      api.topResources(topNRef.current),
    ]);
    if (
      cl.ok &&
      pax.ok &&
      res.ok &&
      usage.ok &&
      audit.ok &&
      byTier.ok &&
      top.ok
    ) {
      setState({
        crewLeads: cl.value,
        passengers: pax.value,
        resources: res.value,
        usage: usage.value,
        audit: audit.value,
        byTier: byTier.value,
        topResources: top.value,
      });
    }
  }, []); // stable — topN is read via ref

  const ping = useCallback(async (): Promise<void> => {
    setStatus("checking");
    const h = await api.health();
    if (h.ok) {
      setStatus("online");
      await refresh();
    } else {
      setStatus("offline");
    }
  }, [refresh]);

  useEffect(() => {
    void ping();
  }, [ping]);

  return (
    <DataContext.Provider value={{ status, state, topN, setTopN, refresh, ping }}>
      {children}
    </DataContext.Provider>
  );
}

// eslint-disable-next-line react-refresh/only-export-components
export const useData = (): DataCtx => useContext(DataContext);
