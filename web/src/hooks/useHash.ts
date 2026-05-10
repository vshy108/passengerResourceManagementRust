import { useEffect, useState } from "react";

/** Returns the current URL hash (e.g. "#/passengers"). Defaults to "#/" when empty. */
export function useHash(): string {
  const [hash, setHash] = useState(() => window.location.hash || "#/");
  useEffect(() => {
    const handler = (): void => setHash(window.location.hash || "#/");
    window.addEventListener("hashchange", handler);
    return () => window.removeEventListener("hashchange", handler);
  }, []);
  return hash;
}
