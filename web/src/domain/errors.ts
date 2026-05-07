// Mirrors src/domain/errors.rs (#[non_exhaustive] DomainError).
// String-literal union chosen over an enum because TypeScript
// discriminated unions on string literals give exhaustiveness checking
// in switch/if chains without needing a class or object.
export type DomainError =
  | "UnauthorizedActor"       // Actor lacks permission for the operation (PS-E1, RS-E1, AC-E1)
  | "AccessDenied"            // Passenger tier is below resource min_tier (AC-E2)
  | "PassengerNotFound"       // Unknown or soft-deleted passenger id (PS-E3, AC-E3)
  | "PassengerAlreadyExists"  // Active passenger with the same id already exists (PS-E2)
  | "ResourceNotFound"        // Unknown or soft-deleted resource id (RS-E3, AC-E4)
  | "ResourceAlreadyExists"   // Active resource with the same id already exists (RS-E2)
  | "CrewLeadNotFound"        // Unknown Crew Lead id on remove/replace (CL-E4)
  | "CrewLeadAlreadyExists"   // Duplicate Crew Lead id (CL-E3)
  | "CrewLeadLimitReached"    // Attempted to add a 4th Crew Lead (CL-E1)
  | "CrewLeadMinimumBreached" // Attempted to remove without replacement (CL-E2)
  | "CrewLeadBootstrapInvalid" // Bootstrap called with != 3 distinct leads (CL-E5)
  // Extra variants used only on the HTTP client side — not in the Rust domain.
  | "NetworkError"  // Fetch failed before a response was received.
  | "Unknown";      // Catch-all for unexpected server responses.

// Result<T> mirrors std::result::Result<T, DomainError>.
// TypeScript has no built-in Result type, so we define a discriminated
// union: `ok: true` and `ok: false` are the discriminant that narrows
// the type inside an `if (r.ok)` branch.
export type Ok<T> = { ok: true; value: T };
export type Err = { ok: false; error: DomainError };
export type Result<T> = Ok<T> | Err;

// Convenience constructors — mirror the `ok(...)` and `err(...)` free
// functions common in Rust/fp-ts style code.
export const ok = <T>(value: T): Ok<T> => ({ ok: true, value });
export const err = (error: DomainError): Err => ({ ok: false, error });
