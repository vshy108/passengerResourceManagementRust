// Mirrors src/domain/errors.rs (#[non_exhaustive] DomainError).
export type DomainError =
  | "UnauthorizedActor"
  | "PassengerNotFound"
  | "PassengerAlreadyExists"
  | "ResourceNotFound"
  | "ResourceAlreadyExists"
  | "CrewLeadNotFound"
  | "CrewLeadAlreadyExists"
  | "CrewLeadLimitReached"
  | "CrewLeadMinimumBreached"
  | "CrewLeadBootstrapInvalid";

// Result mirrors std::result::Result<T, DomainError>.
export type Ok<T> = { ok: true; value: T };
export type Err = { ok: false; error: DomainError };
export type Result<T> = Ok<T> | Err;

export const ok = <T>(value: T): Ok<T> => ({ ok: true, value });
export const err = (error: DomainError): Err => ({ ok: false, error });
