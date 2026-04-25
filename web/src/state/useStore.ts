import { useContext } from "react";
import { StoreContext, type StoreApi } from "./storeContext";

export function useStore(): StoreApi {
  const v = useContext(StoreContext);
  if (!v) throw new Error("useStore must be used inside StoreProvider");
  return v;
}
