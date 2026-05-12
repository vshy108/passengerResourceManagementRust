// Thin client for the Rust axum server (src/interface/http.rs).
//
// Configure the base URL via the Vite env var `VITE_API_BASE`. When
// unset, requests go to `/api/*` and are proxied by Vite (see
// vite.config.ts) to the local serve binary at 127.0.0.1:8080. Set
// `VITE_API_BASE` to point at a remote server.

// All API types are derived from the auto-generated OpenAPI schema.
// Re-run `npm run generate:types` to regenerate after changing the Rust DTOs.
import type { components } from "./openapi.generated";

export type ApiCrewLead    = components["schemas"]["CrewLeadDto"];
export type ApiPassenger   = components["schemas"]["PassengerDto"];
export type ApiResource    = components["schemas"]["ResourceDto"];
export type ApiUsageEvent  = components["schemas"]["UsageEventDto"];
export type ApiAdminEvent  = components["schemas"]["AdminEventDto"];
export type ApiTierCount   = components["schemas"]["TierCountsDto"];
export type ApiTopResource = components["schemas"]["TopResourceDto"];
export type Tier           = components["schemas"]["TierDto"];

// FIX: ErrorCode is now generated from the Rust enum so TypeScript has a
// closed union type instead of `string`. The generated type covers all server
// error codes; client-side codes (NetworkError, Unknown) are added as a union.
export type ErrorCode    = components["schemas"]["ErrorCode"];
export type DomainError  = ErrorCode | "NetworkError" | "Unknown";

export type Result<T> = { ok: true; value: T } | { ok: false; error: DomainError };
function ok<T>(value: T): Result<T> { return { ok: true, value }; }
function err(error: DomainError): Result<never> { return { ok: false, error }; }
function ifMatch(version: number): HeadersInit { return { "If-Match": `"${version}"` }; }

/**
 * Resolved at every call so vitest (and other consumers) can override
 * `VITE_API_BASE` after this module has been imported.
 */
function getBase(): string {
  return (import.meta.env.VITE_API_BASE as string | undefined) ?? "/api";
}

// FIX: the runtime validator used to be a hand-maintained string set that could
// drift from the generated OpenAPI ErrorCode union. This exhaustive Record
// forces typecheck to fail whenever the Rust enum changes and the generated type
// gains or loses a code that SERVER_ERROR_CODE_MAP does not mirror.
const SERVER_ERROR_CODE_MAP = {
  UnauthorizedActor: true,
  AccessDenied: true,
  PassengerNotFound: true,
  PassengerAlreadyExists: true,
  ResourceNotFound: true,
  ResourceAlreadyExists: true,
  CrewLeadNotFound: true,
  CrewLeadAlreadyExists: true,
  CrewLeadLimitReached: true,
  CrewLeadMinimumBreached: true,
  CrewLeadBootstrapInvalid: true,
  InvalidInput: true,
  Unauthorized: true,
  VersionConflict: true,
  InternalError: true,
  DatabaseUnreachable: true,
} as const satisfies Record<ErrorCode, true>;

const KNOWN_CODES: ReadonlySet<string> = new Set<string>(
  Object.keys(SERVER_ERROR_CODE_MAP),
);

function toDomainError(code: string | undefined): DomainError {
  if (code !== undefined && KNOWN_CODES.has(code)) {
    return code as DomainError;
  }
  return "Unknown";
}

interface ErrorBody {
  error: string;
  code: string;
}

/**
 * Bearer token sent with every mutating request.
 * Set via `api.setToken(token)` before calling any mutating endpoint.
 * Defaults to the demo crew-lead token when VITE_API_TOKEN is set.
 */
let _token: string | null =
  (import.meta.env.VITE_API_TOKEN as string | undefined) ?? null;

/**
 * Subscribers notified when any API call returns 401 Unauthorized.
 * The UI can subscribe to show an error banner / redirect to login.
 */
type UnauthorizedListener = () => void;
const _unauthorizedListeners = new Set<UnauthorizedListener>();

function notifyUnauthorized(): void {
  for (const listener of _unauthorizedListeners) {
    listener();
  }
}

async function call<T>(path: string, init?: RequestInit): Promise<Result<T>> {
  try {
    const res = await fetch(`${getBase()}${path}`, {
      ...init,
      headers: {
        "Content-Type": "application/json",
        // FIX: actor identity derived from bearer token, never from request body.
        // Include the header only when a token is configured.
        ...(_token ? { Authorization: `Bearer ${_token}` } : {}),
        ...(init?.headers ?? {}),
      },
    });
    if (res.status === 204) {
      // No content — synthesise a unit value.
      return ok(undefined as unknown as T);
    }
    if (!res.ok) {
      const body = (await res.json().catch(() => null)) as ErrorBody | null;
      // FIX: propagate 401 to all registered listeners so the UI can show
      // an "invalid/expired token" error banner without each call site
      // needing to handle the Unauthorized code explicitly.
      if (res.status === 401) {
        notifyUnauthorized();
      }
      return err(toDomainError(body?.code));
    }
    return ok((await res.json()) as T);
  } catch (_e) {
    return err("NetworkError");
  }
}

