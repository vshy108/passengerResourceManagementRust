// Newtype-like brands (mirrors the Rust ID newtypes) — they prevent
// accidental mix-ups at the type level even though the runtime is just a string.

declare const brand: unique symbol;
type Brand<T, B> = T & { readonly [brand]: B };

export type PassengerId = Brand<string, "PassengerId">;
export type ResourceId = Brand<string, "ResourceId">;
export type CrewLeadId = Brand<string, "CrewLeadId">;

export const passengerId = (s: string): PassengerId => s as PassengerId;
export const resourceId = (s: string): ResourceId => s as ResourceId;
export const crewLeadId = (s: string): CrewLeadId => s as CrewLeadId;
