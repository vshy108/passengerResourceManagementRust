// Thin client for the Rust axum server (src/interface/http.rs).
//
// Configure the base URL via the Vite env var `VITE_API_BASE`. When
// unset, requests go to `/api/*` and are proxied by Vite (see
// vite.config.ts) to the local serve binary at 127.0.0.1:8080. Set
// `VITE_API_BASE` to point at a remote server.
//
// All methods return a tagged `Result<T>` mirroring the rest of the
// demo so error handling is uniform between the in-process services
// and live API calls.

import type { Tier } from "../domain/tier";

export type DomainError = string;
export type Result<T> = { ok: true; value: T } | { ok: false; error: DomainError };
function ok<T>(value: T): Result<T> { return { ok: true, value }; }
function err(error: DomainError): Result<never> { return { ok: false, error }; }

/**
 * Resolved at every call so vitest (and other consumers) can override
 * `VITE_API_BASE` after this module has been imported.
 */
function getBase(): string {
  return (import.meta.env.VITE_API_BASE as string | undefined) ?? "/api";
}

const KNOWN_CODES: ReadonlySet<string> = new Set<DomainError>([
  "UnauthorizedActor",
  "AccessDenied",
  "PassengerNotFound",
  "PassengerAlreadyExists",
  "ResourceNotFound",
  "ResourceAlreadyExists",
  "CrewLeadNotFound",
  "CrewLeadAlreadyExists",
  "CrewLeadLimitReached",
  "CrewLeadMinimumBreached",
  "CrewLeadBootstrapInvalid",
  "NetworkError",
  "Unknown",
]);

function toDomainError(code: string | undefined): DomainError {
  if (code !== undefined && KNOWN_CODES.has(code)) {
    return code as DomainError;
  }
  return "Unknown";
}

export interface ApiCrewLead {
  id: string;
  name: string;
}

export interface ApiPassenger {
  id: string;
  name: string;
  tier: Tier;
  deleted_at: number | null;
}

export interface ApiResource {
  id: string;
  name: string;
  category: string;
  min_tier: Tier;
  deleted_at: number | null;
}

export interface ApiUsageEvent {
  id: number;
  passenger_id: string;
  resource_id: string;
  tier_at_attempt: Tier;
  min_tier_at_attempt: Tier;
  timestamp: number;
  outcome: "Allowed" | "Denied";
}

export interface ApiAdminEvent {
  id: number;
  actor_id: string;
  action: string;
  target_kind: string;
  target_id: string;
  timestamp: number;
  details: string | null;
}

export interface ApiTierCount {
  tier: Tier;
  allowed: number;
  denied: number;
}

export interface ApiTopResource {
  resource_id: string;
  allowed_count: number;
}

interface ErrorBody {
  error: string;
  code: string;
}

async function call<T>(path: string, init?: RequestInit): Promise<Result<T>> {
  try {
    const res = await fetch(`${getBase()}${path}`, {
      ...init,
      headers: {
        "Content-Type": "application/json",
        ...(init?.headers ?? {}),
      },
    });
    if (res.status === 204) {
      // No content — synthesise a unit value.
      return ok(undefined as unknown as T);
    }
    if (!res.ok) {
      const body = (await res.json().catch(() => null)) as ErrorBody | null;
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

  health: (): Promise<Result<string>> =>
    fetch(`${getBase()}/health`)
      .then(async (r) => (r.ok ? ok(await r.text()) : err("Unknown")))
      .catch(() => err("NetworkError")),

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

  reset: (actorId: string): Promise<Result<void>> =>
    call<void>("/reset", {
      method: "POST",
      body: JSON.stringify({ actor_id: actorId }),
    }),

  addCrewLead: (lead: ApiCrewLead): Promise<Result<void>> =>
    call<void>("/crew-leads", {
      method: "POST",
      body: JSON.stringify({ lead }),
    }),

  removeCrewLead: (actorId: string, id: string): Promise<Result<void>> =>
    call<void>(`/crew-leads/${encodeURIComponent(id)}`, {
      method: "DELETE",
      body: JSON.stringify({ actor_id: actorId }),
    }),

  replaceCrewLead: (
    actorId: string,
    oldId: string,
    newLead: ApiCrewLead,
  ): Promise<Result<void>> =>
    call<void>(`/crew-leads/${encodeURIComponent(oldId)}`, {
      method: "PUT",
      body: JSON.stringify({ actor_id: actorId, new_lead: newLead }),
    }),

  useResource: (passengerId: string, resourceId: string) =>
    call<ApiUsageEvent>("/access", {
      method: "POST",
      body: JSON.stringify({
        passenger_id: passengerId,
        resource_id: resourceId,
      }),
    }),

  createPassenger: (
    actorId: string,
    id: string,
    name: string,
    tier: Tier,
  ): Promise<Result<ApiPassenger>> =>
    call<ApiPassenger>("/passengers", {
      method: "POST",
      body: JSON.stringify({ actor_id: actorId, id, name, tier }),
    }),

  changePassengerTier: (
    actorId: string,
    id: string,
    tier: Tier,
  ): Promise<Result<void>> =>
    call<void>(`/passengers/${encodeURIComponent(id)}/tier`, {
      method: "PATCH",
      body: JSON.stringify({ actor_id: actorId, tier }),
    }),

  softDeletePassenger: (actorId: string, id: string): Promise<Result<void>> =>
    call<void>(`/passengers/${encodeURIComponent(id)}`, {
      method: "DELETE",
      body: JSON.stringify({ actor_id: actorId }),
    }),

  createResource: (
    actorId: string,
    id: string,
    name: string,
    category: string,
    minTier: Tier,
  ): Promise<Result<ApiResource>> =>
    call<ApiResource>("/resources", {
      method: "POST",
      body: JSON.stringify({
        actor_id: actorId,
        id,
        name,
        category,
        min_tier: minTier,
      }),
    }),

  changeResourceMinTier: (
    actorId: string,
    id: string,
    tier: Tier,
  ): Promise<Result<void>> =>
    call<void>(`/resources/${encodeURIComponent(id)}/min-tier`, {
      method: "PATCH",
      body: JSON.stringify({ actor_id: actorId, tier }),
    }),

  softDeleteResource: (actorId: string, id: string): Promise<Result<void>> =>
    call<void>(`/resources/${encodeURIComponent(id)}`, {
      method: "DELETE",
      body: JSON.stringify({ actor_id: actorId }),
    }),
};