export const api = {
  get base(): string {
    return getBase();
  },

  /**
   * Register a callback invoked whenever any request returns 401 Unauthorized.
   * Returns an unsubscribe function.
   *
   * Usage:
   *   const unsub = api.onUnauthorized(() => setAuthError(true));
   *   // later: unsub();
   */
  onUnauthorized(listener: UnauthorizedListener): () => void {
    _unauthorizedListeners.add(listener);
    return () => { _unauthorizedListeners.delete(listener); };
  },

  /** Configure the bearer token sent with every mutating request. */
  setToken(token: string | null): void {
    _token = token;
  },

  /** Return the currently configured bearer token (null if unset). */
  getToken(): string | null {
    return _token;
  },

  health: (): Promise<Result<string>> =>
    fetch(`${getBase()}/health`)
      .then(async (r) => (r.ok ? ok(await r.text()) : err("Unknown")))
      .catch(() => err("NetworkError")),

  authCheck: () => call<{ actor_id: string }>("/auth/check"),

  crewLeads: () => call<ApiCrewLead[]>("/crew-leads"),
  passengers: () => call<ApiPassenger[]>("/passengers"),
  resources: () => call<ApiResource[]>("/resources"),
  audit: () => call<ApiAdminEvent[]>("/audit"),
  usage: () => call<ApiUsageEvent[]>("/usage"),
  byTier: () => call<ApiTierCount[]>("/reports/by-tier"),
  topResources: (n: number) =>
    call<ApiTopResource[]>(`/reports/top-resources?n=${n}`),
  history: (passengerId: string) =>
    call<ApiUsageEvent[]>(
      `/reports/history/${encodeURIComponent(passengerId)}`,
    ),

  passenger: (id: string) =>
    call<ApiPassenger>(`/passengers/${encodeURIComponent(id)}`),

  resource: (id: string) =>
    call<ApiResource>(`/resources/${encodeURIComponent(id)}`),

  accessibleFor: (tier: Tier) =>
    call<ApiResource[]>(`/resources/accessible?tier=${tier}`),

  reset: (): Promise<Result<void>> =>
    call<void>("/reset", {
      method: "POST",
    }),

  addCrewLead: (lead: ApiCrewLead): Promise<Result<void>> =>
    call<void>("/crew-leads", {
      method: "POST",
      body: JSON.stringify({ lead }),
    }),

  removeCrewLead: (id: string): Promise<Result<void>> =>
    call<void>(`/crew-leads/${encodeURIComponent(id)}`, {
      method: "DELETE",
    }),

  replaceCrewLead: (
    oldId: string,
    newLead: ApiCrewLead,
  ): Promise<Result<void>> =>
    call<void>(`/crew-leads/${encodeURIComponent(oldId)}`, {
      method: "PUT",
      body: JSON.stringify({ new_lead: newLead }),
    }),

  useResource: (resourceId: string) =>
    call<ApiUsageEvent>("/access", {
      method: "POST",
      body: JSON.stringify({
        resource_id: resourceId,
      }),
    }),

  createPassenger: (
    id: string,
    name: string,
    tier: Tier,
  ): Promise<Result<ApiPassenger>> =>
    call<ApiPassenger>("/passengers", {
      method: "POST",
      body: JSON.stringify({ id, name, tier }),
    }),

  changePassengerTier: (
    id: string,
    tier: Tier,
    version: number,
  ): Promise<Result<void>> =>
    call<void>(`/passengers/${encodeURIComponent(id)}/tier`, {
      method: "PATCH",
      headers: ifMatch(version),
      body: JSON.stringify({ tier }),
    }),

  softDeletePassenger: (id: string, version: number): Promise<Result<void>> =>
    call<void>(`/passengers/${encodeURIComponent(id)}`, {
      method: "DELETE",
      headers: ifMatch(version),
    }),

  createResource: (
    id: string,
    name: string,
    category: string,
    minTier: Tier,
  ): Promise<Result<ApiResource>> =>
    call<ApiResource>("/resources", {
      method: "POST",
      body: JSON.stringify({
        id,
        name,
        category,
        min_tier: minTier,
      }),
    }),

  changeResourceMinTier: (
    id: string,
    tier: Tier,
    version: number,
  ): Promise<Result<void>> =>
    call<void>(`/resources/${encodeURIComponent(id)}/min-tier`, {
      method: "PATCH",
      headers: ifMatch(version),
      body: JSON.stringify({ tier }),
    }),

  softDeleteResource: (id: string, version: number): Promise<Result<void>> =>
    call<void>(`/resources/${encodeURIComponent(id)}`, {
      method: "DELETE",
      headers: ifMatch(version),
    }),
};
