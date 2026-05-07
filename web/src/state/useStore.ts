import { useContext } from "react";
import { StoreContext, type StoreApi } from "./storeContext";

// Convenience hook so components don't need to import StoreContext
// and call useContext themselves. The explicit null-check ensures a
// clear error message if the component tree is missing a StoreProvider.
export function useStore(): StoreApi {
  const v = useContext(StoreContext);
  // Throwing rather than returning null forces the bug to surface
  // immediately at the component that forgot its provider wrapper,
  // rather than silently causing downstream undefined-access errors.
  if (!v) throw new Error("useStore must be used inside StoreProvider");
  return v;
}
