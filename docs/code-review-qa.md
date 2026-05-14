# Code Review Q&A — Spaceship X26 PRMS

> Reference codebase: `passengerResourceManagementRust` (Rust 2024, stable)
> Last updated: 2026-05-14

---

## Table of Contents

1. [Architecture & Layering](#1-architecture--layering)
2. [Domain Layer](#2-domain-layer)
3. [Application Layer — Services](#3-application-layer--services)
4. [Application Layer — Ports & Guards](#4-application-layer--ports--guards)
5. [Infrastructure Layer](#5-infrastructure-layer)
6. [Interface Layer — DTOs](#6-interface-layer--dtos)
7. [Interface Layer — HTTP Handlers](#7-interface-layer--http-handlers)
8. [Error Handling](#8-error-handling)
9. [Concurrency & Thread Safety](#9-concurrency--thread-safety)
10. [Rust Language Specifics](#10-rust-language-specifics)
11. [Testing Strategy](#11-testing-strategy)
12. [Security & Input Validation](#12-security--input-validation)
13. [Design Patterns](#13-design-patterns)
14. [Full-Stack — REST API Design](#14-full-stack--rest-api-design)
15. [Full-Stack — TypeScript / React Frontend](#15-full-stack--typescript--react-frontend)
16. [Full-Stack — Frontend ↔ Backend Contract](#16-full-stack--frontend--backend-contract)
17. [Full-Stack — State Management](#17-full-stack--state-management)
18. [Full-Stack — Observability & Operability](#18-full-stack--observability--operability)
19. [Full-Stack — Performance & Scalability](#19-full-stack--performance--scalability)
20. [Full-Stack — Deployment & DevOps](#20-full-stack--deployment--devops)
21. [Full-Stack — Product & Tradeoff Thinking](#21-full-stack--product--tradeoff-thinking)
22. [Extra Interviewer Angles](#22-extra-interviewer-angles)
23. [Prompt & Submission Rubric Questions](#23-prompt--submission-rubric-questions)
24. [Rapid-Fire Reviewer Follow-Ups](#24-rapid-fire-reviewer-follow-ups)
25. [Exhaustive Deep-Dive Question Bank](#25-exhaustive-deep-dive-question-bank)
26. [Adjacent Full-Stack Review Questions](#26-adjacent-full-stack-review-questions)

---

## Tag Guide

Use these tags mentally while studying. They are intentionally short so you can scan fast before a review.

| Tag | Meaning | Best For |
|---|---|---|
| `[CORE]` | Must-know project fundamentals | Architecture, domain, access policy, audit invariants |
| `[RUST]` | Rust language / type-system questions | Ownership, borrowing, generics, traits, atomics |
| `[API]` | HTTP / REST / DTO contract questions | Axum, routes, status codes, OpenAPI, CORS |
| `[FE]` | Frontend questions | React, TypeScript, Vite, state, UX, accessibility |
| `[TEST]` | Testing and coverage questions | TDD, spec IDs, nextest, coverage, property testing |
| `[SEC]` | Security / privacy / auth questions | Caller-supplied actors, CORS, PII, threat modeling |
| `[OPS]` | Deployment and operations questions | CI, logging, metrics, incidents, migrations, backups |
| `[SCALE]` | Performance and scalability questions | Mutex bottleneck, Vec lookups, pagination, indexes |
| `[TRADEOFF]` | Design trade-off questions | Why this design, what would change in production |
| `[PITCH]` | Interview communication questions | How to explain, demo, defend, or summarize the project |

### Fast Study Order

If you only have limited time, read in this order:

1. `[CORE]` Sections 1-4, 8, 11-13
2. `[API]` / `[FE]` Sections 14-17
3. `[SEC]` / `[OPS]` Sections 18, 20, 22-23
4. `[PITCH]` Sections 21, 24, 26
5. `[RUST]` Deep dive in Section 25 if the reviewer is technical

### Tagging Style If You Want To Mark Individual Questions Later

For individual questions, use this format:

```md
**Q: [CORE][TRADEOFF] Why does `Tier` use `rank()` instead of derived ordering?**
```

I recommend tagging only high-value questions, not every question. Too many tags can make the file harder to read.

---

## 1. Architecture & Layering

Related files: [AGENTS.md](../AGENTS.md), [README.md](../README.md), [src/lib.rs](../src/lib.rs), [src/interface/composition_root.rs](../src/interface/composition_root.rs), [Cargo.toml](../Cargo.toml)

**Q: What is the dependency rule in this codebase and why does it matter?**

A: Dependencies point **inward only**: `interface → application → domain`, with `infrastructure` depending on `application` (it implements port traits). The domain layer has zero I/O, no HTTP, and no system-clock calls. This means all business logic can be compiled and tested without any infrastructure being present. Reversing any arrow — for example, having a domain struct import a serde type — would couple the core model to a serialisation library and make every domain test depend on that library too.

---

**Q: What is the composition root and why is all wiring in one function?**

A: `build_demo_world()` in [`src/interface/composition_root.rs`](../src/interface/composition_root.rs) is the sole place where concrete types (`FakeClock`, `InMemoryUsageEventSink`, `InMemoryAdminEventSink`) are instantiated and injected into services. Benefits:
- Every binary and every test assembles the same object graph from one function — no hidden global state.
- Swapping an adapter (e.g., replacing `FakeClock` with a system clock) requires editing exactly one file.
- Services never "know" which concrete type they received; they only see the port trait.

---

**Q: Why does a `World` struct exist? Why not pass each service individually?**

A: `World` is a **composition root container** — a named struct whose only job is to give `Arc<Mutex<T>>` a single `T` to wrap.

**The problem without it:** Axum's `State<T>` is one generic slot. With five services you would need five separate `Arc<Mutex<...>>` — one per service. That creates three problems:
1. **Deadlock risk** — if two handlers acquire multiple locks in different orders, they can deadlock.
2. **Borrow splitting breaks** — the `use_resource` handler needs `passengers` and `resources` as immutable borrows *and* `access` as a mutable borrow simultaneously. Rust's borrow checker can split borrows across *fields of one struct*, but not across separate `MutexGuard`s from separate locks.
3. **Handler signatures explode** — every handler would carry five `State<Arc<Mutex<...>>>` extractor arguments.

**What `World` gives you:**
```rust
pub type AppState = Arc<Mutex<World>>;
```
- One lock, one guard, one `State<AppState>` in every handler.
- `let World { passengers, resources, access, .. } = &mut *w;` gives the borrow checker enough information to approve the split borrows in `use_resource`.
- `/reset` atomically replaces all state in a single line: `*lock_world(&state) = fresh_world;`
- `build_demo_world()` is the **only** file that knows which concrete types are wired together — domain and application layers never import `World`.

If the system later moves to per-aggregate locks, `World` is removed and each service gets its own `Arc<Mutex<...>>` — the change is localised entirely to `composition_root.rs` and `http.rs`.

---

**Q: Why is the HTTP feature gated behind `--features http`?**

A: It keeps the core library (`lib.rs`) free of Axum, Tokio, tower-http, and utoipa dependencies. Projects that embed this crate as a library but provide their own transport (gRPC, CLI, message bus) don't pay the compile-time or binary-size cost of a web framework. The `serve.rs` binary only compiles with that feature enabled, enforcing the separation at the toolchain level.

---

**Q: The `src/bin/` directory has only `serve.rs`. Why no `cli.rs`?**

A: The spec (`AGENTS.md §2`) mentions `cli.rs` as a planned entrypoint, but only the HTTP server is scoped for the current slice. Adding the CLI later means adding `src/bin/cli.rs` and optionally a `cli` feature — none of the existing layers would need to change because business logic lives in the service layer, not the binary.

---

**Q: What would you change first if you needed to add a real database?**

A: Write a new struct in `src/infrastructure/` (e.g., `PostgresPassengerRepo`) that implements the port traits (`UsageEventSink`, `UsageEventSource`, etc.). Update `build_demo_world()` to inject it instead of the in-memory implementations. The domain and application layers remain untouched. This is the payoff of the Repository pattern + dependency inversion: the "seam" between business logic and storage is a trait, not a concrete import.

---

## 2. Domain Layer

Related files: [src/domain/tier.rs](../src/domain/tier.rs), [src/domain/passenger.rs](../src/domain/passenger.rs), [src/domain/resource.rs](../src/domain/resource.rs), [src/domain/usage_event.rs](../src/domain/usage_event.rs), [src/domain/admin_event.rs](../src/domain/admin_event.rs), [src/domain/errors.rs](../src/domain/errors.rs), [specs/01-tier-policy.md](../specs/01-tier-policy.md), [specs/05-access.md](../specs/05-access.md)

**Q: Why does `Tier` have an explicit `rank()` method instead of deriving `PartialOrd`?**

A: Derived `PartialOrd` on enums uses declaration order. If someone reorders the variants in the source file (e.g., alphabetically), the ordering semantics silently change. `rank()` returns explicit `u8` constants (`Silver=1`, `Gold=2`, `Platinum=3`) that are pinned to the spec (`TP-R1`). The `can_access` method always goes through `rank()`, making the ordering contract visible and compiler-checked: adding a new tier requires adding a new arm to the `match`, or it won't compile.

---

**Q: [EXTENSIBILITY] Adding a new tier currently requires changes in 7 spots across 3 files. Is there a better approach?**

A: Yes — and the right fix has two independent parts with different tradeoff profiles.

**Part 1 — domain `tier.rs` (3 spots → 1): `define_tiers!` macro (high value, high cost)**

A `macro_rules!` macro can generate the enum, `rank()`, `TryFrom<&str>`, and a new `as_str()` method from a single table:

```rust
macro_rules! define_tiers {
    ( $( ($name:ident, $rank:expr) ),* $(,)? ) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum Tier { $( $name, )* }

        impl Tier {
            pub fn rank(self) -> u8 { match self { $( Tier::$name => $rank, )* } }
            pub fn as_str(self) -> &'static str { match self { $( Tier::$name => stringify!($name), )* } }
            pub const ALL: &'static [Tier] = &[ $( Tier::$name, )* ];
        }

        impl TryFrom<&str> for Tier {
            type Error = InvalidTier;
            fn try_from(value: &str) -> Result<Self, Self::Error> {
                match value { $( stringify!($name) => Ok(Tier::$name), )* _ => Err(InvalidTier(value.to_owned())) }
            }
        }
    };
}

define_tiers! { (Silver, 1), (Gold, 2), (Diamond, 3), (Platinum, 4) }
```

Adding a tier becomes one line. The compiler still enforces exhaustiveness inside the macro-generated matches.

**Tradeoffs:** IDE "go to definition" degrades (lands on the macro call, not a clear line). Compile errors inside the expansion point at the call site. For a learning repo the verbosity of explicit code is more valuable — a reader can understand `enum Tier { Silver, Gold, ... }` without knowing `macro_rules!`. Reserve this for a production monorepo with many contributors and frequent tier changes.

**Part 2 — infrastructure stores (2 spots each → 0): `as_str()` delegation (high value, low cost)**

The `tier_to_str` / `tier_from_str` helpers in `pg_store.rs` and `sqlite_event_store.rs` hold match arms that the compiler does NOT check (they're string comparisons, not exhaustive pattern matches on the enum). Adding a tier without updating them causes a silent runtime error.

Fix by adding `as_str()` to the plain domain enum (no macro needed) and delegating from both stores:

```rust
// domain/tier.rs — add one method, keep everything else explicit
pub fn as_str(self) -> &'static str {
    // Reuse TIER_NAMES — no second match to maintain.
    TIER_NAMES
        .iter()
        .find(|(_, t)| *t == self)
        .map(|(name, _)| *name)
        .expect("TIER_NAMES covers all variants")
}
```

```rust
// infrastructure/pg_store.rs and sqlite_event_store.rs — replace match bodies
fn tier_to_str(t: Tier) -> &'static str { t.as_str() }
fn tier_from_str(s: &str) -> rusqlite::Result<Tier> {
    Tier::try_from(s).map_err(|e| rusqlite::Error::InvalidColumnName(e.0))
}
```

After this, adding a tier updates only `tier.rs` (3 spots, all compiler-guided) and `dto.rs` (the wire DTO, which is intentionally separate). The infrastructure stores need zero changes.

**Recommendation:** Apply Part 2 only. It eliminates the silent-failure risk with a three-line change and zero cognitive overhead. Part 1 is available if the team later decides the single-source-of-truth benefit justifies the macro complexity.

---

**Q: Why are `PassengerId`, `ResourceId`, and `CrewLeadId` newtypes rather than plain `String`?**

A: Newtypes make IDs incompatible at the type level. Passing a `ResourceId` where a `PassengerId` is expected is a compile error, not a silent bug. In a handler that receives three different IDs in the same request, this prevents argument-order mistakes that would otherwise only surface at runtime. The cost is wrapping/unwrapping via `.0`, which has zero runtime overhead.

---

**Q: `deleted_at` is `Option<Timestamp>` and only ever set once. Why not use a dedicated `DeletedPassenger` type?**

A: The spec (PS-I2) requires `get()` to return the latest soft-deleted record — the service needs to look up deleted entities by ID. Keeping them in the same `Passenger` struct means the lookup code is uniform. A separate type would require a second map and duplicated lookup logic. The invariant "set once, never cleared" is enforced procedurally: `soft_delete` only moves entries from `active` to `deleted`; nothing ever moves them back or changes `deleted_at` after it is set.

---

**Q: `UsageEvent` stores `tier_at_attempt` and `min_tier_at_attempt` as snapshots rather than just the IDs. Is that necessary?**

A: Yes, per AC-R5/R6. Tier changes are allowed post-hoc (a Silver passenger can be upgraded to Platinum). Without snapshots, a retrospective lookup of the passenger's current tier would rewrite history — a past denial would look like an obviously-allowed access. Storing both values at event creation makes the audit log self-contained: every row explains the outcome without any external join to current entity state.

---

**Q: Why is `Outcome` `Copy` but `UsageEvent` is not?**

A: `Outcome` is a unit-only enum (two variants, no payload) — it fits in a single byte and has no heap resources, so `Copy` is safe and convenient. `UsageEvent` contains `PassengerId` and `ResourceId` (which wrap `String`, a heap-allocated type). `String` is not `Copy` because copying it would require a heap allocation. The correct operation for `UsageEvent` is `.clone()` (explicit, intentional copy), not implicit `Copy`.

---

**Q: Why does the `domain/` module use `thiserror` but is otherwise restricted from external crates?**

A: `thiserror` is a pure procedural macro crate that generates `std::error::Error` impls — it produces zero runtime code and adds no transitive dependencies beyond the macro expansion. The real restriction is on I/O, async runtimes, and serialisation crates (serde, axum) in the domain. Allowing `thiserror` and `chrono` (for timestamp types) is a pragmatic exception that buys ergonomic error handling without any architectural harm.

---

**Q: `AdminAction` is a closed enum (no `#[non_exhaustive]`), but `DomainError` is `#[non_exhaustive]`. Why the difference?**

A: `AdminAction` is an internal domain type only ever consumed by this crate's own `match` blocks. Making it exhaustive means the compiler points at every unhandled case when a new action is added — this is exactly what you want when extending the audit system. `DomainError` is the public surface returned to callers outside the crate; adding a new variant should not be a breaking change for consumers, so `#[non_exhaustive]` forces them to include a wildcard arm, making forward-compatibility explicit.

---

## 3. Application Layer — Services

Related files: [src/application/crew_lead_service.rs](../src/application/crew_lead_service.rs), [src/application/passenger_service.rs](../src/application/passenger_service.rs), [src/application/resource_service.rs](../src/application/resource_service.rs), [src/application/access_service.rs](../src/application/access_service.rs), [src/application/reporting_service.rs](../src/application/reporting_service.rs), [tests/lifecycle.rs](../tests/lifecycle.rs)

**Q: Why is `CrewLeadService::add` implemented as "always returns `CrewLeadLimitReached`"?**

A: The spec (CL-R2) says the cap is fixed at exactly 3 and adding beyond that is always rejected. Implementing `add` as an unconditional error rather than a conditional check makes the invariant structurally obvious: you can't accidentally reach `CrewLeadLimitReached` only on some code paths. It also satisfies callers that need to call `add` via a trait without checking the count themselves. The same logic applies to `remove` always returning `CrewLeadMinimumBreached`.

---

**Q: `CrewLeadService::replace` does an in-place swap of one slot. What invariants does it protect?**

A: Two invariants: (1) the count stays exactly 3 — no element is added or removed, only updated; (2) duplicate IDs are rejected — if `new_lead.id` matches a *different* slot, it returns `CrewLeadAlreadyExists`. Replacing a slot with the same `id` (i.e., renaming the lead in-place) is explicitly allowed because the slot index check (`i != slot`) gates the collision detection.

---

**Q: `PassengerService` uses two `Vec`s (`active` and `deleted`) instead of one `Vec` with a boolean flag. What is the trade-off?**

A: With two Vecs:
- `list()` is O(n) over active entries with no filtering — the hot path has no wasted work.
- `soft_delete` is a `remove`+`push` — O(n) swap, but rare.
- Code explicitly models the two states as separate collections, making the invariant visible.

With a single `Vec<(Passenger, bool)>`:
- `list()` must filter every call — wasted work proportional to deleted count.
- Logic is slightly simpler but conflates two distinct collections into one.

For a small roster (dozens of passengers) the performance difference is irrelevant; the design choice here is primarily about clarity.

---

**Q: `PassengerService::get` checks active first, then deleted. Why does the order matter?**

A: It implements the spec rule (PS-R9) that `get` returns the active record if one exists, otherwise the latest soft-deleted one. If you checked `deleted` first, an active passenger with the same ID (which shouldn't happen given `soft_delete` moves the record out of `active`) would be hidden. The order also documents intent: "is this person active? If not, find their archived record."

---

**Q: `AccessService::use_resource` has extra generic parameters `<PC, RC>` for the passenger and resource service clocks. Why not reuse the service's `C`?**

A: In tests and compositions where different services happen to use different `FakeClock` instances (or where the access service's clock is distinct from the others), the types would not match. Allowing independent clock generics on the method makes the API more flexible without any runtime cost — the compiler generates specialised code per concrete combination. In practice, most callers pass the same concrete clock type everywhere, but the API doesn't enforce that restriction.

---

**Q: Where does `next_event_id` live and how is it kept monotonic?**

A: In `AccessService` as a `u64` field initialised to `1`. It is incremented (`+= 1`) immediately after constructing each `UsageEvent`, before appending to the sink. Because `AccessService` is behind `&mut self` on `use_resource`, only one call can be in-flight at a time — no concurrent ID collision is possible. The ID is never reset, so event IDs are unique for the lifetime of the service.

---

**Q: `ReportingService` is generic over `'a` and `S: UsageEventSource + ?Sized`. When does `?Sized` matter?**

A: `?Sized` opts out of the default `Sized` bound, allowing `S` to be a **trait object** (`dyn UsageEventSource`). Without it, writing `ReportingService<dyn UsageEventSource>` would not compile because `dyn Trait` has unknown size. In HTTP tests, the sink is behind a `MutexGuard`, and passing a `&dyn UsageEventSource` is more ergonomic than threading a concrete generic type through the handler. The `'a` lifetime ties the borrow of the source to the `ReportingService`'s lifetime so the service cannot outlive its data.

---

**Q: `top_resources` only counts `Outcome::Allowed` events, not `Outcome::Denied`. Is that correct per the spec?**

A: Yes, per RP-R3: "top resources by successful (allowed) access count." Including denied attempts would inflate counts for highly-restricted resources that many low-tier passengers attempt but fail to use — the metric would measure "most attempted" not "most used". Separating the two makes the report actionable: it shows which resources are actually being consumed.

---

## 4. Application Layer — Ports & Guards

Related files: [src/application/ports.rs](../src/application/ports.rs), [src/application/guards.rs](../src/application/guards.rs), [src/domain/actor.rs](../src/domain/actor.rs)

**Q: Why do all port traits require `Send + Sync`?**

A: In the HTTP server, the `World` struct (which holds all services) is wrapped in `Arc<Mutex<World>>` and shared across multiple Axum handler threads. For `Arc<T>` to be `Send + Sync`, every `T` inside it must also be `Send + Sync`. Adding `Send + Sync` as supertraits on `Clock`, `UsageEventSink`, etc. pushes that requirement onto implementors, giving a compile-time guarantee that no non-thread-safe type (e.g., `Rc<T>`) can accidentally be injected. The cost is `FakeClock` must be thread-safe, which is why it uses `AtomicI64` rather than `Cell<i64>`.

---

**Q: `UsageEventSink` and `UsageEventSource` are two separate traits over the same data. Why split them?**

A: ISP (Interface Segregation Principle). `AccessService` needs write-only access (`append`); `ReportingService` needs read-only access (`list`). Splitting the traits means each service declares exactly the capability it needs. `InMemoryUsageEventSink` implements both, but a future read-only replica or a streaming sink would implement only one. It also makes test doubles easier: you can pass a minimal mock that only implements `UsageEventSink` to test access, without implementing the read path.

---

**Q: Why is `require_crew_lead` a free function rather than a method on `Actor` or `CrewLeadService`?**

A: It is an *authorisation guard* shared by `PassengerService`, `ResourceService`, and `CrewLeadService`. If it were a method on `Actor`, the domain type would encode application-layer policy — a layering violation. If it were on `CrewLeadService`, every other service would need to import `CrewLeadService` just for the guard, creating an unnecessary coupling. As a free function in `application/guards.rs`, it is a shared utility with a minimal, focused dependency footprint.

---

**Q: `require_crew_lead` returns `&CrewLeadId` borrowed from the actor. Why not return an owned `CrewLeadId`?**

A: Returning a reference avoids a heap allocation (cloning the inner `String`) when the caller doesn't need an owned copy. Callers that do need ownership (e.g., for audit emission) call `.clone()` explicitly — this makes the allocation visible in the code rather than hidden inside the guard. The borrow lifetime is tied to the input `&Actor`, so the compiler guarantees the returned ID reference is never used after the actor is dropped.

---

## 5. Infrastructure Layer

Related files: [src/infrastructure/fake_clock.rs](../src/infrastructure/fake_clock.rs), [src/infrastructure/system_clock.rs](../src/infrastructure/system_clock.rs), [src/infrastructure/in_memory_usage_event_sink.rs](../src/infrastructure/in_memory_usage_event_sink.rs), [src/infrastructure/in_memory_admin_event_sink.rs](../src/infrastructure/in_memory_admin_event_sink.rs), [src/infrastructure/sqlite_event_store.rs](../src/infrastructure/sqlite_event_store.rs), [src/infrastructure/pg_store.rs](../src/infrastructure/pg_store.rs)

**Q: `FakeClock` uses `AtomicI64` with `Ordering::Relaxed`. Why `Relaxed` and not `SeqCst`?**

A: `Relaxed` guarantees atomicity (the read-modify-write is indivisible) but makes no cross-thread memory ordering promises. Here we only need each call to `now()` to return a strictly increasing unique value. There is no cross-field dependency — no other memory write needs to be "published" alongside the timestamp. `SeqCst` would add a full memory fence on every call, which is unnecessary overhead. The correctness argument: even with `Relaxed`, each `fetch_add` returns a unique value because no two calls can get the same return value from an atomic increment.

---

**Q: `InMemoryAdminEventSink` derives `Clone`, but cloning it doesn't copy the buffer. Isn't that surprising?**

A: It is a deliberate and documented choice. The struct holds `Arc<Mutex<Vec<AdminEvent>>>`, and `Clone` on `Arc` increments the reference count (a pointer copy), not the data. This "shared handle" clone semantics is the standard Rust pattern for types you want to share across threads. The composition root exploits this: it creates one sink, clones a handle into each service, and later calls `.snapshot()` on the original to read all events. The doc comment on the struct explicitly states "all clones see the same buffer."

---

**Q: `InMemoryAdminEventSink::snapshot` panics on a poisoned mutex. Is that acceptable?**

A: Yes, per AGENTS.md §3 and the inline doc comment. Mutex poisoning occurs when a thread panics while holding the lock, leaving the protected data in an unknown state. For an append-only audit log, a poisoned mutex means a partial write may have corrupted the event sequence — this is genuinely unrecoverable. Propagating the panic (rather than silently returning empty data or swallowing the error) makes the failure loud and visible, which is the correct behaviour for infrastructure that must never silently lose audit data.

---

**Q: Where would you put a new persistence adapter (e.g. a JSON file-backed repository)?**

A: Add `src/infrastructure/json_file_repo.rs` with a struct (e.g., `JsonFilePassengerRepo`) that implements the relevant port traits. No other layer changes. The composition root in `src/interface/composition_root.rs` would be updated to construct and inject the JSON-backed repo. The application and domain layers are untouched — this is the intended extension point.

---

**Q: `SqliteEventStore` uses `rusqlite`. Why `rusqlite` and not `sqlx` for SQLite?**

A: `rusqlite` is a synchronous, low-overhead binding. The infrastructure layer accesses SQLite on the handler thread (inside the Mutex critical section) so async I/O would gain nothing — the lock is already held while waiting. `sqlx` is async-first and adds compile-time query checking that requires a live DB during `cargo build`; `rusqlite` is simpler for the sync write-through model. The `busy_timeout` pragma is set so concurrent writes park rather than immediately returning `SQLITE_BUSY`.

---

**Q: What SQLite pragmas does `SqliteEventStore` set on open, and why?**

A:
- `PRAGMA journal_mode=WAL` — enables Write-Ahead Logging. WAL allows concurrent readers while a write is in progress; the default `DELETE` journal would block all reads during a write.
- `PRAGMA synchronous=NORMAL` — fsync on every WAL checkpoint rather than every write; a good durability/performance trade-off on local storage.
- `PRAGMA busy_timeout=5000` — if another connection holds a write lock, retry for up to 5 000 ms before returning `SQLITE_BUSY`. Without this, two simultaneous writes could immediately return an error.
- `PRAGMA foreign_keys=ON` — enforces referential integrity on tables that reference each other.

---

**Q: `PgStore` uses `sqlx` with `PgPool`. What is a connection pool and why not open a new connection per request?**

A: A connection pool is a bounded set of pre-opened database connections shared across requests. Opening a PostgreSQL connection involves a TCP handshake, authentication, and session initialisation — typically 20–100 ms. Reusing a pool connection costs only a checkout from a queue (microseconds). `PgPool` in `sqlx` handles connection lifecycle, max-size limits, and health checks automatically.

---

**Q: `SystemClock` is the production clock adapter. What separates it from `FakeClock` at the type level?**

A: Both implement the `Clock` port trait (`fn now(&self) -> Timestamp`). `SystemClock` calls `std::time::SystemTime::now()` and converts to nanoseconds since epoch. `FakeClock` holds an `AtomicI64` that increments on each call. The composition root in `serve.rs` constructs a `SystemClock` for the live server; tests construct `FakeClock::starting_at(0)`. No caller ever sees the concrete type — only the trait — so swapping is a single-line change in the composition root.

---

## 6. Interface Layer — DTOs

Related files: [src/interface/dto.rs](../src/interface/dto.rs), [src/domain/tier.rs](../src/domain/tier.rs), [src/domain/passenger.rs](../src/domain/passenger.rs), [src/domain/resource.rs](../src/domain/resource.rs)

**Q: Why have a `TierDto` enum that mirrors `Tier` exactly? Isn't it just duplication?**

A: The mirror is intentional. It keeps the domain type free of `serde`, `utoipa`, and HTTP-related dependencies. If the wire format ever diverges (e.g., the API renames `"Platinum"` to `"Elite"` for marketing reasons), the DTO changes without touching the domain model. The `From<Tier> for TierDto` and `From<TierDto> for Tier` impls are the conversion bridge — exhaustive `match` blocks mean adding a new tier variant causes a compile error in both directions until both sides are updated.

---

**Q: Request DTOs use `#[serde(deny_unknown_fields)]` but response DTOs do not. Why the asymmetry?**

A: Rejecting unknown fields on **requests** is a security and strictness measure: it prevents clients from sending unexpected fields that might be misinterpreted, and it catches typos early (e.g., `"teir"` instead of `"tier"` would silently be ignored without this flag). On **responses**, adding new fields to a response DTO is a non-breaking API change that clients should tolerate. Applying `deny_unknown_fields` to responses would make no sense — the server controls what it sends.

---

**Q: Response DTOs use `From<&Passenger> for PassengerDto` (borrow) but `From<CrewLeadDto> for CrewLead` takes ownership. Why?**

A: For responses, we are serialising a reference to service-owned data — we don't want to (or need to) take ownership. A borrow-based `From` lets us clone the fields we need without consuming the source. For requests (DTO → domain), we have already parsed the incoming JSON into an owned DTO and can move its fields directly into the domain struct — no clone needed. This is the zero-cost path for inbound data.

---

**Q: `ActorOnlyReq` and `UseResourceReq` both carry an `actor_id: String`. Why not reuse `ActorOnlyReq` inside `UseResourceReq`?**

A: Composition would couple the OpenAPI schema shape of `UseResourceReq` to `ActorOnlyReq` in a way that makes the schema harder to read independently. Both structs are simple two-field structs; the duplication is minimal and each DTO clearly documents its own wire shape without the reader needing to look up a base type. The principle here aligns with AGENTS.md §8: don't introduce abstractions without a concrete reason to reduce coupling.

---

## 7. Interface Layer — HTTP Handlers

Related files: [src/interface/http.rs](../src/interface/http.rs), [src/interface/dto.rs](../src/interface/dto.rs), [src/interface/composition_root.rs](../src/interface/composition_root.rs), [src/bin/serve.rs](../src/bin/serve.rs), [tests/http_access.rs](../tests/http_access.rs), [tests/http_common](../tests/http_common)

**Q: How does the `use_resource` handler avoid borrow-checker errors when it needs both `passengers` and `resources` (immutable) and `access` (mutable) from the same `World`?**

A: It uses **borrow splitting**: the handler destructures `*w` (the `MutexGuard<World>`) into separate named references `let World { ref passengers, ref resources, ref mut access, .. } = *w;`. Rust's borrow checker can track independent borrows of *distinct fields* of a struct, but it cannot see through a method call that takes `&mut self` to the whole struct. Destructuring makes the borrows explicit and disjoint, allowing the compiler to prove no aliasing occurs.

---

**Q: Why does the router apply `DefaultBodyLimit::max(64 * 1024)`?**

A: It defends against oversized request bodies that could exhaust memory or cause slow parsing. All DTOs in this system are tiny (a handful of string fields). A 64 KiB cap is orders of magnitude above the legitimate maximum while still blocking accidental or malicious large payloads. This is an OWASP Top 10 mitigation (denial-of-service via resource exhaustion).

---

**Q: How does `lock_world` work and why does it panic on a poisoned mutex?**

A: `lock_world` calls `state.lock().expect(...)`. The same reasoning as the admin sink applies: a poisoned `World` mutex means a handler panicked while mutating service state, leaving the in-memory collections in an unknown state. Returning an error response from a poisoned-mutex situation would give the client a misleading "try again" signal when the server state is actually corrupted. A panic turns the thread into a 500 response and triggers a restart in production, which is the correct recovery path.

---

**Q: The `/reset` endpoint rebuilds the entire `World`. What is it for and what are its risks?**

A: It is a convenience endpoint for the demo/testing environment — it allows a test harness or a developer to wipe all state and return to a known seed state without restarting the process. The risk in a real deployment would be catastrophic data loss. For this project scope (in-memory, demo-only), it is acceptable. In a production system it would either not exist or be protected by a strong authentication check.

---

**Q: How does request-ID tracing work in the middleware stack?**

A: Two `tower-http` layers are stacked:
1. `SetRequestIdLayer` — assigns a `x-request-id` UUID to every inbound request that doesn't already carry one.
2. `PropagateRequestIdLayer` — copies the request ID onto the outgoing response headers.

Handlers that use `tracing` macros automatically include the request ID in structured log output (via the tracing subscriber). This enables correlating a client-reported request ID with a specific set of log lines — essential for debugging in multi-request environments.

---

**Q: Axum handlers are `async fn`. Does that mean the business logic is async too?**

A: No. The domain and application layers are entirely synchronous. Axum requires handler functions to be `async` because Tokio's scheduler expects them, but the handlers themselves immediately acquire a `Mutex` lock and call synchronous service methods. The `async` is structural overhead from the framework — no `await` points exist inside the business logic path. If the system later needs async I/O (e.g., database calls), the service methods could be made `async` at that point without changing the domain layer.

---

**Q: Why is there a `lock_world` function but no `release_world`?**

A: Because the release is **automatic** — this is Rust's RAII (Resource Acquisition Is Initialization) pattern.

`lock_world` returns a `MutexGuard<'_, World>`. That type implements `Drop`: when the variable holding it goes out of scope, Rust automatically calls its destructor, which releases the mutex. No manual release call exists — or is needed:

```rust
async fn list_passengers(...) -> Json<Vec<PassengerDto>> {
    let w = lock_world(&state);   // lock acquired
    Json(w.passengers.list()...)
}   // ← w dropped here → lock released automatically
```

Forgetting to release a lock is therefore **impossible** — the compiler tracks variable lifetimes and enforces the release at drop time. In C you'd call `pthread_mutex_unlock()` by hand and could forget it; in Rust the type system makes that bug unrepresentable.

The one place where the drop timing matters explicitly is `reset_world`, which uses an inner block to force early release before calling `lock_world` a second time:

```rust
{
    let w = lock_world(&state);   // lock acquired
    // check actor is a valid crew lead …
}   // ← w dropped HERE, lock released
// safe to lock again:
*lock_world(&state) = fresh;      // would deadlock if w above were still live
```

---

## 8. Error Handling

Related files: [src/domain/errors.rs](../src/domain/errors.rs), [src/interface/http.rs](../src/interface/http.rs), [src/interface/dto.rs](../src/interface/dto.rs), [tests/access.rs](../tests/access.rs), [tests/http_access.rs](../tests/http_access.rs)

**Q: Why is `DomainError` marked `#[non_exhaustive]`?**

A: Future spec iterations will add new error variants (e.g., `QuotaExceeded`, `ScheduleConflict`). Without `#[non_exhaustive]`, every external `match` on `DomainError` would be a breaking change. With it, external callers are forced to add a `_ => ...` wildcard arm, making forward-compatibility explicit and compiler-enforced. Internal code in this crate still gets exhaustiveness checking — `#[non_exhaustive]` only relaxes the constraint for code outside the crate.

---

**Q: How does `DomainError` map to HTTP status codes?**

A: The `domain_error_to_response` function in [`src/interface/http.rs`](../src/interface/http.rs) performs the mapping:

| Error variant | HTTP status |
|---|---|
| `UnauthorizedActor` | 403 Forbidden |
| `AccessDenied` | 403 Forbidden |
| `*NotFound` | 404 Not Found |
| `*AlreadyExists` | 409 Conflict |
| `CrewLead*` (limit/minimum/bootstrap) | 400 Bad Request |
| Unknown (wildcard) | 500 Internal Server Error |

The wildcard arm for `#[non_exhaustive]` errors returns 500, which is the safe default when an unexpected error variant appears — it signals "something went wrong" without leaking internal details.

---

**Q: Why is `let _ = result;` forbidden by AGENTS.md?**

A: Silently discarding a `Result` hides errors. In Rust, unused `Result` values trigger a `#[must_use]` warning by default — `let _ = result` suppresses that warning without handling the error. The AGENTS.md rule enforces that every error is either propagated (`?`), matched on, or explicitly handled with a documented rationale. It prevents the category of bug where an audit event or state mutation silently fails.

---

**Q: Access check failures for "not found" entities return an error *without* emitting a usage event. Is that intentional?**

A: Yes, per AC-R2 and AC-R3. A `UsageEvent` requires both a valid `tier_at_attempt` and `min_tier_at_attempt` — these fields are snapshots of the passenger's and resource's state at the moment of the attempt. If either entity does not exist, those fields cannot be populated accurately. The spec is explicit: only the tier-check step (AC-R4) always emits an event (even on deny). Pre-conditions that fail before the check leave the sink empty.

---

## 9. Concurrency & Thread Safety

Related files: [src/interface/http.rs](../src/interface/http.rs), [src/interface/composition_root.rs](../src/interface/composition_root.rs), [src/infrastructure/fake_clock.rs](../src/infrastructure/fake_clock.rs), [src/infrastructure/in_memory_admin_event_sink.rs](../src/infrastructure/in_memory_admin_event_sink.rs)

**Q: The entire `World` is behind a single `Mutex`. What are the concurrency implications?**

A: All mutations are serialised: at any instant only one handler is running business logic. This is correct for an in-memory, demo-scope system — it avoids data races with the simplest possible implementation. The trade-off is throughput under concurrent load: every request waits for the lock, making the server effectively single-threaded.

The PostgreSQL backend path moves beyond this: `WorldShards` replaces the single `Arc<Mutex<World>>` with per-aggregate `RwLock` fields — one per service type. Read-heavy endpoints (e.g. `GET /passengers` while `POST /resources` is in flight) no longer block each other. A documented lock-acquisition order (passengers → resources → access → reporting) prevents deadlocks. See [`docs/postgres-migration-plan.md`](postgres-migration-plan.md) for the full design.

---

**Q: `Arc<Mutex<T>>` vs `RwLock<T>` — why not use `RwLock` for better read throughput?**

A: `RwLock` allows multiple concurrent readers but requires exclusive access for writes. In this system, most endpoints that appear "read-only" (e.g., `GET /passengers`) hold a shared reference to `World` but services mutate internal state indirectly (e.g., emitting audit events on a read changes `next_audit_id`). Mixing `RwLock` read locks with service methods that secretly take `&mut self` would be unsound. Using `Mutex` everywhere is conservative and correct.

---

**Q: `FakeClock` is used in the HTTP server, not just tests. Is that intentional?**

A: Yes, explicitly. The composition root uses `FakeClock` for the demo/development server. The clock starts at 0 and increments by 1 nanosecond per call, so timestamps are deterministic and reproducible for demo purposes. A production system would substitute a `SystemClock` adapter that wraps `SystemTime::now()` or `chrono::Utc::now()` — the composition root would be the only change needed. This is possible precisely because `Clock` is a port trait.

---

**Q: `Arc<T>` is used for the admin sink but not for the usage event sink. Why?**

A: `InMemoryAdminEventSink` needs to be cloned cheaply so both the service and the composition root can hold a handle to the same buffer — the composition root needs the handle to call `.snapshot()` for the `/audit` endpoint. `InMemoryUsageEventSink` is owned directly inside `AccessService` and accessed via `access.sink()` — no second handle is needed because the HTTP handler reaches through the locked `World` to get it. The `Arc` is not needed when a single ownership chain suffices.

---

## 10. Rust Language Specifics

Related files: [src/application/access_service.rs](../src/application/access_service.rs), [src/application/passenger_service.rs](../src/application/passenger_service.rs), [src/application/crew_lead_service.rs](../src/application/crew_lead_service.rs), [src/application/reporting_service.rs](../src/application/reporting_service.rs)

**Q: What does `let Actor::Passenger(passenger_id) = actor else { return Err(...); }` mean?**

A: This is a **let-else** statement (stabilised in Rust 1.65). If the pattern `Actor::Passenger(passenger_id)` matches, `passenger_id` is bound for the rest of the enclosing scope. If it does not match, the `else` block runs — which must diverge (return, panic, break, or continue). It is semantically equivalent to:
```rust
let passenger_id = match actor {
    Actor::Passenger(id) => id,
    _ => return Err(DomainError::UnauthorizedActor),
};
```
The let-else form is preferred because it avoids a nested block and keeps the happy path at the top level.

---

**Q: What is the `?` operator and when is it used?**

A: `?` is shorthand for "propagate the error or unwrap the value." Applied to `Result<T, E>`:
- If the value is `Ok(v)`, unwrap to `v` and continue.
- If the value is `Err(e)`, return `Err(e.into())` from the enclosing function (with an implicit `Into` conversion if needed).

It replaces verbose `match` blocks and makes the happy path linear and readable. In this codebase it appears on every guard call (`require_crew_lead(actor)?`) and every repository lookup (`.ok_or(DomainError::PassengerNotFound)?`).

---

**Q: What is the difference between `Clone` and `Copy` in Rust?**

A: `Copy` types are duplicated **implicitly** on assignment or move — the compiler inserts a bitwise copy. It is only allowed for types with no heap resources (stack-only data). `Clone` is an **explicit** `.clone()` call that may allocate heap memory. Rust requires you to call `.clone()` intentionally so allocations are visible in the code. In this codebase: `Tier`, `Timestamp`, `Outcome` are `Copy` (tiny, stack-only). `PassengerId`, `Passenger`, `UsageEvent` are `Clone`-only (contain `String` which is heap-allocated).

---

**Q: Why does `PassengerService` use `iter_mut()` in `change_tier` but `iter()` in `create`?**

A: `iter()` yields shared references (`&Passenger`), sufficient for read-only operations like checking `p.id == id`. `iter_mut()` yields exclusive references (`&mut Passenger`), needed to mutate a field in-place (`slot.tier = new_tier`). Using `iter_mut()` unnecessarily (when only reading) would require the Vec to be exclusively borrowed, blocking any simultaneous read — Rust would reject it. The compiler enforces the distinction.

---

**Q: `CrewLeadService::replace` uses `enumerate()` for collision detection. Why not just iterate without indices?**

A: The check needs to exclude the slot being replaced. Without the index, the only information available is the element itself — you'd have to compare the old ID against `new_lead.id` for every element, including the slot being replaced, which would incorrectly block in-place updates (same `old_id` and same `new_lead.id`). `enumerate()` provides the index so the collision check can be gated to `i != slot`.

---

**Q: `Vec::position` is used in `replace`. What does it return and what does `let Some(slot) = ... else { ... }` do?**

A: `position` scans the iterator and returns `Option<usize>` — `Some(index)` of the first match, or `None` if no element matches. The let-else destructures the `Option`: if `Some(slot)`, bind `slot` as `usize` for the rest of the function; if `None`, execute the else block (return `Err(CrewLeadNotFound)`). This is the idiomatic Rust alternative to a null-check in imperative languages.

---

**Q: `sort_by` in `top_resources` uses `.then_with(|| ...)`. What is `then_with` and why is the closure lazy?**

A: `Ordering::then_with` applies a secondary comparator only when the primary comparison returns `Equal`. The closure is lazy — it is not evaluated unless the counts are equal. This is a performance optimisation: for large lists, ties may be rare, so avoiding the secondary comparison in the common case is worthwhile. The non-lazy `then` variant takes an `Ordering` value (pre-evaluated), which would compute the secondary sort for every pair regardless of the primary result.

---

**Q: Why does `ReportingService::aggregate_by_tier` use `HashMap::entry().or_default()` instead of a direct insert?**

A: `entry(k).or_default()` returns a `&mut TierCounts` — either the existing entry or a freshly inserted default. It is a single hash lookup: no separate "contains key" check followed by an insert. The alternative (`if map.contains_key(k) { ... } else { map.insert(k, default); }`) would be two lookups and is also not idiomatic Rust. The pre-population loop already ensures every tier exists, so `or_default()` here always finds an existing entry — it is defensive but costs nothing.

---

## 11. Testing Strategy

Related files: [tests/access.rs](../tests/access.rs), [tests/lifecycle.rs](../tests/lifecycle.rs), [tests/reporting.rs](../tests/reporting.rs), [tests/http_access.rs](../tests/http_access.rs), [src/infrastructure/fake_clock.rs](../src/infrastructure/fake_clock.rs), [specs/](../specs)

**Q: Why are tests named with spec IDs like `fn tp_r1_s10_rank_silver_is_one`?**

A: Full traceability. When a test fails in CI, the name immediately points to the spec document, rule, and scenario (e.g., `TP-R1`, scenario 10). A reviewer seeing the failure never needs to open the code to know which business rule broke. It also enforces discipline when writing tests: naming the test forces the author to identify the exact spec item being covered before writing any assertions.

---

**Q: What guarantees does the test suite give about no network, no real filesystem, and no real clock?**

A: All services are constructed with:
- `FakeClock` — in-process deterministic counter, no system calls.
- `InMemoryUsageEventSink` / `InMemoryAdminEventSink` — heap-only, no file I/O.
- No `TcpListener` or file paths in unit/integration tests.

HTTP tests use `tower::ServiceExt::oneshot` or `axum_test::TestClient`, which drives the router in-process without binding a real port. The compiler enforces this indirectly: any attempt to use a type that accesses the filesystem or network in the domain layer would require importing a crate that is not in `Cargo.toml`.

---

**Q: What is the "Red → Green → Refactor" workflow mentioned in AGENTS.md and how is it applied here?**

A: The workflow:
1. **Red**: Write a test named after the spec scenario. It fails because the implementation does not exist.
2. **Green**: Write the minimum code to make the test pass. No cleanup.
3. **Refactor**: Improve code quality (extract helpers, rename, simplify) while keeping tests green.
4. **Commit**: One commit per spec ID (e.g., `test(access): AC-S7 denied access emits event`).

Applied in this codebase: each spec file (`specs/05-access.md`) has numbered scenarios (`AC-S1..S10`), each scenario has a corresponding test in `tests/access.rs`, and each test was committed after going green.

---

**Q: Integration tests in `tests/` share no state between test cases. How is isolation achieved?**

A: Each test function builds its own services from scratch:
```rust
let clock = FakeClock::starting_at(0);
let mut svc = PassengerService::new(clock);
```
There are no globals, no `static mut`, and no shared mutable state between tests. Cargo/nextest runs tests in parallel processes — even if two tests happened to use the same variable name, they run in separate address spaces. FakeClock's start value can be set per test, making timestamps predictable regardless of execution order.

---

**Q: Why does the test file `access.rs` define a local `admin()` helper returning `Actor::CrewLead(...)`?**

A: It is a test fixture that avoids repeating the same construction in every test body. `admin()` returns a specific, named crew lead actor used whenever the test needs an actor that should be rejected by `use_resource` (AC-R1). Giving it a named helper makes tests self-documenting: `admin()` clearly means "a crew lead actor, which is invalid for resource access." A plain inline `Actor::CrewLead(CrewLeadId("aria".into()))` would be equally correct but less readable across ten tests.

---

**Q: AGENTS.md says `cargo nextest` is preferred over `cargo test`. What is the difference?**

A: `cargo nextest` is a third-party test runner (`cargo-nextest`); `cargo test` is the built-in one.

| | `cargo test` | `cargo nextest` |
|---|---|---|
| **Execution model** | Tests within one binary run in threads — shared process, shared static state | Each test runs as a **separate process** — true isolation, no state leaks between tests |
| **Speed** | Single-threaded per binary | Runs all tests in parallel across processes; typically 2–3× faster on multi-core |
| **Output** | All output interleaved; hard to read on failure | Clean per-test output, failures shown with full context, progress bar |
| **Failure isolation** | A panic in one test can corrupt or hang others in the same binary | A panic kills only that test process; all others continue |
| **Retry** | None built-in | `--retries N` for flaky tests |
| **Filtering** | `cargo test foo` substring match | `-E 'test(foo)'` — full filter expression language |
| **CI output** | No JUnit by default | `--profile ci` emits JUnit XML natively |

**The one trade-off:** `cargo nextest` does not run `doctests` — those still need `cargo test --doc`. That is why AGENTS.md says "preferred" rather than "required."

For this repo, `nextest` is preferred because test names include spec IDs (e.g., `ac_r1_s7_denied_emits_event`) and nextest's clean per-test output makes it immediately obvious which spec scenario failed without scrolling through interleaved output.

---

## 12. Security & Input Validation

Related files: [src/interface/dto.rs](../src/interface/dto.rs), [src/interface/http.rs](../src/interface/http.rs), [src/application/guards.rs](../src/application/guards.rs), [deny.toml](../deny.toml), [AGENTS.md](../AGENTS.md)

**Q: Where is input validation enforced and why there specifically?**

A: At the **interface boundary** only:
- `#[serde(deny_unknown_fields)]` on every request DTO rejects payloads with unexpected keys.
- `TierDto` → `Tier` via `From` is always valid (the DTO enum mirrors the domain enum exactly), but `TryFrom<&str> for Tier` in the domain validates raw string input if it ever arrives that way.
- `DefaultBodyLimit` blocks oversized bodies.
- Path and query parameters are parsed at the handler level; malformed values return 400 before reaching services.

Keeping validation at the boundary follows AGENTS.md §5: once input passes the boundary, it is represented as a domain type that is valid by construction. Defensive re-validation deep in the stack would be dead code.

---

**Q: The `Actor` type carries no authentication token. Is that a security gap?**

A: No longer a gap. The `AuthActor` extractor in `src/interface/http.rs` reads `Authorization: Bearer <token>`, resolves the token against `PRMS_API_KEYS` (a `HashMap<String, String>` built at startup), and returns 401 for missing or unknown tokens. The actor ID is derived from the token — the request body no longer carries `actor_id`. Tokens are compared using `subtle::ConstantTimeEq` to prevent timing-based token enumeration (OWASP A07). Adding stronger authentication (signed JWTs, session store, RBAC) would mean adding a new port trait (e.g., `AuthProvider`) and updating the extractor — the domain layer would not change.

---

**Q: Why is there a `/reset` endpoint and what risk does it carry?**

A: It is a demo/development convenience that rebuilds the entire `World` to seed state. In a production system it would be a catastrophic data-loss endpoint and should either not exist or be protected by a strong authentication layer. For this project (in-memory, no persistence), the risk is limited to disrupting a running demo session. It is acceptable at this scope but would need to be removed or gated before any real deployment.

---

**Q: HTTP responses include an `ErrorBody` struct. What fields does it expose and why not expose the raw Rust error message?**

A: `ErrorBody` exposes `code` (a machine-readable string like `"AccessDenied"`) and `message` (a human-readable description). The `message` comes from `thiserror`'s `#[error("...")]` attribute, which is a fixed string per variant — it does not include stack traces, file paths, or internal state. This follows the principle of not leaking implementation details in error responses (OWASP A05 Security Misconfiguration). A 500-level error returns a generic message, not the raw panic message.

---

**Q: What OWASP Top 10 risks are explicitly mitigated in this codebase?**

| Risk | Mitigation |
|---|---|
| A03 Injection | `deny_unknown_fields`, strict `TryFrom` parsing, no dynamic SQL |
| A04 Insecure Design | Layered architecture, domain invariants enforced by types |
| A05 Security Misconfiguration | No secrets in code, `deny_unknown_fields`, body size limit |
| A06 Vulnerable Components | `cargo deny` (`deny.toml`), pinned toolchain |
| A08 Software & Data Integrity | Append-only event logs, snapshot fields that cannot be rewritten |
| A09 Logging & Monitoring | Every access attempt emits a `UsageEvent`; request IDs on all responses |

---

## 13. Design Patterns

Related files: [src/application/ports.rs](../src/application/ports.rs), [src/application/access_service.rs](../src/application/access_service.rs), [src/application/passenger_service.rs](../src/application/passenger_service.rs), [src/interface/composition_root.rs](../src/interface/composition_root.rs), [AGENTS.md](../AGENTS.md)

**Q: What is the Repository pattern and where is it applied?**

A: The Repository pattern abstracts persistence behind an interface, decoupling business logic from storage details. Here, `PassengerService` stores `active: Vec<Passenger>` and `deleted: Vec<Passenger>` internally, and port traits (`UsageEventSink`, `UsageEventSource`) are the repository interfaces for events. The pattern's payoff: swapping from in-memory to database storage only requires a new infrastructure adapter — no application or domain code changes.

---

**Q: What is the Strategy pattern and how does it appear in the access policy?**

A: `Tier::can_access(resource_min_tier) -> bool` is a strategy: it encapsulates the access rule in a single, swappable location. If the policy changed (e.g., Gold can access Platinum in some contexts), you would modify or extend `can_access` — or introduce a `AccessPolicy` trait with multiple implementations. The current implementation is a simple rank comparison, but the method boundary allows that to change without touching `AccessService`.

---

**Q: What is the Builder pattern and where is it used?**

A: The Builder pattern constructs complex objects step-by-step. It appears in `PassengerService::new(clock).with_audit(sink)` and `ResourceService::new(clock).with_audit(sink)`. `new(clock)` creates a minimal service; `with_audit(sink)` adds optional audit capability. Each method consumes `self` and returns `Self`, enabling method chaining. The pattern avoids constructor parameter explosion (the alternative would be a single constructor with optional parameters, making call sites verbose with `None` arguments).

---

**Q: The codebase avoids the service-locator pattern. What is that pattern and why is it avoided?**

A: A service locator is a global registry where code calls `ServiceLocator::get::<PassengerService>()` to retrieve dependencies at runtime. Problems: dependencies are invisible in function signatures (hidden coupling); testing requires resetting global state; the locator becomes a catch-all dependency magnet. This codebase uses explicit constructor injection instead — every service declares its dependencies as constructor parameters, making all dependencies visible, type-checked, and testable without global state.

---

**Q: The codebase mentions "do not use trait objects when a plain enum suffices." When does this guidance apply?**

A: Use a plain enum when the set of variants is **closed** (known at compile time) and you don't need external code to add new variants. Example: `Tier` is a closed set of 3 variants — a trait object `Box<dyn MembershipLevel>` would add heap allocation, dynamic dispatch, and open-endedness where none are needed. Use a trait object when the set is **open** (plugins, future extensions) or when you need runtime polymorphism across unrelated types. `AdminEventSink` and `Clock` are trait objects because the set of adapters (in-memory, file-backed, database-backed) is open.

---

**Q: What does SOLID mean in the context of this codebase?**

| Principle | Application |
|---|---|
| **S**RP | One service per aggregate (`PassengerService`, `ResourceService`, etc.); domain types hold no orchestration logic |
| **O**CP | New tier variant: add the enum arm, compiler finds every non-exhaustive match. New adapter: add a struct in `infrastructure/`, no existing code changes |
| **L**SP | Trait implementors (`FakeClock`, `InMemoryAdminEventSink`) honour their contracts — no surprising side effects |
| **I**SP | Small, focused traits (`UsageEventSink` vs `UsageEventSource`); services depend only on what they use |
| **D**IP | Services depend on port traits (defined in `application/`), not on concrete adapters; wiring happens in the composition root |

---

---

## 14. Full-Stack — REST API Design

Related files: [src/interface/http.rs](../src/interface/http.rs), [src/interface/dto.rs](../src/interface/dto.rs), [src/bin/serve.rs](../src/bin/serve.rs), [tests/http_access.rs](../tests/http_access.rs), [README.md](../README.md)

**Q: Is the REST API design consistent with HTTP conventions? Walk through a few endpoints.**

A: Mostly yes:
- `GET /passengers` → 200 list; `POST /passengers` → 201 Created (creates resource, should ideally return 201 not 200, worth checking).
- `PATCH /passengers/{id}/tier` — uses PATCH (partial update) correctly rather than PUT (full replacement).
- `DELETE /passengers/{id}` — maps to soft-delete, returns 200 with the deleted entity. A purist would argue DELETE should return 204 No Content, but returning the deleted state is useful for client confirmation.
- `POST /access` — correct; the access attempt is a side-effectful action (creates a `UsageEvent`) so POST is appropriate, not GET.
- `/reset` as `POST` is reasonable since it has a side effect (state mutation), though it is a demo-only endpoint.

---

**Q: The `/crew-leads/{id}` route uses `PUT` for replace. Is that semantically correct?**

A: Yes. `PUT` is idempotent and means "replace the resource at this URI with the given representation." `CrewLeadService::replace` atomically swaps the old lead for a new one — calling it twice with the same body is safe (second call would find `old_id` missing and return `CrewLeadNotFound` or succeed if `new_lead` is already in place). The HTTP verb matches the operation. `PATCH` would be more appropriate only if the payload expressed a *partial* update (e.g., changing the name only).

---

**Q: The API returns `DomainError` codes as strings in the `ErrorBody`. What are the trade-offs of this design?**

A: Advantages: machine-readable codes (`"PassengerNotFound"`) allow clients to branch on error type without parsing a human-readable message. The TypeScript frontend's `KNOWN_CODES` set consumes these directly, mapping to the same `DomainError` union type. Disadvantages: the error codes are now part of the public API contract — renaming a Rust variant (e.g., `PassengerNotFound` → `PassengerMissing`) is a breaking change for all clients. In a production API you would version these codes separately from the Rust source, or define them in an OpenAPI spec as a string enum.

---

**Q: The API exposes an OpenAPI spec at `/openapi.json` via utoipa. What value does this provide?**

A: It provides a machine-readable, self-documenting contract that clients can use to auto-generate HTTP client code (e.g., TypeScript SDK from `openapi-generator`), validate request/response shapes, and power interactive documentation (Swagger UI, Redoc). For a full-stack project it closes the contract loop: the Rust server owns the spec, and the frontend can be generated or validated against it rather than maintaining types manually. The current frontend writes its API types by hand (`api.ts`) — generating them from `/openapi.json` would be the next step.

---

**Q: `openapi.json` doesn't appear in the repository as a committed file. Is it missing?**

A: No — it is a derived artifact, not source. `utoipa`'s `#[derive(OpenApi)]` on the empty `ApiDoc` struct compiles every `#[utoipa::path(...)]` annotation across all handlers into a static in-memory spec at build time. The `openapi_json` handler returns it on every `GET /openapi.json` request:

```rust
async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())   // generated method from the derive macro
}
```

To inspect it while the server is running:

```bash
cargo run --features http --bin serve
curl http://127.0.0.1:8080/openapi.json | jq
```

Committing a static copy would create a second source of truth that silently drifts whenever handler annotations change. If a CI pipeline needs a committed file for type generation (e.g., `openapi-typescript`), the idiomatic approach is a build script or `cargo xtask` that starts the server, fetches `/openapi.json`, and writes it as a CI artifact — not a manually maintained static file. Note also that `env!("CARGO_PKG_VERSION")` in the `info` block embeds the version from `Cargo.toml` at compile time, so the served spec is always version-accurate without manual updates.

---

**Q: What HTTP status code does the API return when a `DomainError::AccessDenied` occurs vs `DomainError::UnauthorizedActor`? Are those the right codes?**

A: Both map to `403 Forbidden`. The distinction:
- `UnauthorizedActor`: the actor's *role* is wrong (crew lead trying to use a resource). This is closer to `403` (authenticated but not authorised).
- `AccessDenied`: the passenger's *tier* is too low. This is also `403` — the actor is identified but lacks the required permission level.

Using `401 Unauthorized` would be wrong here because 401 means "you need to authenticate first" — in this system all actors are pre-identified. `403` is correct for both cases.

---

**Q: There is no pagination on `GET /passengers`, `GET /resources`, or `GET /usage`. When would that become a problem?**

A: As soon as the dataset grows beyond a few thousand records, unbounded list endpoints cause:
1. Large JSON payloads that slow client rendering.
2. High memory pressure on the server (serialising the entire collection).
3. Long response times that degrade perceived performance.

The fix is cursor-based or offset-based pagination (`?limit=50&cursor=...`). For the current in-memory demo scope (single-flight, small roster) this is acceptable, but it should be flagged as a production concern. The usage event log is the highest-risk endpoint — every `POST /access` adds an event and the log is never truncated.

---

**Q: The API uses snake_case field names in JSON (`passenger_id`, `min_tier`). Is that a convention choice?**

A: Yes, it aligns with the Rust serde defaults (which mirror snake_case struct field names by default). JSON APIs commonly use either `camelCase` (JavaScript convention) or `snake_case` (Python/Go convention). The choice here is internally consistent — all endpoints use snake_case. The TypeScript frontend maps these to camelCase in its own type definitions (e.g., `minTier`, `deletedAt`). In a production API you would document this in the OpenAPI spec and ensure all clients follow it.

---

## 15. Full-Stack — TypeScript / React Frontend

Related files: [web/src/App.tsx](../web/src/App.tsx), [web/src/domain/tier.ts](../web/src/domain/tier.ts), [web/src/domain/ids.ts](../web/src/domain/ids.ts), [web/src/services/accessService.ts](../web/src/services/accessService.ts), [web/src/services/world.ts](../web/src/services/world.ts), [web/vite.config.ts](../web/vite.config.ts)

**Q: The frontend has its own `web/src/domain/tier.ts` that mirrors `src/domain/tier.rs`. How is spec parity maintained?**

A: Both files implement the same `rank()` function and `canAccess()` comparison, with the same variant names (`Silver`, `Gold`, `Platinum`). The spec (`specs/01-tier-policy.md`) is the single source of truth — both implementations are derived from it. Parity is maintained by test coverage: the Rust tests cover the Rust implementation and Vitest covers the TypeScript one. If the spec changes, both must be updated. A more robust approach would be code generation: generate TypeScript types from the Rust enums at build time (e.g., via `ts-rs` crate), eliminating the manual mirror entirely.

---

**Q: TypeScript `type Brand<T, B>` is used for `PassengerId`, `ResourceId`, and `CrewLeadId`. What problem does this solve?**

A: JavaScript/TypeScript has no runtime newtype concept — all IDs are plain strings at runtime. The `Brand` trick attaches a phantom type tag via `declare const brand: unique symbol`, which exists only at the type-checker level. This means passing a `ResourceId` where a `PassengerId` is expected is a TypeScript compile error, mirroring the Rust newtype protection. The runtime cost is zero — the brand is erased after type-checking. The factory functions (`passengerId("p-001")`) perform the cast, keeping the branded type as the only way to create valid IDs.

---

**Q: `ManualClock` in the frontend is not thread-safe (it uses a plain mutable field). Is that a problem?**

A: No — JavaScript is single-threaded (one main thread, no shared mutable memory across Web Workers). There is no concurrent access to `ManualClock.current`, so a plain increment (`this.current += 1`) is safe. The `AtomicI64` used in the Rust `FakeClock` is necessary because Rust's test runner and Tokio spawn real OS threads. The difference in implementation reflects the runtime environment, not a design inconsistency.

---

**Q: The React app uses `useState` for `version` to trigger re-renders rather than storing services in state directly. Why?**

A: Services (`PassengerService`, `AccessService`, etc.) are mutable class instances — they are not serialisable plain objects. React's `useState` diffing would never detect a mutation to a class instance (same reference, changed contents). The pattern in `store.tsx` is:
1. Hold the `World` in a stable `useState` ref (never replaced, mutated in-place).
2. Hold a `version: number` in a second `useState` (incremented on every mutation via `mutate()`).
3. Components that call `useStore()` re-render when `version` changes, not when `world` changes.

This is a **manual subscription** pattern — simpler than Redux but brittle at scale (every mutation must go through `mutate()` or the UI goes stale).

---

**Q: `StoreContext` is `createContext<StoreApi | null>(null)` and `useStore` throws if called outside `StoreProvider`. Is that the right guard?**

A: Yes. The `null` default makes the missing-provider case detectable at runtime (instead of silently using an empty/broken context). The `throw new Error(...)` in `useStore` provides a clear, actionable error message rather than a cryptic `Cannot read properties of null`. This is the standard TypeScript/React pattern when a context requires a provider — the alternative (`createContext<StoreApi>(DEFAULT_VALUE)`) would require a meaningful default world, which doesn't exist here.

---

**Q: The `App.tsx` has both an in-browser TypeScript world and a `LiveServerPanel` that talks to the Rust server. They have independent state. What is the purpose of running both?**

A: It is an end-to-end validation tool. The TypeScript services are the "reference implementation" running in the browser — they should always behave identically to the Rust server. A developer can perform the same operations in both panels and compare results. If they diverge, either the TypeScript port has a bug or the Rust server has a regression. It also demonstrates the architecture's core claim: the business rules are portable enough to be re-implemented in TypeScript without changing the spec.

---

**Q: The Vite dev server proxies `/api/*` to `127.0.0.1:8080`. What problem does this solve and what happens in production?**

A: It solves the **same-origin policy** problem: browsers block XHR/fetch requests to a different origin (different host, port, or scheme) unless the server sends the appropriate CORS headers. By proxying `/api/*` through the Vite dev server (port 5173), the browser sees all requests as same-origin. In production, the same is achieved either by: (a) serving the built frontend static files from the same Rust server process, or (b) deploying behind a reverse proxy (nginx, Caddy) that routes `/api/*` to the Rust binary. The Rust server also has a CORS layer (`CorsOrigins::Any` for demo), which covers scenario (b).

---

**Q: Coverage in `vite.config.ts` excludes `src/components/**` but includes `src/**/*.ts`. What does this tell you about the testing philosophy?**

A: Components are excluded from coverage measurement — they are React UI elements whose correctness is harder to assert with unit tests (rendering, event handling, visual layout). The included `src/**/*.ts` (pure TypeScript: `domain/`, `services/`) is where the business logic lives — `tier.ts`, `accessService.ts`, etc. This mirrors the Rust approach: domain and application layers are rigorously unit-tested; the interface layer (HTTP handlers / React components) is tested at the integration level (HTTP tests / manual browser testing). Coverage metrics are most meaningful where the logic is pure.

---

## 16. Full-Stack — Frontend ↔ Backend Contract

Related files: [web/src/services/api.ts](../web/src/services/api.ts), [web/src/domain/errors.ts](../web/src/domain/errors.ts), [src/interface/dto.rs](../src/interface/dto.rs), [src/interface/http.rs](../src/interface/http.rs), [web/vite.config.ts](../web/vite.config.ts)

**Q: The TypeScript `api.ts` file defines `ApiPassenger`, `ApiResource`, etc. as manual type declarations. What is the risk and how would you eliminate it?**

A: The risk is **contract drift** — if the Rust server changes a field name (e.g., `min_tier` → `minimum_tier`) or adds a required field, the TypeScript types go stale silently. The API call succeeds at runtime (JSON parses fine) but the frontend reads `undefined` instead of the field value. Eliminating the risk:
1. **Generate TypeScript types from the OpenAPI spec** served at `/openapi.json` using `openapi-typescript` or `openapi-generator`. Types are always in sync with the server.
2. **Runtime validation** (e.g., `zod`) at the API boundary — validate the response against a schema and fail loudly on mismatch rather than propagating `undefined`.

---

**Q: The frontend `DomainError` type is a TypeScript union that mirrors the Rust `DomainError` enum. What happens when a new Rust error variant is added?**

A: The TypeScript union does not automatically update. API calls that return the new error code fall through to the `"Unknown"` catch-all in `toDomainError()`. The frontend shows a generic error message rather than the specific one. The fix is the same as above: generate the error codes from the OpenAPI spec, where the Rust utoipa annotations define the closed set of error codes as a string enum. Until then, adding a new error variant requires a manual update in `web/src/domain/errors.ts` and `api.ts`.

---

**Q: The `VITE_API_BASE` env var determines where the frontend sends API requests. How would you configure this for staging vs production?**

A: Vite's env vars are resolved at **build time**, not runtime. You would:
1. Create `.env.staging` with `VITE_API_BASE=https://staging.api.example.com`.
2. Create `.env.production` with `VITE_API_BASE=https://api.example.com`.
3. Run `vite build --mode staging` or `vite build` (production is the default).

The built static files contain the baked-in URL. For a runtime-configurable URL (e.g., the same build artefact deployed to multiple environments), you would inject a `window.__CONFIG__` object from the server at page load time and read it in `getBase()` instead of `import.meta.env`.

---

**Q: The frontend `AccessService.useResource` checks `actor.id !== passengerId` (AC-R6). The Rust backend does not have this check. Is there a discrepancy?**

A: Yes, and it is worth raising. The Rust `AccessService` checks `Actor::Passenger(passenger_id)` — it only verifies the actor is a passenger, not that the actor's ID matches the `passenger_id` parameter (because the HTTP API derives both from the same request body). The TypeScript service adds a stricter check that the actor is acting on their own behalf. This could either be a spec gap (AC-R6 should be added to the spec) or an over-strict TypeScript interpretation. The right answer is to open the spec file, clarify the rule, and align both implementations.

---

## 17. Full-Stack — State Management

Related files: [web/src/state/store.tsx](../web/src/state/store.tsx), [web/src/state/storeContext.ts](../web/src/state/storeContext.ts), [web/src/state/useStore.ts](../web/src/state/useStore.ts), [web/src/services/world.ts](../web/src/services/world.ts)

**Q: The frontend uses React Context + manual version bumping instead of a dedicated state library (Redux, Zustand, Jotai). When would you reach for a library instead?**

A: The current approach works for a demo-scale app with a small, predictable mutation surface (`mutate()` wraps every write). You would reach for a library when:
- **Many components** need fine-grained subscriptions (context re-renders the entire tree on every `version` bump — wasteful if only one panel changed).
- **Async operations** (loading states, error states per request) need to be managed — a plain `version` counter can't express "passengers are loading" vs "passengers failed to load."
- **DevTools** become important — Redux DevTools or Zustand's devtools middleware give time-travel debugging, which is valuable when tracking down subtle UI bugs.
- **Optimistic updates** are needed — showing the result before the server confirms, then rolling back on error.

For this scope, the custom context is fine and has zero external dependencies.

---

**Q: State is held entirely in-memory in both the React frontend and the Rust backend. What are the consequences of a page refresh or server restart?**

A: All state is lost — by design. The `App.tsx` header says "state lives in memory · refresh to reset." For a demo system this is acceptable. In a real product:
- Frontend state would be persisted in `localStorage` or `IndexedDB` for session continuity, or derived from server responses.
- Backend state would be persisted to a database; the server would be stateless (load-balanced replicas all read/write the same DB).
- The `/reset` endpoint would not exist (or would be strongly authenticated).

---

**Q: Components subscribe to `world` and `version` via `useStore()`. Could this cause unnecessary re-renders?**

A: Yes. Every mutation (any panel's button click) increments `version`, which causes every component that calls `useStore()` to re-render — even panels unaffected by the change. For a demo with ~7 panels this is imperceptible. At scale, the fix is to use a selector pattern: `useStore(s => s.world.passengers)` so only components that depend on passengers re-render when passengers change. Libraries like Zustand and Jotai build this in; with the current custom context you would need to implement selector memoisation manually with `useMemo` + structural equality checks.

---

## 18. Full-Stack — Observability & Operability

Related files: [src/bin/serve.rs](../src/bin/serve.rs), [src/interface/http.rs](../src/interface/http.rs), [Cargo.toml](../Cargo.toml)

**Q: The HTTP server assigns a `x-request-id` UUID to every request. How would you use this in practice?**

A: Every log line emitted by a handler should include the request ID in the structured log context (via `tracing::Span`). When a bug report comes in with a `x-request-id` from a client's response header, you can grep the log aggregator (Datadog, Loki, CloudWatch) for that exact ID and see the full trace: which endpoint was called, what parameters were received, what service calls were made, and what error (if any) was returned. Without request IDs, correlating a single request's log lines in a multi-request log stream is impractical.

---

**Q: What observability features are already in the production build?**

A:
1. **Prometheus metrics** (`GET /metrics`) — gauges for crew-lead/passenger/resource counts; counters for usage events (allowed vs denied) and admin events. Scraped directly by Prometheus or compatible agents.
2. **Readiness probe** (`GET /health/ready`) — returns JSON with entity counts, a DB liveness ping result, and 503 if any lock is poisoned. Distinct from `GET /health` (liveness only).
3. **Request ID tracing** — `x-request-id` UUID on every request/response; propagated into `tracing::Span` so log lines are correlated.
4. **Structured logging** — `--log-format json` emits newline-delimited JSON suitable for Loki, Datadog, or CloudWatch ingestion.

**Q: What observability is still absent for a full production setup?**

A:
1. **Latency histograms** — the current `/metrics` tracks counts, not latency distributions (p50/p95/p99). `axum-prometheus` or manual `Histogram` instruments would fill this gap.
2. **Distributed tracing** — OpenTelemetry spans to propagate trace context from the React frontend through the Rust server to the database.
3. **Alerting thresholds** — SLO definitions (e.g., p99 < 200 ms, error rate < 0.1%) backed by alertmanager rules.

---

**Q: `tracing-subscriber` reads `RUST_LOG` for log level configuration. How would you manage this across environments?**

A: Set `RUST_LOG` as an environment variable per deployment:
- `RUST_LOG=info` for production (info + warn + error).
- `RUST_LOG=debug` for staging or local debugging.
- `RUST_LOG=passenger_resource_management=trace` to trace only this crate without noisy framework output.

In a containerised deployment (Docker/Kubernetes) the env var is injected via a ConfigMap or secret. The `clap` `--bind`/`--cors-origins` pattern in `serve.rs` already demonstrates the env-var-as-config approach — `RUST_LOG` extends this naturally.

---

## 19. Full-Stack — Performance & Scalability

Related files: [src/application/passenger_service.rs](../src/application/passenger_service.rs), [src/application/resource_service.rs](../src/application/resource_service.rs), [src/application/reporting_service.rs](../src/application/reporting_service.rs), [src/interface/http.rs](../src/interface/http.rs), [web/src/services/reportingService.ts](../web/src/services/reportingService.ts)

**Q: All service data is in a single `Vec` per entity type. What is the time complexity of `list()`, `get()`, and `soft_delete()`?**

A: 
- `list()` — O(1), just returns a slice reference.
- `get(id)` — O(n), linear scan through `active` then `deleted`.
- `soft_delete(id)` — O(n) to find + O(n) for `Vec::remove` (shifts elements). Effectively O(n).
- `create(id)` — O(n) for the duplicate check.

For dozens of passengers/resources (current scope) this is fine. At tens of thousands of records, `get` and `create` become bottlenecks. The fix is to maintain a `HashMap<Id, usize>` index alongside the `Vec` for O(1) lookups — a standard trade-off of memory for speed.

---

**Q: The Rust server is single-threaded under the Mutex. How many concurrent requests can it handle?**

A: Theoretically many — Tokio can accept many connections concurrently — but they are all serialised at the Mutex. Practically, throughput is bounded by the time each handler holds the lock (typically microseconds for in-memory operations). For a demo this is fine. For load testing: if each request holds the lock for 100 µs, the maximum throughput is ~10,000 requests/second on a single core. Real bottlenecks under high concurrency would be measured with `wrk` or `k6` before optimising.

---

**Q: The frontend `top_resources` report sorts the entire usage event array on every render. Is that a performance concern?**

A: For a demo with hundreds of events, no. For millions of events in a production reporting service, sorting in the browser is impractical. The correct approach is to move aggregation to the server and cache or incrementally maintain the sorted result — the Rust `ReportingService::top_resources` already does this on the server side. The frontend `ReportingService` is only used when operating in offline/in-browser mode; the `LiveServerPanel` fetches from `/reports/top-resources` directly.

---

**Q: How would you add caching to the `GET /reports/top-resources` endpoint?**

A: Options in increasing sophistication:
1. **In-process memoisation**: Cache the result in the `World` struct, invalidate on every `POST /access`. Simple but only works for a single-process server.
2. **HTTP caching headers**: Return `Cache-Control: max-age=60` to let clients and CDN proxies cache the response for 60 seconds. Zero server-side code change, but stale by up to 60 seconds.
3. **Redis / external cache**: Cache the computed result keyed by (e.g.) the last event ID. Any process in a cluster can read it; invalidated when the event log grows.

The right choice depends on how stale the report can be (SLO) and whether the server is replicated.

---

## 20. Full-Stack — Deployment & DevOps

Related files: [Cargo.toml](../Cargo.toml), [rust-toolchain.toml](../rust-toolchain.toml), [deny.toml](../deny.toml), [README.md](../README.md), [web/package.json](../web/package.json)

**Q: The `deny.toml` file bans wildcard version specs and audits known vulnerability advisories. What does this protect against?**

A: 
- **`wildcards = "deny"`**: Wildcard version constraints (`"*"`) mean "any version", which would allow a `cargo update` to silently pull in a breaking or vulnerable version. Pinned constraints make upgrades explicit and auditable.
- **`advisories`**: `cargo-deny` checks every dependency against the RustSec Advisory Database. A dependency with a known CVE causes CI to fail, forcing a remediation decision before it ships.
- **`licenses`**: Only explicitly allowed SPDX licences pass (MIT, Apache-2.0, BSD-3-Clause). This prevents accidentally shipping GPL-licensed code in a proprietary product.

---

**Q: The `rust-toolchain.toml` pins the Rust version. Why is this important in a team environment?**

A: Without pinning, each developer and CI runner uses whatever `rustup` version happens to be active. Clippy lint rules, edition semantics, and language features can differ across versions — a lint that passes on one developer's machine fails in CI. Pinning ensures the entire team compiles with the same compiler, the same edition rules, and the same clippy version. It also makes upgrades deliberate: bumping the toolchain file is a tracked change, not an implicit side effect of running `rustup update`.

---

**Q: How would you containerise this application for deployment?**

A: A two-stage Dockerfile:

```dockerfile
# Stage 1: build
FROM rust:1.82-slim AS builder
WORKDIR /app
COPY . .
RUN cargo build --release --features http --bin serve

# Stage 2: minimal runtime image
FROM debian:bookworm-slim
COPY --from=builder /app/target/release/serve /usr/local/bin/serve
EXPOSE 8080
ENV PRMS_BIND=0.0.0.0:8080
CMD ["serve"]
```

The two-stage build keeps the final image small (no Rust toolchain in production). The binary is statically linked by default on Linux with `musl` or dynamically links `libc` on `glibc` distros. The frontend would be built separately (`npm run build`) and served as static files either from the same container (Nginx sidecar) or a CDN.

---

**Q: There is no CI configuration file (`.github/workflows/`, `.gitlab-ci.yml`) in the workspace listing. What would you add?**

A: `.github/workflows/ci.yml` is present and runs:
1. `cargo fmt --check` — reject unformatted code.
2. `cargo clippy --all-targets -- -D warnings` and `cargo clippy --all-targets --features http -- -D warnings -W clippy::pedantic` — zero lint warnings.
3. `cargo nextest run` (default) + `cargo llvm-cov nextest --features http --fail-under-lines 96` — all tests pass, 96%+ coverage gated.
4. `cargo deny --all-features check advisories bans licenses sources` — supply-chain audit.
5. `npm run lint && npm run typecheck && npm run build` — TypeScript and frontend checks.
6. Playwright E2E job that starts the Rust server, seeds state, and runs the full browser test suite.

Each check is a separate job; the E2E job depends on the `rust` and `web` jobs.

---

## 21. Full-Stack — Product & Tradeoff Thinking

Related files: [docs/plan-passengerResourceManagement.prompt.md](plan-passengerResourceManagement.prompt.md), [specs/](../specs), [src/application/crew_lead_service.rs](../src/application/crew_lead_service.rs), [src/application/access_service.rs](../src/application/access_service.rs)

**Q: [TRADEOFF] What are the good and bad points of using Rust as a backend language?**

A: **Good points:**

- **Memory safety without GC** — no GC pauses, no null-pointer crashes, no data races; caught at compile time. `#![forbid(unsafe_code)]` in the domain layer makes this explicit.
- **Performance** — comparable to C/C++; zero-cost abstractions mean the clean layering (ports, generics, trait objects) compiles down to efficient machine code.
- **Compile-time thread-safety** — `Send`/`Sync` traits in `src/application/ports.rs` mean the compiler verifies that types are safe to share across threads. This codebase uses per-aggregate `RwLock` shards (one per service) so concurrent reads on different aggregates proceed without blocking. The compiler ensured correctness at every step of the refactor from the original single `Mutex<World>`. The traits guarantee that if you later move to even finer-grained locking, every unsound sharing is a compile error rather than a race condition caught in production.
- **Exhaustive pattern matching** — adding a new `DomainError` variant forces every `match` to handle it; no silent fallthrough. Maps directly to the "closed enum" spec rules (TP-R4, AU-R3).
- **`Result<T, E>` instead of exceptions** — errors are explicit, typed, and traced to spec IDs. The compiler won't let you silently ignore one.
- **Tiny binaries and fast startup** — no runtime VM; the whole server ships as a single binary, easy to containerise.
- **`cargo` toolchain** — testing (`nextest`), linting (`clippy`), formatting (`rustfmt`), and coverage (`llvm-cov`) all in one tool, enforced in CI.

**Bad points:**

- **Steep learning curve** — ownership, borrowing, and lifetimes take weeks to internalise. The heavy inline comments throughout this codebase exist precisely because of this.
- **Slow compile times** — especially with heavy crates like `axum` + `tokio`; monomorphisation generates a lot of specialised code.
- **Verbose for simple tasks** — sharing mutable state across threads (trivial in Python/Go) requires explicit `Arc<Mutex<T>>` boilerplate (visible in `InMemoryAdminEventSink`).
- **Smaller ecosystem** — fewer production-ready libraries than Node/Go/Java; ORMs, auth middleware, and observability tooling are less mature.
- **Async complexity** — `async`/`await` with `tokio` adds another mental layer on top of ownership and lifetimes.
- **Harder to hire for** — smaller talent pool than Go, Java, or TypeScript backend engineers.

**For this project specifically:** Rust is a strong fit because the domain rules are pure logic with no I/O — exactly where Rust's type system (newtypes, exhaustive enums, `Result`) shines as a *correctness* tool, not just a performance one.

---

**Q: The spec fixes the crew lead count at exactly 3. What would you change if the product requirement changed to "at least 1, at most 10"?**

A: Three changes:
1. Replace `CrewLeadBootstrapInvalid` check (`len != 3`) with a range check (`len < 1 || len > 10`).
2. Change `add` from "always reject" to "reject only if `len == 10`".
3. Change `remove` from "always reject" to "reject only if `len == 1`".

The invariant constant would move from a hardcoded `3` to configurable `MIN_LEADS` and `MAX_LEADS`. Test names would update to match the new spec IDs. The key point: the change is localised to `CrewLeadService` and its tests — nothing in the domain, infrastructure, or interface layers would change.

---

**Q: Access is currently binary — a passenger either can or cannot use a resource based on tier rank. How would you extend this to support time-limited access (e.g., Platinum access for 24 hours)?**

A: Without changing the existing structure:
1. Add a `valid_until: Option<Timestamp>` field to `Passenger` or to a new `TierGrant` entity.
2. Extend `can_access` (or introduce a new `AccessPolicy` trait) to take the current timestamp as a parameter.
3. The `AccessService` already injects a `Clock` — it would pass `clock.now()` into the policy check.
4. Emit the expiry in the `UsageEvent` snapshot so the audit log reflects what was true at the time.

This is where the `Clock` abstraction pays off: the access policy can be tested with a `FakeClock` set to a time before or after expiry — no real time involved.

---

**Q: The system has no persistent storage. If you had to add it, where would you start and what would you change minimally?**

A: Start with the port traits. `UsageEventSink` and `UsageEventSource` already abstract the event store — implement a `SqliteUsageEventSink` (or Postgres equivalent) in `src/infrastructure/`. For entity storage, introduce `PassengerRepository`, `ResourceRepository` traits in `src/application/ports.rs` and implement them. Update `build_demo_world()` in the composition root to use the database-backed repos. The domain and service layers are untouched. The total change surface: one new infrastructure file per entity, one updated port file, one updated composition root.

---

**Q: This system is described as a demo. What three things would you address first before calling it production-ready?**

A: (This is intentionally open-ended — here is one strong answer.)

1. **Authentication**: The `actor_id` is caller-supplied with no verification. Add a token-based auth layer (JWT or session cookie) at the interface boundary that maps a verified identity to an `Actor`. The domain layer stays unchanged.
2. **Persistent storage**: In-memory state resets on restart. Add a database-backed repository (point above) with migrations, backups, and a tested recovery path.
3. **Operational readiness**: Metrics endpoint, structured logs shipped to an aggregator, health check that tests the DB connection, graceful degradation plan. A system that can't be observed in production cannot be operated safely.

---

**Q: Walk me through exactly what IS implemented in this repository and what is NOT implemented if the goal is a production top-tier application.**

A:

**✅ Implemented**

| Area | Detail |
|------|--------|
| **Domain model** | `Passenger`, `Resource`, `CrewLead`, `Tier` (Silver/Gold/Platinum), all as newtypes and enums. No primitives leaking through the layer boundary. |
| **Tier access policy** | `can_access(passenger_tier, resource_min_tier)` via `rank()` comparison — all 9 tier-pair combinations spec-verified. |
| **Passenger lifecycle** | Create, soft-delete, change tier — crew-lead-only, gated by `is_crew_lead()` guard. |
| **Resource lifecycle** | Create, soft-delete, change `min_tier` — crew-lead-only. |
| **Crew lead management** | Bootstrap with exactly 3 leads; `replace` (atomically swaps); `add` (always 409, spec CL-R2); `remove` (always 409, spec CL-R3). |
| **Access checks** | `AccessService::use_resource` — validates passenger and resource exist, evaluates tier policy, emits `UsageEvent` (allowed OR denied). |
| **Audit trail** | Every admin mutation (create/delete/tier-change/crew-lead-replace) emits an `AdminEvent` via `AdminEventSink`. Append-only, in-memory. |
| **Usage event log** | Every access attempt stored in `InMemoryUsageEventSink`. Includes snapshot fields: `tier_at_attempt`, `min_tier_at_attempt` — correct even after future tier changes. |
| **Reporting** | `aggregate_by_tier` (allowed/denied counts per tier), `top_resources` (top N by allowed count), `personal_history` (all events for one passenger). |
| **HTTP adapter** | ~27 Axum endpoints: full CRUD for passengers/resources, crew-lead management, access, audit, usage, reports, reset. Feature-gated (`--features http`). |
| **OpenAPI spec** | Dynamically generated by `utoipa` at `/openapi.json`. Always in sync with handler annotations, version embedded from `Cargo.toml`. |
| **CORS + request IDs** | `tower-http` layers: `CorsLayer` (configurable origin list or `Any`), `SetRequestIdLayer` + `PropagateRequestIdLayer` for `x-request-id` correlation. |
| **Error mapping** | `DomainError` → HTTP status (400/403/404/409). Machine-readable `code` string in every error body. |
| **Deterministic clock** | `FakeClock` (`AtomicI64`, `Ordering::Relaxed`) — injected as a port trait. Time-sensitive business logic is fully testable with no `std::time`. |
| **Input validation** | `TryFrom` and `serde` at the interface boundary — unknown tiers, malformed IDs rejected before reaching services. Pagination params and `TopNQuery.n` are capped at the boundary (OWASP A04). |
| **Test suite** | Unit tests (alongside source), integration tests (`tests/`), HTTP integration tests (`tests/http_*.rs`). Named after spec IDs. `cargo nextest` < 60 s. |
| **Code coverage** | `cargo llvm-cov` configured; `coverage.json` artifact in repo. 96%+ line coverage gate in CI; only `src/bin/` excluded. |
| **Static analysis** | `clippy --all-targets --all-features -- -D warnings -W clippy::pedantic` enforced. `rustfmt` enforced. `cargo-deny` for supply chain. |
| **React frontend** | Thin client in `web/`; fetches all content from the Rust axum backend. TypeScript types generated from `/openapi.json` via `npm run generate:types`. |
| **AGENTS.md** | Machine-readable contributor contract; codifies architecture, testing, commit format, and naming conventions. |
| **Authentication** | `AuthActor` extractor reads `Authorization: Bearer <token>`, resolves against `PRMS_API_KEYS` (token → actor-id), returns 401 for missing/unknown tokens. Constant-time comparison via `subtle::ConstantTimeEq` (OWASP A07). |
| **Persistent storage** | SQLite opt-in via `PRMS_DB_PATH`. Usage and admin events are written through on every `append()`; entity state is restored from SQLite on startup. Without `PRMS_DB_PATH`, falls back to in-memory seeded demo. |
| **PostgreSQL backend** | `--features postgres` + `PRMS_PG_URL`. `PgEntityStore` restores entities on startup and batch-syncs via `sync_all`. Event sinks are still in-memory per-process (load-on-boot); per-request write-through is a planned Phase 4 item. |
| **Database migrations** | `migrations/001_initial.sql` — append-only `usage_events` and `admin_events` tables, WAL mode, entity state tables. `SqliteEntityStore` restores entity state on startup. |
| **Pagination** | `?offset=N&limit=N` (default 0/100, max offset 1 000 000, max limit 1 000) on all list endpoints via `PaginationQuery`. `TopNQuery.n` capped at 1 000. |
| **Rate limiting** | Per-IP token-bucket governor (`tower_governor`), configurable via `--rate-limit-rps` / `--rate-limit-burst`. Disabled in tests to avoid loopback-IP exhaustion. |
| **Graceful shutdown** | SIGTERM/SIGINT handled in `serve.rs` via `tokio::signal`; configurable drain window (`--shutdown-grace-secs`, default 10 s). |
| **Structured logging** | `tracing` + `tracing-subscriber`; `--log-format text|json` (`PRMS_LOG_FORMAT`). JSON format ships newline-delimited records suitable for Loki/Datadog/CloudWatch. `x-request-id` propagated through spans. |
| **Metrics** | `GET /metrics` — Prometheus text format. Gauges for crew leads, passengers, resources; counters for usage events (allowed/denied split) and admin events. |
| **Health check depth** | `GET /health/ready` — JSON with entity counts (crew leads, passengers, resources, usage events, admin events); DB liveness ping; 503 if any lock is poisoned. |
| **Concurrency** | Per-aggregate `RwLock` shards replace the original single `Arc<Mutex<World>>`. Reads on different aggregates proceed without blocking; documented lock-acquisition order prevents deadlocks. |
| **CI/CD pipeline** | `.github/workflows/ci.yml` — fmt, clippy (default + `--features http`), nextest (default), llvm-cov (96% gate), cargo-deny, web build, and Playwright E2E. |
| **Container / deployment** | Multi-stage `Dockerfile` (builder: `rust:1-bookworm`, runtime: `debian:bookworm-slim`), non-root uid 10001. `docker-compose.yml` + `Caddyfile` for automatic TLS via Let's Encrypt. |

---

### Coverage Exclusions — By Design (not worth testing)

The CI gate is **96% line coverage** (currently ~98.65%). The remaining uncovered
lines are structurally or architecturally untestable and should **not** be
patched with low-value tests:

| Source location | Why it cannot / should not be tested |
|---|---|
| `http.rs:1346-1348` `add_crew_lead` success branch | **Spec CL-R2 permanently prohibits this path.** `CrewLeadService::add()` always returns `CrewLeadLimitReached`. The success arm is unreachable without violating the invariant. |
| `http.rs:1375-1377` `remove_crew_lead` success branch | Same as above — **CL-R3** makes `remove()` always return `CrewLeadMinimumBreached`. |
| `http.rs:579-586` `ping_db → Some(false)` 503 branch | Requires a live SQLite connection that breaks *after* the `World` is constructed. No test-safe way to corrupt the connection without production-code changes (interrupt handle not exposed). |
| `http.rs:398` `CrewLeadBootstrapInvalid` in `map_err` | Only reachable if `build_demo_world()` fails (malformed seed data). Covered by unit test at the `CrewLeadService` level; reaching it via HTTP is not achievable in a hermetic test. |
| `http.rs:400` `VersionConflict` in `map_err` | 412 is returned directly by `version_conflict()` *before* `err_response_owned` is called — dead by design in the current handler flow. |
| `http.rs:404` `_ => InternalError` wildcard arm | `#[allow(unreachable_patterns)]` — intentional forward-compat arm for future `DomainError` variants. Unreachable with all current variants mapped. |
| `http.rs:1471` `/reset` bootstrap error | `build_demo_world()` is called with hard-coded valid seed data; the error arm would only fire on a coding bug, not at runtime. |
| `composition_root.rs:288-342` Postgres `build_world` error arms | Require a live failing PostgreSQL instance. Gated behind `--features postgres` which is not in the CI matrix. |
| `composition_root.rs:367-374` `BuildError::Debug` derive | Auto-generated by `#[derive(Debug)]`; LLVM instruments it but it is not exercised in tests. Not worth driving via `{:?}` format in a test. |

**Rule of thumb:** if covering a line requires either violating the spec, mocking
infrastructure at the OS level, or using `--features` not in CI, leave it
uncovered and document it here instead.

---

**❌ Not Implemented (would be required for a production top-tier app)**

| Area | Gap | Where it would go |
|------|-----|-------------------|
| **TLS / HTTPS** | The server binds plain HTTP; TLS is terminated externally by Caddy in the provided `docker-compose.yml`. Direct TLS is not built in. | `rustls` + `axum-server-tls`, or keep Caddy as the TLS terminator. |
| **Multi-instance / horizontal scaling** | Postgres backend exists (`--features postgres`): entities are restored from PG on startup and can be batch-synced. However, event sinks are still in-memory-first — new events and mutations during a process lifetime are NOT written through to PG per-request. Two instances would diverge immediately after startup. | Convert event sinks and entity mutations to async write-through (per-request SQL) rather than load-on-boot + batch flush. Remove `entity_store = None` in the Postgres path. |
| **Optimistic concurrency control** | No `version` field; last-write-wins on concurrent tier updates. | `version` column on `passengers`/`resources`; `If-Match`/ETag headers; `409 Conflict` on stale writes. |
| **Soft-delete querying** | `list()` silently omits deleted records; no API to inspect deleted history. | `GET /passengers?include_deleted=true`; expose `deleted_at` in list responses. |
| **Frontend error handling** | Unknown error codes fall through to a generic message. | Generate error code enum from OpenAPI; exhaustive TypeScript switch with explicit fallback. |
| **Accessibility** | No `aria-label`s audited, no keyboard-only path verified, colour-only error signals. | axe / Playwright accessibility scans; WCAG 2.1 AA compliance pass. |
| **Tamper-evident audit log** | In-memory `Vec<AdminEvent>` is append-only by convention only. | Hash-chain events on insert; persist to append-only table; anchor periodic checkpoints externally. |
| **Secret management** | No vault/env-secret infrastructure defined. | 12-factor env vars with a secrets manager (AWS Secrets Manager, Vault) — never committed to the repo. |
| **API versioning** | No versioning strategy; every field rename is a breaking change. | Path versioning (`/v1/...`) or media-type versioning; OpenAPI compatibility checks in CI. |
| **Load / chaos testing** | No performance baseline, no fault-injection tests. | `k6` or `wrk` load test for throughput baseline; chaos tests for mutex-poisoning recovery path. |

**Summary:** The repository is solid for levels 1–4 of the assignment scope — the domain model, business rules, audit trail, reporting, HTTP adapter, and React demo are all present and tested. The gaps above are the standard delta between a well-engineered demo and a production service. None of them require redesigning the domain or service layers; they are infrastructure, operations, and security concerns that the port-trait architecture was explicitly designed to absorb without touching business logic.

---

**Q: For each production gap above, how would you actually implement it?**

A:

**1. TLS / HTTPS.** In production, terminate TLS at a reverse proxy (nginx or Caddy) — both handle certificate renewal via Let's Encrypt automatically. The provided `docker-compose.yml` + `Caddyfile` already do this. For embedded TLS, replace `tokio::net::TcpListener` in `serve.rs` with `axum_server::tls_rustls::RustlsConfig::from_pem_file(cert, key)` from the `axum-server` crate. Never store certificates in the repo; mount them via Kubernetes secrets or a secrets manager at deploy time. Enforce HSTS once TLS is active.

**2. Multi-instance / horizontal scaling.** The Postgres backend (`--features postgres`) already handles entity restore on startup (`build_world_with_postgres`) and batch sync via `PgEntityStore::sync_all`. The missing piece is making the write path async and per-request: today, new mutations go to in-memory `RwLock` shards and event sinks are in-memory buffers. A second instance starting from the same Postgres DB would restore the same snapshot but then immediately diverge as mutations accumulate in each process's memory. The fix is to replace `entity_store = None` in the Postgres composition root path with a `PgEntityStore` that is called on every mutation (like `flush_to_db` for SQLite, but async) and to implement async `UsageEventSink` / `AdminEventSink` backed by PG so all event appends write through. Once all reads/writes go through PG, the per-aggregate `RwLock` shards become a cache that can be invalidated or replaced with direct DB reads, and the `Deployment` can scale `replicas` freely.

**3. Optimistic concurrency control.** Add a `version: i64` column to `passengers` and `resources`, defaulting to `0`, incremented on every update. Request DTOs for mutations include `expected_version: i64`. The `UPDATE` statement uses `WHERE id = $1 AND version = $2`; if it affects 0 rows, the row was modified by a concurrent request — return `409 Conflict`. Surface this on the HTTP layer with `If-Match: "<version>"` header; the client re-fetches and retries. The `Passenger` domain struct gains a `version` field; the port trait `PassengerRepo::update` accepts the expected version.

**4. Soft-delete querying.** Add `include_deleted: Option<bool>` to the `GET /passengers` and `GET /resources` query structs. The port trait `list(include_deleted: bool)` passes the flag to the repository, which either filters `WHERE deleted_at IS NULL` or returns all rows. The response DTO already includes `deleted_at: Option<i64>` — it becomes non-null for deleted records. Add a dedicated `GET /passengers/{id}/history` endpoint that always returns the record regardless of deletion status, for audit reconciliation purposes.

**5. Frontend error handling.** Generate the error code string enum from the OpenAPI spec (the `ErrorBody.code` field can be defined as an `enum` in the `utoipa` schema annotation). The TypeScript generated type becomes `code: "PassengerNotFound" | "AccessDenied" | ...`. Replace any `KNOWN_CODES` set and string-switch with a type-safe exhaustive switch over the generated union. Add an `isKnownError(code: string): code is KnownErrorCode` type guard for runtime validation of responses from older API versions.

**6. Accessibility.** Audit every interactive element with `axe-core` (via `@axe-core/react` in development mode, which logs violations to the browser console). Fix in priority order: all `<input>` elements need `<label htmlFor>` or `aria-label`; all icon-only buttons need `aria-label`; the allowed/denied outcome must not rely on colour alone (add a text label or icon with accessible name). Run `npx playwright test --grep accessibility` using Playwright's built-in `checkA11y` from `axe-playwright` as a CI gate. Target WCAG 2.1 AA.

**7. Tamper-evident audit log.** On every `AdminEvent` insert, compute `event_hash = SHA-256(previous_hash || canonical_json(event))` and store it alongside the event. The first event uses a known genesis hash (e.g., `SHA-256("genesis")`). A verification endpoint `GET /audit/verify` replays the hash chain and returns `{ valid: true, length: n }` or the first broken index. For stronger guarantees, periodically write the head hash to an external immutable store (S3 object with object lock, or a public blockchain anchor). This makes any deletion or insertion detectable.

**8. Secret management.** Never pass secrets via environment variables in plain text in production. Use a secrets manager: in AWS, store credentials in Secrets Manager or Parameter Store and fetch them at startup using the `aws-config` + `aws-sdk-secretsmanager` crates. In Kubernetes, mount secrets as environment variables from `Secret` objects (never `ConfigMap`). Rotate secrets without redeployment by adding a background task that refreshes the database password from the vault on a schedule. The `deny.toml` already blocks yanked crates; extend it with a `cargo audit` step in CI to catch known CVEs.

**9. API versioning.** Prefix all routes with `/v1/` from the start. In the `router_with` function, nest the existing routes under `.nest("/v1", v1_router())`. When a breaking change is needed, create `router_v2()` with the new shape and mount it at `/v2/`. The OpenAPI spec at `/openapi.json` serves the latest version by default; add `/v1/openapi.json` and `/v2/openapi.json` as separate endpoints. Add an OpenAPI compatibility check in CI (`oasdiff breaking old.json new.json`) that fails the build if a new commit introduces breaking changes to a currently-served version.

**10. Load / chaos testing.** Write a `k6` script (`tests/load/access_flood.js`) that creates 10 passengers and 5 resources, then hammers `POST /access` at 500 RPS for 60 seconds, asserting p99 latency < 200 ms and error rate < 0.1%. Run it in CI on merge to `main` against a staging environment. For chaos testing, use `toxiproxy` to inject latency and connection drops on the database connection and assert the server returns `503` gracefully rather than hanging. Test mutex-poisoning recovery by intentionally panicking a handler in a canary test and asserting the next request receives a 500 (not a hang).

---

**Summary:** The repository covers the full assignment scope — domain model, business rules, audit trail, reporting, HTTP adapter with auth/rate-limiting/pagination/metrics, SQLite and PostgreSQL persistence, React thin client with generated types, Playwright E2E, and CI/CD with coverage gating. The remaining gaps (TLS termination, multi-instance write-through, optimistic concurrency, soft-delete querying, frontend error enum, accessibility, tamper-evident audit, secret management, API versioning, load/chaos testing) are infrastructure and operations concerns that the port-trait architecture was explicitly designed to absorb without touching business logic.

---

## 22. Extra Interviewer Angles

Related files: [src/interface/http.rs](../src/interface/http.rs), [web/src/services/api.ts](../web/src/services/api.ts), [web/src/services/accessService.ts](../web/src/services/accessService.ts), [src/domain/usage_event.rs](../src/domain/usage_event.rs), [src/domain/admin_event.rs](../src/domain/admin_event.rs)

**Q: If you were reviewing this repository, what is the first risk you would call out?**

A: The biggest production risk is that identity is caller-supplied. The API accepts an `actor_id` in request bodies and turns it directly into an `Actor`, so a malicious client could impersonate a crew lead or another passenger if they know the ID. For a demo this is acceptable, but for a real system the interface layer needs authentication middleware that validates a token/session and derives the actor from trusted identity claims. The service layer can still accept `Actor`; the difference is that the client no longer controls it directly.

---

**Q: What is one subtle frontend/backend mismatch you would ask about?**

A: The TypeScript `AccessService` checks that the passenger actor is acting as themselves (`actor.id === passengerId`), while the Rust `AccessService` only extracts the passenger ID from `Actor::Passenger(...)` and does not compare it with a separate subject ID. This is a good review question because it tests whether the candidate noticed cross-implementation drift. The right fix is not to guess; update the access spec first, then align Rust and TypeScript tests and implementations.

---

**Q: How would you prevent frontend/backend type drift long-term?**

A: Generate the frontend API client from the Rust OpenAPI output (`/openapi.json`). Today, `web/src/services/api.ts` manually declares API response types and error codes. That works for a small demo, but it creates drift risk: a Rust field rename or enum addition will not automatically update TypeScript. A generated client (`openapi-typescript`, `orval`, or `openapi-generator`) would make the Rust server contract the source of truth. I would still add runtime validation for critical response shapes if the API is consumed across trust boundaries.

---

**Q: What accessibility questions could a reviewer ask about the React UI?**

A: They may ask whether all form controls have associated labels, whether buttons have meaningful text or `aria-label`s, whether color is not the only signal for allowed/denied states, whether keyboard-only users can complete every workflow, and whether focus is managed after mutations or errors. The current code review material focuses mostly on domain and API correctness; for a full-stack role, you should be ready to talk about WCAG basics, semantic HTML, visible focus states, and testing with tools like axe or Playwright accessibility checks.

---

**Q: How would you add optimistic UI updates to the frontend?**

A: For operations like changing a passenger tier, the UI could immediately update local state before the server confirms, then roll back if the request fails. The important pieces are: keep a snapshot of the previous state, apply the optimistic mutation, show a pending state, reconcile on success, and restore the snapshot on error. For this app, optimistic updates are not necessary because operations are local/in-memory and fast, but in a real networked UI they improve perceived responsiveness. I would avoid optimistic updates for audit-sensitive operations unless rollback behavior is very clear.

---

**Q: What would you do if two admins edit the same passenger at the same time?**

A: Today, the in-memory `Mutex` serialises writes inside one server process, but it does not solve multi-user conflict semantics. If two admins update the same passenger tier, the last request wins. In production I would add optimistic concurrency control: each passenger has a `version` or `updated_at`; update requests include `If-Match`/ETag or an expected version; the server rejects stale writes with `409 Conflict` or `412 Precondition Failed`. The frontend then shows a conflict resolution message and reloads the current state.

---

**Q: How would you make audit logs tamper-evident?**

A: The current event sinks are append-only by convention, but in-memory vectors are not tamper-evident. For production, persist audit events in an append-only store and hash-chain them: each event stores `previous_hash` and `event_hash = hash(previous_hash + canonical_event_json)`. Any deletion or modification breaks the chain. For stronger guarantees, periodically anchor the head hash in an external system (object storage, ledger, or signed checkpoint). This is especially relevant because access and admin events are part of the system's compliance story.

---

**Q: What database schema would you design for this system?**

A: A simple relational schema:
- `crew_leads(id primary key, name, created_at, replaced_at nullable)`
- `passengers(id primary key, name, tier, deleted_at nullable, version)`
- `resources(id primary key, name, category, min_tier, deleted_at nullable, version)`
- `usage_events(id primary key, passenger_id, resource_id, tier_at_attempt, min_tier_at_attempt, outcome, timestamp)`
- `admin_events(id primary key, actor_id, action, target_kind, target_id, timestamp, details)`

The snapshot fields in `usage_events` are intentionally duplicated rather than normalised away. Audit history must remain correct even if current passenger/resource rows change later.

---

**Q: Would you use SQL transactions for access attempts?**

A: Yes, if access attempts were persisted. The read of passenger/resource state, the permission decision, and the insert into `usage_events` should be one transaction at an appropriate isolation level. Otherwise a tier or min-tier could change between the read and the event insert, producing a snapshot that does not match the decision. In the current implementation, the `access` write lock is held for the entire `use_resource` call; the `passengers` and `resources` read locks are released after their data is read. This prevents interleaving within a single access attempt. With a database backend, the equivalent is a `SERIALIZABLE` or `REPEATABLE READ` transaction spanning the read and insert.

---

**Q: What API versioning strategy would you use?**

A: For a small internal API, I would start with additive changes only and rely on OpenAPI compatibility checks in CI. If breaking changes become necessary, use path versioning (`/v1/passengers`) or media-type versioning (`Accept: application/vnd.prms.v1+json`). Path versioning is easier for frontend teams and API gateways. Error codes and enum values should be treated as part of the contract, so renaming `AccessDenied` is a breaking change just like removing a JSON field.

---

**Q: How would you test the Rust API and React frontend together end-to-end?**

A: Start the Rust server with `cargo run --features http --bin serve`, start the Vite dev server, then use Playwright to drive the browser. The test should cover a real user flow: create passenger, create resource, attempt denied access, upgrade tier, attempt allowed access, verify reports and audit log. This catches issues unit tests miss: CORS/proxy mistakes, JSON field mismatches, broken form wiring, and layout/interaction regressions.

---

**Q: What would you monitor first after deploying this system?**

A: I would monitor request rate, error rate, p95/p99 latency, number of denied vs allowed access attempts, audit event append failures, and memory usage. The usage event log grows forever in memory, so memory growth is especially important. For product insight, denied access spikes by tier/resource are also useful: they may reveal confusing permissions or passengers frequently attempting resources above their tier.

---

**Q: How would you explain the project to a non-technical interviewer in one minute?**

A: It is a small passenger resource management system for a spaceship. Crew leads manage passengers and resources. Each passenger has a tier — Silver, Gold, or Platinum — and each resource has a minimum tier. When a passenger tries to use a resource, the system decides allowed or denied, records the attempt, and exposes reports. The important engineering part is that the code is split cleanly: business rules are pure and tested, the HTTP API is just an adapter, and the React demo exercises the same rules from a user's point of view.

---

**Q: If a reviewer asks "why should we hire you from this project?", what answer connects the repo to full-stack work?**

A: This project shows I can work across the stack while keeping boundaries clear. On the backend, I modelled domain rules with Rust types, deterministic clocks, append-only audit events, and tested services. On the API layer, I exposed those services through Axum with DTOs, CORS, request IDs, OpenAPI, and error mapping. On the frontend, I mirrored the same rules in TypeScript, used branded ID types to catch mistakes, built a React demo, and wired a live-server panel through Vite's proxy. The key point is not just that I wrote backend and frontend code — it is that I treated the contract between them as something testable and reviewable.

---

## 23. Prompt & Submission Rubric Questions

Related files: [docs/plan-passengerResourceManagement.prompt.md](plan-passengerResourceManagement.prompt.md), [README.md](../README.md), [AGENTS.md](../AGENTS.md), [Cargo.toml](../Cargo.toml), [rust-toolchain.toml](../rust-toolchain.toml)

**Q: If you want to add a new requirement to this project, what is the correct flow?**

A: The flow follows AGENTS.md §9 — spec first, code last. AGENTS.md itself is **not** touched for individual features; it contains house rules about *how* to work, not *what* to build. The sequence is:

1. **Update the plan** (`docs/plan-passengerResourceManagement.prompt.md`) — add the feature to the relevant level's deliverables and update the "Done" checklist so the intent is recorded before any code moves.
2. **Write or update the spec** (`specs/XX-feature.md`) — define numbered rules (`R1`, `R2`…), invariants (`I1`…), and scenarios (`S1`, `S2`…). This becomes the single source of truth. No code is written until the spec is settled and reviewed.
3. **Write failing tests** — in `tests/feature.rs` or a `#[cfg(test)]` block inline with the source, named after the scenario IDs (e.g., `fn feat_r1_s1_description`). They must be **red**: the implementation does not exist yet.
4. **Write the minimum code to go green** — domain types first (if new types are needed), then service methods, then port/infrastructure changes if needed, then HTTP handlers and DTOs last. Stop as soon as the tests pass; resist the urge to over-engineer.
5. **Refactor with tests green** — clean up duplication, add inline comments explaining non-obvious choices, ensure `clippy` and `rustfmt` pass.
6. **Commit** — conventional commit scoped to the spec ID: `feat(feature): FR-R1 description`. One commit per spec ID where possible.

AGENTS.md is only updated if the new requirement changes the **house rules** themselves — e.g., a new mandatory tool, a new architectural layer, or a new constraint on code style. Adding a domain feature like pagination, time-limited access, or an auth layer does not change AGENTS.md.

---

**Q: The prompt asks reviewers to judge the project as if written by an experienced engineer. What evidence in the repo supports that?**

A: The strongest evidence is not one flashy feature; it is the consistency of engineering discipline across the repo. Business rules are written as specs first, tests are named with spec IDs, the architecture has clear dependency direction, domain code is pure, failures return `Result` instead of panicking, and optional adapters like HTTP and React are kept outside the core path. An experienced engineer also makes reviewer experience easy: the README has a short quickstart, the toolchain is pinned, and tests run without external services.

---

**Q: The plan says "SOLID principles" are part of the grading values. Which SOLID principles are easiest to defend in this codebase?**

A: SRP and DIP are the easiest to defend. SRP appears in the service split: `PassengerService` handles passenger lifecycle, `ResourceService` handles resource lifecycle, `AccessService` handles access attempts, and `ReportingService` handles read-only analytics. DIP appears through application-layer port traits (`Clock`, `UsageEventSink`, `AdminEventSink`) that are implemented by infrastructure adapters. ISP is also visible because read and write event capabilities are separate (`UsageEventSource` vs `UsageEventSink`). OCP is partly supported by adding new adapters without changing existing services, but a new tier still requires editing exhaustive matches, which is intentional because the tier set is currently closed.

---

**Q: When coding with AI assistance, what should the human own and what can the AI do?**

A: The split follows a simple rule: **humans own intent and judgment; AI owns speed and mechanical correctness**.

**Human must own:**

| Responsibility | Why |
|---|---|
| **Deciding what to build** | AI cannot read business context, stakeholder priorities, or product risk — it will happily generate the wrong feature perfectly. |
| **Writing and approving the spec** | The spec encodes the rules. If the human skips this, the AI generates code against an implicit spec it invented, which drifts from reality. In this repo, specs live in `specs/` and rules are numbered (`R1`, `R2`). The human reads and signs off on them. |
| **Reviewing all generated code** | AI code is statistically plausible, not provably correct. The human checks: does this match the spec? Are invariants preserved? Are there security holes? Could this panic or leak? |
| **Making architectural decisions** | Which layer owns this logic? Do we need a new port trait or can we extend an existing one? These decisions compound — the human must set the direction. |
| **Committing and signing off** | The human's name is on the commit. GPG-signing (as in this repo) makes that explicit. |

**AI can accelerate:**

| Task | How AI helps |
|---|---|
| **Boilerplate generation** | Given the spec scenario, AI writes the failing test skeleton — naming it correctly (`fn feat_r1_s1_...`), stubbing the assertions. Human fills in the exact values. |
| **First-pass implementation** | AI writes the minimum code to pass the test. Human reviews for spec compliance, edge cases, and style. |
| **Inline comments** | AI explains *why* code is written a certain way (e.g., the `AtomicI64` in `FakeClock`, the borrow-split in `use_resource`). Human verifies the explanation is accurate. |
| **Boilerplate-heavy layers** | DTO `From` impls, `#[utoipa::path(...)]` annotations, HTTP handler wiring — these are mechanical given the domain types already exist. |
| **Refactoring within green tests** | AI suggests cleaner patterns (e.g., `entry().or_default()` vs manual `contains_key` + `insert`). Human decides whether to accept. |
| **Documentation** | AI drafts Q&A answers, glossary entries, README sections. Human edits for accuracy and tone. |
| **Searching and explaining existing code** | AI reads the codebase and explains what a function does, where a type is used, or why a constraint exists. |

**The failure modes to avoid:**

- **Vibe-coding without reviewing**: accepting AI output without reading it. The AI does not understand the spec; only the human does.
- **Skipping the spec step**: asking AI to "implement pagination" without first defining the rules. The AI will pick reasonable defaults that may be wrong for this system.
- **Over-delegating architecture**: AI will suggest patterns (service locator, global state, `unwrap()` everywhere) that violate this repo's rules. The human must reject them and explain the constraint.
- **Treating AI as infallible on security**: AI-generated auth code, input validation, and crypto are statistically derived from training data — they need explicit security review, not just functional testing.

**In this repo specifically:** AGENTS.md is the AI's instruction contract — it tells the AI what conventions to follow. The human wrote AGENTS.md. Every spec, every architectural decision, every commit message convention in it came from human judgment. The AI uses those rules to generate consistent, rule-compliant code faster than typing it by hand.

---

**Q: Where could someone argue the code is not fully OOP, and how would you answer?**

A: Rust is not a classical inheritance-based OOP language, so the code does not use base classes or subclass polymorphism. The design still uses object-oriented ideas where they fit: services encapsulate state and expose methods, domain objects have typed identities, and behaviour is attached to types (`Tier::rank`, `Tier::can_access`). For polymorphism, Rust uses traits instead of inheritance. The important answer is that the project applies OOP/design patterns pragmatically rather than forcing Java-style inheritance into Rust.

---

**Q: The prompt planned a CLI, but the implemented project has an HTTP server and React UI. How would you explain that in review?**

A: The plan originally resolved on CLI as the first interface, but the implementation went beyond the base deliverable by adding an Axum HTTP adapter and a React demo. The important architectural claim still holds: the interface layer is thin and business logic remains in services. If a reviewer asks about the missing CLI, I would say it is an interface choice, not a domain gap; adding `src/bin/cli.rs` would call the same composition root and services without changing domain or application code. The HTTP + React path actually demonstrates more full-stack capability than a CLI alone.

---

**Q: The prompt mentions JSON persistence as an above-and-beyond planned extra. It is not implemented. Is that a problem?**

A: Not for the core requirements. Levels 1-3 require in-memory management, access validation, audit logging, and reporting. JSON persistence is explicitly listed as optional above-and-beyond work. The correct review answer is to acknowledge the scope boundary: persistence is designed for through port traits, but not implemented yet. I would explain where it would go (`src/infrastructure/`) and how the composition root would inject it, while being honest that current state resets on restart.

---

**Q: The prompt says target 100% coverage, but the plan notes coverage is around 99.63% lines / 98.38% regions. How would you handle that question?**

A: I would be transparent. Function coverage is 100%, line coverage is very close to 100%, and the remaining gaps are region-level branches such as derive-generated formatting paths or optional audit branches. The plan already documents the exact next tests needed: sink-less passenger/resource services, colliding `replace_audited`, and `InvalidTier` display/debug formatting. I would not claim 100% if it is not true; I would say the code is well-covered, the gaps are known, and the path to close them is small and documented.

---

**Q: If a reviewer asks "show me TDD evidence," what should you point to?**

A: Point to three things: (1) the `specs/` directory with numbered rules and scenarios, (2) test names that include those IDs (`AC-S7`, `TP-R1`, etc.), and (3) commits if the git history is available. The strongest answer is: "Each feature was driven from a spec scenario into a test, then implementation." If the reviewer wants direct evidence beyond final code, git history matters — conventional commits mapped to spec IDs make the red/green/refactor story visible.

---

**Q: What should be in the AI Usage Disclosure, and why does the prompt require it?**

A: It should state which AI tools were used, where AI helped most, what was manually reviewed or rewritten, what AI suggestions were rejected, and what verification was done independently. The reason is trust and accountability: AI usage is allowed, but the reviewer still needs to know the candidate understood the code and validated it. A strong disclosure does not say "AI wrote this"; it says "AI accelerated scaffolding and review prep, but I owned the architecture, invariants, tests, and verification."

---

**Q: What AI-generated output would you explicitly say you rejected?**

A: I would mention rejecting over-engineered abstractions, hidden global state, runtime dependencies in the domain layer, and any suggestion that used `unwrap()`/`expect()` in expected failure paths. If AI suggested a database, auth system, or broad framework rewrite before the core spec was complete, that was rejected as scope creep. This answer shows judgement: using AI well means not accepting everything it produces.

---

**Q: The prompt says not to include `target/` in the ZIP, but the workspace has a `target/` directory. What should you do before submission?**

A: Exclude it from the ZIP. `target/` contains build artifacts and can be huge; reviewers should rebuild from source. The submission should include source files, specs, README, Cargo files, toolchain config, tests, and optionally git history if requested. It should exclude `target/`, coverage HTML output, `.DS_Store`, temporary files, and any generated artifacts that can be recreated. `Cargo.lock` should be included because this is an application-style crate and reproducible dependency resolution helps reviewers.

---

**Q: The README quickstart says `rustup show && cargo nextest run`. What if the reviewer does not have `cargo-nextest` installed?**

A: The README should either mention installing nextest (`cargo install cargo-nextest --locked`) or provide `cargo test` as a fallback. The prompt prefers `cargo nextest run` for reviewer DX, but a robust README can say "preferred: `cargo nextest run`; fallback: `cargo test`." If the challenge expects no extra setup, documenting the fallback matters. The current house rules say reviewer should be able to run `rustup show && cargo nextest run`, so making that exact path work is still the priority.

---

**Q: The prompt says "no env vars required for the core path." Does the project satisfy that?**

A: Yes for the Rust core tests. `cargo nextest run` exercises domain/application/infrastructure integration without external services, network, filesystem persistence, or environment variables. Env vars exist only for optional HTTP runtime configuration (`PRMS_BIND`, `PRMS_CORS_ORIGINS`, `RUST_LOG`) and frontend configuration (`VITE_API_BASE`). That separation is important: optional adapters can be configurable, but the reviewer's core path stays deterministic and zero-setup.

---

**Q: How would you explain the difference between the planned domain model and the final implementation?**

A: The plan uses sketch types like `Id` and `DateTime<Utc>`, while the implementation uses typed ID newtypes and a custom `Timestamp`. That is an improvement: newtypes prevent ID mix-ups and the custom timestamp supports deterministic tests without wall-clock dependencies. The final implementation also split admin and usage events into separate sinks, which makes each audit stream clearer. The key is that the final implementation follows the plan's intent while tightening type safety and testability.

---

**Q: What are the three strongest design trade-offs to explain from the prompt?**

A: First, **opt-in SQLite vs always-on persistence**: in-memory is the default so the core test path needs no infrastructure; SQLite is opt-in via `PRMS_DB_PATH`. Data resets on restart without it — a documented, acceptable trade-off for the demo scope. Second, **per-aggregate `RwLock` shards vs actor-model channels**: per-aggregate locking is simple and correct; an actor-model (each service as a tokio task + channel) would eliminate locks entirely but adds complexity. Third, **generated TypeScript types vs handwritten mirror**: `openapi-typescript` now generates types from the live `/openapi.json`, eliminating drift risk. These trade-offs match the current scope and each has a clear upgrade path.

---

**Q: The prompt mentions "clean, readable code — no dead code." What would you check before final submission?**

A: I would run `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic`, `cargo fmt --check`, `npm run lint`, and a quick `rg` for `TODO`, `FIXME`, `dbg!`, commented-out code, unused files, and local machine paths. I would also scan README commands line by line on a fresh clone. A reviewer should not find stale plan-only artifacts presented as complete work.

---

**Q: If a reviewer asks why the code uses Rust instead of a more common full-stack backend language, what is a good answer?**

A: Rust fits this problem because the domain is invariant-heavy: typed IDs prevent entity mix-ups, enums model closed sets like `Tier` and `Outcome`, and `Result` forces explicit error handling. The borrow checker also prevents accidental shared mutable state. For a real team, Rust has a steeper learning curve than Node or Java, so I would choose it when correctness, performance, and long-term reliability matter enough to justify that cost. This project uses Rust to demonstrate type-driven design, not because every CRUD app needs Rust.

---

**Q: What would you say if the reviewer asks whether the project is over-engineered?**

A: I would say the core is intentionally small: each service is a simple in-memory struct, ports are minimal traits, and there is no database/auth system beyond the prompt scope. The layering may look formal for a small assignment, but it pays off because optional HTTP and React adapters were added without moving business rules. I would also acknowledge that for a tiny script-only solution this would be too much; for a code-review challenge judged on SOLID, TDD, extensibility, and reviewer DX, the structure is appropriate.

---

**Q: What should you be ready to demo live during the review?**

A: Be ready to run the core tests, start the HTTP server, call `/health`, make one allowed and one denied access attempt, show `/usage` and `/reports/by-tier`, and open the React demo if time allows. Also be ready to navigate from a spec rule (for example `AC-S7`) to its test and then to the implementation in `AccessService`. That spec → test → code path is the clearest way to show ownership.

---

## 24. Rapid-Fire Reviewer Follow-Ups

Related files: [src/domain/tier.rs](../src/domain/tier.rs), [src/application/access_service.rs](../src/application/access_service.rs), [src/interface/http.rs](../src/interface/http.rs), [web/src/domain/tier.ts](../web/src/domain/tier.ts), [web/src/services/api.ts](../web/src/services/api.ts)

**Q: What is the single most important invariant in the system?**

A: Crew leads are exactly 3 after bootstrap. That invariant controls who can administer passengers and resources, so breaking it weakens the whole admin model.

---

**Q: What is the single most important audit invariant?**

A: Every valid access attempt emits exactly one `UsageEvent`, whether allowed or denied. Not-found or unauthorised precondition failures do not emit usage events because there is no valid passenger/resource snapshot to record.

---

**Q: Why is soft-delete better than hard-delete here?**

A: Because the system has audit and reporting requirements. If a passenger or resource were hard-deleted, old usage events would point to missing context. Soft-delete preserves history while excluding deleted records from active lists and future access checks.

---

**Q: What does `tier_at_attempt` protect against?**

A: It protects against history rewriting. If a passenger is upgraded later, old denied events still show the passenger's tier at the time of denial.

---

**Q: What does `min_tier_at_attempt` protect against?**

A: It protects against resource policy changes rewriting history. If a resource minimum tier changes later, old events still show the requirement that existed when the attempt happened.

---

**Q: Why not store only current tier and current resource min tier in reports?**

A: Reports would become historically incorrect. Reporting must use event snapshots, not current mutable state.

---

**Q: What does the code gain from `#[non_exhaustive]` on `DomainError`?**

A: It allows adding future error variants without breaking downstream consumers that match on the enum. External callers must include a wildcard arm.

---

**Q: What does the code gain from exhaustive `match` on `Tier`?**

A: If a new tier is added, the compiler points to every place that must be updated: ranking, DTO conversions, reports, tests, and access matrix logic.

---

**Q: If you added a `Diamond` tier, what files would likely change?**

A: `src/domain/tier.rs`, DTO conversions in `src/interface/dto.rs`, reporting pre-population, tests/specs for tier policy and access matrix, and the TypeScript mirror in `web/src/domain/tier.ts`.

---

**Q: What is the difference between authorisation and authentication in this repo?**

A: Authentication is proving who the caller is; this repo does not implement it. Authorisation is checking whether the provided `Actor` is allowed to perform an operation; that is implemented in services via guards and access policy.

---

**Q: Where should authentication be added if needed?**

A: In the interface layer, before constructing `Actor`. The HTTP handler or middleware should validate a token/session, then derive a trusted `Actor` for service calls.

---

**Q: Why is `Actor` passed into service methods instead of stored globally?**

A: It makes authorisation explicit at every call site and keeps tests deterministic. Hidden global identity would make services harder to test and reason about.

---

**Q: What is the biggest limitation of using `Vec` for storage?**

A: Lookups are linear (`O(n)`). Fine for a small demo, but a production system would use indexed storage such as `HashMap` or a database index.

---

**Q: Why does the project avoid `unwrap()` in domain/application code?**

A: Expected failures are part of the domain model and should return `DomainError`. `unwrap()` would turn recoverable business errors into panics.

---

**Q: Where is `expect()` acceptable in this codebase?**

A: In tests and limited infrastructure cases such as poisoned mutex handling, where the state is unrecoverable and a loud failure is preferable to silent corruption.

---

**Q: Why is the domain layer free of serde?**

A: Serialization is an interface concern. Keeping serde out of domain lets the core model stay independent of JSON/API shape and compile without HTTP-related dependencies.

---

**Q: Why are DTOs separate from domain types?**

A: DTOs are wire contracts; domain types are business concepts. Separating them lets the API evolve without forcing business-model changes and keeps validation at the boundary.

---

**Q: What does `#[serde(deny_unknown_fields)]` prevent?**

A: It prevents clients from sending extra/typo fields that would otherwise be silently ignored. This catches mistakes and reduces attack surface.

---

**Q: What is the most suspicious endpoint from a production security perspective?**

A: `/reset`, because it destroys in-memory state. It is fine for a demo but should be removed or strongly protected in production.

---

**Q: What endpoint is most likely to need pagination first?**

A: `/usage`, because every access attempt appends a usage event and the log can grow unbounded.

---

**Q: Why is `/access` a POST instead of a GET?**

A: It causes a side effect by appending a `UsageEvent`. GET should be safe and side-effect-free.

---

**Q: Why does denied access still return an error if an event is recorded?**

A: The event records the attempt for audit, while the `Err(AccessDenied)` tells the caller the requested operation was not allowed. Audit side effect and business result are separate concerns.

---

**Q: What would you change to make event IDs unique across server restarts?**

A: Use database-generated IDs, UUIDs, or a persisted sequence. The current in-memory counter is unique only for the lifetime of one service instance.

---

**Q: What is the danger of using wall-clock time directly in tests?**

A: Tests become nondeterministic and can fail depending on timing, scheduling, or clock changes. Injecting `Clock` keeps tests repeatable.

---

**Q: Why is a `FakeClock` useful beyond tests?**

A: It makes demos reproducible. The same sequence of actions produces the same timestamps, which helps reviewers and tests compare results reliably.

---

**Q: What does `Arc<Mutex<World>>` solve?**

A: It gives all HTTP handlers shared mutable access to the same in-memory service graph while preventing data races.

---

**Q: What problem does `Arc<Mutex<World>>` create?**

A: It serialises all requests through one lock, limiting concurrency and making long handlers block unrelated requests.

---

**Q: Why not use a global static `World`?**

A: A global would hide dependencies, complicate tests, and create lifecycle/reset problems. Explicit state injection through Axum's `State` keeps ownership clear.

---

**Q: What is the best argument that the React demo is not just decoration?**

A: It mirrors the same business rules in TypeScript and includes a live-server panel, so it exercises both the in-browser model and the Rust HTTP contract from a user's perspective.

---

**Q: What is the risk of mirroring rules in TypeScript?**

A: Drift. The TypeScript and Rust implementations can diverge unless tests or generated types keep them aligned.

---

**Q: What one improvement would most reduce frontend/backend drift?**

A: Generate TypeScript API types/client code from the Rust OpenAPI schema.

---

**Q: What one improvement would most improve production readiness?**

A: Add real authentication and persistent storage. If forced to pick one first, authentication, because caller-supplied actor IDs are the highest security risk.

---

**Q: What one improvement would most improve scalability?**

A: Replace the single in-memory `World` with database-backed repositories and request-scoped transactions, then remove the single global mutex bottleneck.

---

**Q: What one improvement would most improve reviewer confidence?**

A: A green CI pipeline running fmt, clippy, Rust tests with HTTP feature, cargo-deny, coverage, and frontend lint/typecheck/tests.

---

**Q: If asked to summarise the project in one technical sentence, what would you say?**

A: It is a spec-driven, layered Rust domain model with deterministic tests, an optional Axum HTTP adapter, and a React/TypeScript demo that exercises the same passenger-resource access rules end to end.

---

## 25. Exhaustive Deep-Dive Question Bank

### Rust Ownership, Borrowing, And Type System

Related files: [src/application/access_service.rs](../src/application/access_service.rs), [src/application/passenger_service.rs](../src/application/passenger_service.rs), [src/application/resource_service.rs](../src/application/resource_service.rs), [src/infrastructure/fake_clock.rs](../src/infrastructure/fake_clock.rs)

**Q: Why do service methods often take owned `String`/ID values on create, but borrowed IDs on lookup/update?**

A: Create methods need to store the values inside service-owned structs, so taking ownership avoids unnecessary clones. Lookup/update methods only need to compare against existing records, so they take borrowed IDs (`&PassengerId`, `&ResourceId`) and avoid moving or cloning caller-owned values.

---

**Q: Why does `create` clone the newly created entity before pushing it into the Vec?**

A: The service both stores the entity and returns it to the caller. Pushing moves ownership into the Vec, so the returned value needs another owned copy. Cloning is explicit, readable, and cheap enough for small structs. In a higher-performance version, the method could push first and return a reference, but that would complicate lifetimes and API ergonomics.

---

**Q: Why do some structs derive `Hash`?**

A: Types used as `HashMap` keys need `Eq + Hash`. `Tier` needs `Hash` for `aggregate_by_tier`, and ID newtypes may need it for future indexed repositories. Deriving `Hash` is correct for immutable value objects whose equality is based on all fields.

---

**Q: Why should IDs be immutable once assigned?**

A: IDs are identity, not editable attributes. Mutating an ID would break references from usage/admin events and any future database foreign keys. If an ID needs to change, that should be modelled as creating/replacing an entity or adding an alias, not editing identity in place.

---

**Q: Why is `Timestamp` a newtype instead of plain `i64`?**

A: A newtype communicates meaning and prevents accidental use of arbitrary integers where a timestamp is expected. It also gives a central place to add formatting, validation, or conversion later without changing every call site.

---

**Q: What does `#[must_use]` buy you on methods like constructors and query methods?**

A: It warns when a caller accidentally ignores a meaningful return value. Ignoring `FakeClock::starting_at(...)`, `Tier::rank()`, or a query result is probably a bug. It is a lightweight static check that improves API ergonomics.

---

**Q: What is monomorphisation and where does it show up here?**

A: Monomorphisation is Rust generating specialised machine code for each concrete generic type. `AccessService<C, S>` and `PassengerService<C>` are generic over clock/sink types; the compiler creates concrete versions for `FakeClock`, in-memory sinks, etc. This gives static dispatch and no vtable cost, at the cost of potentially larger binaries if many concrete combinations exist.

---

**Q: Why use generics for `AccessService<C, S>` but trait objects for optional audit sinks?**

A: The access service always needs its clock and usage sink, so generics give zero-cost abstraction for core behaviour. Audit is optional and would otherwise pollute every service type signature with another generic parameter. A boxed trait object keeps the public service type simpler when audit is disabled, and the dynamic dispatch cost is negligible for audit emission.

---

**Q: What is the downside of using `Box<dyn AdminEventSink>`?**

A: It introduces heap allocation and dynamic dispatch, and it erases the concrete type. That can make some optimisations and tests less direct. It is acceptable here because audit emission is not a hot path and the simpler service type is worth it.

---

**Q: Could `AccessService` return `&UsageEvent` instead of `UsageEvent`?**

A: It could, but it would force the returned reference to be tied to the sink's internal storage and to the borrow of `self`. That complicates lifetimes and exposes storage details. Returning an owned `UsageEvent` is simpler, stable, and decoupled from how events are stored.

---

**Q: Why is cloning an event acceptable here?**

A: The event is small and contains a few strings and copyable values. The clarity of appending one owned event and returning another owned event outweighs micro-optimising clone cost. If events became large, IDs could be interned or stored as `Arc<str>`, but that is unnecessary now.

---

**Q: What Rust feature prevents data races in the HTTP state?**

A: The combination of ownership, `Send + Sync` bounds, and `Mutex` ensures only one mutable borrow of `World` exists at a time. The compiler rejects sharing non-thread-safe types across handlers unless they satisfy the required traits.

---

**Q: Could the project use `Rc<RefCell<T>>` instead of `Arc<Mutex<T>>`?**

A: Not for the HTTP server. `Rc<RefCell<T>>` is single-threaded and not `Send + Sync`; Axum handlers may run on multiple Tokio worker threads. `Arc<Mutex<T>>` is the thread-safe equivalent for shared mutable state.

---

**Q: Why does `AtomicI64` not need a `Mutex` around it?**

A: Atomic operations are already thread-safe and indivisible. The clock only needs to increment a single integer, so a mutex would be unnecessary overhead.

---

**Q: What is a poisoned mutex, and why is it treated as unrecoverable here?**

A: A mutex becomes poisoned when a thread panics while holding it. That means the protected data may be partially mutated. For audit logs or the shared `World`, continuing could silently operate on corrupted state, so panicking loudly is safer than pretending recovery is possible.

---

### Domain And Business Rules

Related files: [src/domain/tier.rs](../src/domain/tier.rs), [src/domain/passenger.rs](../src/domain/passenger.rs), [src/domain/resource.rs](../src/domain/resource.rs), [src/domain/usage_event.rs](../src/domain/usage_event.rs), [specs/01-tier-policy.md](../specs/01-tier-policy.md), [specs/05-access.md](../specs/05-access.md)

**Q: Is `Tier::try_from("gold")` expected to succeed?**

A: No. Tier parsing is case-sensitive and accepts only canonical names (`Silver`, `Gold`, `Platinum`). Rejecting lowercase catches invalid client input at the boundary.

---

**Q: Should `Silver` be able to access `Silver` resources?**

A: Yes. The access rule is `passenger.rank() >= resource.min_tier.rank()`, so same-tier access is allowed.

---

**Q: Should `Platinum` access `Gold` and `Silver` resources?**

A: Yes. Higher tiers inherit lower-tier access. That is the central policy in `Tier::can_access`.

---

**Q: If a resource is downgraded from Platinum to Silver, do old denied events become allowed?**

A: No. Old events remain unchanged because they store `min_tier_at_attempt`. Future access checks use the new min tier.

---

**Q: If a passenger is soft-deleted, should they appear in reports?**

A: Their historical usage events should still appear in reports because audit history is immutable. They should not appear in active passenger lists or be allowed to make future access attempts.

---

**Q: If a resource is soft-deleted, should old personal histories still show it?**

A: Yes. Past usage remains historically true. Soft deletion only affects current listing and future access checks.

---

**Q: Should duplicate IDs be checked across deleted records too?**

A: That is a product decision. The current service primarily rejects active duplicates. If deleted records are retained forever and IDs are stable audit references, reusing a deleted ID can confuse history. For production, I would reject duplicates across both active and deleted records unless the spec explicitly allows resurrection.

---

**Q: Should changing a passenger tier to the same tier emit an admin event?**

A: The current implementation treats tier changes as idempotent but may still emit an event. Both behaviours are defensible. If audit log means "every requested admin mutation," emit it. If it means "only actual state changes," skip no-op events. The spec should decide; tests should encode that decision.

---

**Q: Should deleting an already-deleted passenger return success or not found?**

A: Current service treats soft-deleted passengers as not active, so a second delete should return `PassengerNotFound`. That makes active-state operations explicit. Another reasonable API design is idempotent delete returning success, but it should be specified and tested.

---

**Q: Should `list_accessible_for` include deleted resources?**

A: No. It is a discovery operation for currently usable resources, so soft-deleted resources must be excluded.

---

**Q: Why does reporting group by `tier_at_attempt` instead of current passenger tier?**

A: The report asks what tier was responsible for usage at the time of access. Current tier would rewrite historical activity after upgrades/downgrades.

---

**Q: Why are denied attempts ignored in `top_resources`?**

A: `top_resources` is about actual usage/demand fulfilled, not failed attempts. If the product wanted "most attempted resources," that would be a separate report including denied attempts.

---

**Q: What report would you add next?**

A: I would add denied attempts by resource and tier. It would reveal resources users frequently try to access but cannot, which can inform tier-policy changes or capacity planning.

---

### HTTP, Axum, And API Behaviour

Related files: [src/interface/http.rs](../src/interface/http.rs), [src/interface/dto.rs](../src/interface/dto.rs), [src/bin/serve.rs](../src/bin/serve.rs), [tests/http_access.rs](../tests/http_access.rs)

**Q: Why is the HTTP adapter described as thin?**

A: It translates requests into DTOs/domain values, calls services, and maps results to HTTP responses. It does not implement business rules directly.

---

**Q: What would be a red flag in the HTTP layer?**

A: Duplicating tier checks, directly mutating domain collections, or making access decisions in handlers. That would leak business logic into the interface layer.

---

**Q: Why does the router return JSON DTOs instead of domain structs directly?**

A: DTOs are the external contract. Domain structs can change internally without forcing API changes, and DTOs can carry serde/OpenAPI derives without polluting the domain layer.

---

**Q: What should happen on malformed JSON?**

A: Axum's JSON extractor rejects malformed JSON before the handler runs, returning a 400-style client error. That is boundary validation.

---

**Q: What should happen on unknown JSON fields?**

A: Serde rejects the request because request DTOs use `deny_unknown_fields`. The handler should not run with partially ignored input.

---

**Q: Why does CORS exist in the server?**

A: The React frontend may be served from a different origin during development or deployment. CORS controls which browser origins can call the API.

---

**Q: Why is `CorsOrigins::Any` risky in production?**

A: It allows any website to send browser requests to the API. If the API later uses cookies or bearer tokens, this can widen attack surface. Production should use a restricted allowlist.

---

**Q: Why is request body size limited?**

A: The API expects tiny JSON payloads. A body limit reduces denial-of-service risk from very large request bodies.

---

**Q: Should `POST /passengers` return 200 or 201?**

A: Ideally 201 Created because it creates a new passenger. Returning 200 is usable but less semantically precise. This is a good polish improvement.

---

**Q: Should `DELETE` return the deleted resource or no body?**

A: Either is defensible. 204 No Content is conventional; returning the deleted representation helps clients confirm what was deleted. The API should be consistent and documented.

---

**Q: What is the difference between 400, 403, 404, and 409 in this API?**

A: 400 means invalid request or invalid operation shape, 403 means actor lacks permission/access, 404 means entity not found or inactive, and 409 means conflict such as duplicate ID.

---

**Q: Why should errors include both `code` and `message`?**

A: `code` is stable and machine-readable for clients; `message` is human-readable for debugging and UI display.

---

**Q: Should internal panic messages ever be returned to clients?**

A: No. Production APIs should return generic 500 messages and log internal details server-side.

---

**Q: How does rate limiting work in this codebase?**

A: Per-IP token-bucket rate limiting is implemented via `tower-governor` and wired in `serve.rs` before the router. The defaults are 10 tokens replenished per second (`--rate-limit-rps`) with a burst of 50 (`--rate-limit-burst`). It is **enabled by default** (`PRMS_ENABLE_RATE_LIMIT=true`) and must be explicitly disabled in integration tests (`PRMS_ENABLE_RATE_LIMIT=false`) because all test requests share the loopback IP and would exhaust the token bucket within seconds. The middleware is injected in `serve.rs` without touching any application or domain code — this is the correct layer for cross-cutting concerns.

---

**Q: How would you add idempotency keys for create endpoints?**

A: Accept an `Idempotency-Key` header, store request/result pairs in persistence, and replay the prior result for duplicate keys. This matters for network retries where a client is unsure whether a create succeeded.

---

### Frontend, UX, And TypeScript

Related files: [web/src/App.tsx](../web/src/App.tsx), [web/src/components](../web/src/components), [web/src/domain/errors.ts](../web/src/domain/errors.ts), [web/src/services/api.ts](../web/src/services/api.ts), [web/src/state/store.tsx](../web/src/state/store.tsx)

**Q: Why use TypeScript union types for domain errors?**

A: They give exhaustive checking in frontend code. If a component switches on `DomainError`, TypeScript can help ensure all known cases are handled.

---

**Q: What is the weakness of TypeScript branded IDs?**

A: They are compile-time only. Any string can still arrive from the network at runtime, so API responses should be validated if the client cannot fully trust them.

---

**Q: Why does `getBase()` resolve `VITE_API_BASE` on every call?**

A: It lets tests override environment configuration after module import. If the value were read once at module load, tests would be more brittle.

---

**Q: What should the frontend show when the API is offline?**

A: A clear network error state with retry affordance, not a silent failure. The API client already maps transport problems to `NetworkError`, so components should surface that distinctly from domain errors.

---

**Q: What is the difference between a domain error and a network error in the frontend?**

A: A domain error means the server processed the request and rejected it by business rules. A network error means the request could not complete or the server response could not be reached/parsed.

---

**Q: Why not store everything in localStorage?**

A: localStorage is synchronous, string-only, and not suitable as a source of truth for multi-user data. It is fine for preferences or cached demo state, but persistent business state belongs on the server.

---

**Q: What frontend test would you add first?**

A: A test for the access flow: Silver passenger denied on Gold resource, then upgraded and allowed. It covers domain rules, state mutation, and report/audit updates.

---

**Q: What UI state is easy to forget?**

A: Loading, empty, error, success, disabled, and validation states. Demo UIs often only show the happy path, but production UIs need all of them.

---

**Q: Why does accessibility matter in a code review?**

A: Full-stack work includes user experience. A feature that cannot be used by keyboard or screen-reader users is incomplete, even if the backend is correct.

---

**Q: What accessibility test would you run?**

A: Keyboard-only navigation through all forms, labels checked by screen-reader inspection, contrast checks, and automated axe checks in Playwright.

---

**Q: Why avoid using color alone for access outcomes?**

A: Color-blind users and screen readers may not perceive the distinction. Use text, icons, or status labels in addition to color.

---

### Testing, Coverage, And Quality Gates

Related files: [tests/access.rs](../tests/access.rs), [tests/lifecycle.rs](../tests/lifecycle.rs), [tests/http_access.rs](../tests/http_access.rs), [web/src/services/__tests__](../web/src/services/__tests__), [web/vite.config.ts](../web/vite.config.ts), [Cargo.toml](../Cargo.toml)

**Q: What is the difference between line coverage and region coverage?**

A: Line coverage checks whether a source line executed. Region coverage checks finer-grained branches/expressions within a line. A line can be covered while one branch inside it remains untested.

---

**Q: Why can derive macros affect coverage?**

A: Derive macros generate code that coverage tools can count as regions/functions. Some generated paths, like formatting branches, may be awkward or low-value to test directly.

---

**Q: Is 100% coverage the same as 100% correctness?**

A: No. Coverage says code executed, not that assertions were meaningful or that all behaviours are correct. It is a useful gate, not a correctness proof.

---

**Q: What is mutation testing, and would it help here?**

A: Mutation testing changes code automatically (for example `>=` to `>`) and checks whether tests fail. It would be valuable for tier policy and access rules because those are small logic-heavy functions.

---

**Q: What property-based test would fit this codebase?**

A: Tier ordering properties: reflexivity (`t.can_access(t)`), transitivity, and monotonicity. Another useful property: changing current tier does not mutate existing `UsageEvent` snapshots.

---

**Q: What integration test gives the most confidence?**

A: The lifecycle test: create passenger/resource, deny access, upgrade tier, allow access, query reports, soft-delete, and verify future access fails without rewriting history.

---

**Q: Why test HTTP separately from services?**

A: Service tests prove business rules. HTTP tests prove routing, JSON DTOs, status codes, error mapping, and state wiring. Both layers can fail independently.

---

**Q: Why test TypeScript services separately from React components?**

A: Pure TypeScript service tests are fast and precise. Component tests are slower and better for interaction/rendering behaviour. Separating them keeps feedback tight.

---

**Q: What flaky-test risk exists in this repo?**

A: Shared global state would be the biggest risk, but the repo avoids it. If HTTP tests ever used a real bound port, port collisions could create flakiness; in-process router tests avoid that.

---

**Q: Why is `nextest` preferred over plain `cargo test`?**

A: It runs tests faster and gives better failure output/isolation. `cargo test` remains a useful fallback because it is built into Cargo.

---

**Q: What should CI run before coverage?**

A: Formatting, clippy, and tests. Coverage is useful only after the code compiles cleanly and tests pass.

---

### Security And Threat Modeling

Related files: [src/interface/http.rs](../src/interface/http.rs), [src/interface/dto.rs](../src/interface/dto.rs), [src/application/guards.rs](../src/application/guards.rs), [deny.toml](../deny.toml), [AGENTS.md](../AGENTS.md)

**Q: What is the most realistic abuse case today?**

A: A client impersonates a crew lead by sending a known `actor_id` and then changes passenger/resource state. This is because authentication is simulated.

---

**Q: What is the safest minimal auth improvement?**

A: Add middleware that validates a signed token and maps claims to `Actor`, then remove `actor_id` from request bodies. Services still receive `Actor`, but clients no longer choose it.

---

**Q: What should be logged for security-sensitive actions?**

A: Actor, action, target, timestamp, outcome, request ID, and source identity metadata where appropriate. Avoid logging secrets or full tokens.

---

**Q: Could detailed error messages leak information?**

A: Yes. `PassengerNotFound` vs `UnauthorizedActor` can reveal whether an ID exists. For internal tools this may be acceptable; public APIs sometimes intentionally collapse errors to reduce enumeration risk.

---

**Q: What is ID enumeration and does this API risk it?**

A: ID enumeration is guessing IDs to discover valid entities. Because IDs are plain strings and endpoints return not-found vs success, an attacker could probe IDs if unauthenticated. Auth and non-guessable IDs mitigate this.

---

**Q: Should IDs be UUIDs?**

A: For production, yes or at least non-guessable identifiers. The current string IDs are easy for demos and tests. Newtypes mean the inner representation can later become UUID without changing high-level service APIs too much.

---

**Q: What secret-handling requirement applies here?**

A: No secrets should be committed. If auth is added later, tokens/keys should come from secret management, not source code or checked-in config files.

---

**Q: What would you add to protect admin endpoints?**

A: Authentication, authorization middleware, CSRF protection if cookie-based auth is used, rate limiting, audit logs for all admin attempts, and restricted CORS origins.

---

**Q: Why is CORS not authentication?**

A: CORS is a browser enforcement mechanism controlling which web pages can make cross-origin requests. It does not stop non-browser clients like curl or backend scripts.

---

### Persistence, Data Modeling, And Transactions

Related files: [src/application/ports.rs](../src/application/ports.rs), [src/infrastructure/in_memory_usage_event_sink.rs](../src/infrastructure/in_memory_usage_event_sink.rs), [src/infrastructure/in_memory_admin_event_sink.rs](../src/infrastructure/in_memory_admin_event_sink.rs), [docs/plan-passengerResourceManagement.prompt.md](plan-passengerResourceManagement.prompt.md)

**Q: What table should be append-only?**

A: `usage_events` and `admin_events`. They are audit/history logs and should not be updated or deleted except through retention policies with explicit compliance approval.

---

**Q: Would you normalise tier into a separate table?**

A: Probably not initially. Tier is a tiny closed enum. Storing it as a constrained string or small integer is simpler. A separate table only helps if tiers become configurable business data.

---

**Q: What database constraint protects against duplicate active passengers?**

A: A unique index on `passengers(id)`. If soft-deleted rows remain in the same table and ID reuse is disallowed, the primary key handles it. If ID reuse is allowed, use a different surrogate primary key and a partial unique index for active IDs.

---

**Q: What database constraint protects tier values?**

A: A check constraint such as `tier in ('Silver', 'Gold', 'Platinum')`, or a database enum type. The app should validate too, but database constraints protect data integrity from all writers.

---

**Q: Why should access event insertion be in the same transaction as the permission decision?**

A: To ensure the event snapshot matches the state used for the decision. Without a transaction, another update could interleave between read and write.

---

**Q: What isolation level would you start with?**

A: Read committed plus row locks may be enough for simple access checks. If exact consistency under concurrent tier/resource updates is required, use repeatable read or explicit `SELECT ... FOR UPDATE` on the passenger/resource rows.

---

**Q: How would you migrate from in-memory to database without breaking tests?**

A: Keep service tests using in-memory fakes and add adapter integration tests for the database repositories. The service API should depend on traits so the implementation can be swapped at the composition root.

---

**Q: What data retention question should product answer?**

A: How long usage/admin audit events must be kept. Space mission audit data might be retained indefinitely, but privacy/compliance requirements may impose retention or deletion rules.

---

### Refactoring And Extensibility

Related files: [src/application/passenger_service.rs](../src/application/passenger_service.rs), [src/application/resource_service.rs](../src/application/resource_service.rs), [src/application/crew_lead_service.rs](../src/application/crew_lead_service.rs), [src/application/ports.rs](../src/application/ports.rs), [specs/](../specs)

**Q: What is one refactor you would avoid right now?**

A: Introducing a generic repository abstraction for every entity before persistence exists. The current Vec-backed services are simple and clear. Abstract only when a concrete adapter needs it.

---

**Q: What duplication is acceptable here?**

A: Passenger and resource services have similar lifecycle operations. Some duplication is acceptable because their domain rules may diverge. Prematurely abstracting them into a generic CRUD service could obscure business language.

---

**Q: What duplication might you remove later?**

A: Audit emission helper logic across passenger/resource/crew-lead services, if it grows or becomes inconsistent. A small `AuditEmitter` could reduce repetition without hiding business rules.

---

**Q: How would you add capacities to resources?**

A: Add capacity fields to `Resource`, add a reservation/usage policy in application logic, snapshot relevant capacity state in events if audit requires it, and test over-capacity denial separately from tier denial.

---

**Q: How would you add resource schedules?**

A: Introduce schedule value objects and a scheduling policy in the domain/application layer, inject clock/time ranges for tests, and update access checks to validate both tier and schedule.

---

**Q: How would you add multiple ships or tenants?**

A: Add a `ShipId`/tenant ID to all aggregate roots and events, enforce it in repositories and queries, and derive tenant from authenticated identity rather than request body.

---

**Q: What would you change if tiers became configurable by admins?**

A: `Tier` could no longer be a closed enum. It would become a persisted entity with rank/order, and access policy would compare dynamic rank values. This is a major domain change and would require new specs and tests.

---

**Q: Why is a closed enum good for current tiers?**

A: The prompt defines exactly three tiers. A closed enum gives compile-time exhaustiveness and makes invalid tiers unrepresentable.

---

### Reviewer Presentation And Communication

Related files: [README.md](../README.md), [AGENTS.md](../AGENTS.md), [specs/05-access.md](../specs/05-access.md), [tests/access.rs](../tests/access.rs), [src/application/access_service.rs](../src/application/access_service.rs)

**Q: What is the best file to open first in a review?**

A: `README.md` for the overview and quickstart, then `specs/05-access.md` plus `tests/access.rs` and `src/application/access_service.rs` to show spec-to-test-to-code traceability.

---

**Q: What feature best demonstrates the whole stack?**

A: The access attempt flow. It touches domain tier policy, passenger/resource state, access service, usage event sink, HTTP endpoint, report queries, and React UI.

---

**Q: What is the cleanest demo scenario?**

A: Silver passenger attempts Gold resource and is denied; crew lead upgrades passenger to Gold/Platinum; passenger retries and is allowed; reports show one denied and one allowed event with correct snapshots.

---

**Q: What should you say if you do not know an answer in review?**

A: Be honest, identify the relevant file/spec to check, and explain how you would verify. For example: "I don't want to guess the exact status code; I would check `domain_error_to_response` and the HTTP tests."

---

**Q: What answer style works best in a code review interview?**

A: Start with the direct answer, then explain the trade-off, then point to the file/test/spec that proves it. Avoid long theory unless asked.

---

**Q: What is the best way to handle criticism of a design choice?**

A: Acknowledge the trade-off, explain why it matched the current scope, and describe the upgrade path. For example: "Yes, the single mutex limits concurrency; for this in-memory demo it keeps invariants simple, and the next step is database-backed repositories with transactional writes."

---

**Q: What should you avoid saying in review?**

A: Avoid claiming production readiness if auth/persistence are missing, avoid saying AI wrote code you do not understand, and avoid defending every choice as perfect. Mature reviewers like honest trade-off awareness.

---

**Q: What is the one thing you want the reviewer to remember?**

A: The code is spec-driven and intentionally layered: business rules are pure, tested, and isolated from transport/UI concerns, while the HTTP and React adapters demonstrate that the design can support a full-stack product path.

---

## 26. Adjacent Full-Stack Review Questions

Related files: [README.md](../README.md), [AGENTS.md](../AGENTS.md), [Cargo.toml](../Cargo.toml), [deny.toml](../deny.toml), [web/package.json](../web/package.json), [src/bin/serve.rs](../src/bin/serve.rs)

**Q: If this project became a team-owned service, what documentation would you add first?**

A: I would add an operations runbook: how to start the server, required commands before merging, how to read logs, how to reset demo state, how to troubleshoot common failures, and where each business rule lives. The README is good for reviewers; a runbook is for maintainers. I would also add an architecture decision record (ADR) explaining why the project uses layered architecture, in-memory state for now, and feature-gated HTTP.

---

**Q: What is an ADR and which ADRs would fit this project?**

A: ADR means Architecture Decision Record. It is a short document capturing a meaningful decision, context, alternatives, and consequences. Good ADRs for this repo: "Use clean architecture layers," "Use injected `Clock` instead of wall-clock time," "Use soft-delete for passengers/resources," "Use OpenAPI for HTTP contract," and "Keep persistence out of scope for the core implementation."

---

**Q: How would you onboard a new developer to this repo?**

A: Start with `README.md`, then `AGENTS.md`, then one vertical slice: `specs/05-access.md` → `tests/access.rs` → `src/application/access_service.rs` → `src/interface/http.rs` → React `AccessPanel`. That path shows the codebase's core rhythm: spec, test, service, adapter, UI. Then ask them to add a small test-only change or a new report to practice the workflow.

---

**Q: What code ownership boundaries would you define?**

A: Domain/application code should be reviewed by someone who understands the business rules; interface/HTTP code should be reviewed by someone comfortable with API contracts and Axum; React changes should include frontend review for accessibility and UX. Cross-boundary changes, especially DTO/domain mapping or event schema changes, need both backend and frontend review because they can break the contract.

---

**Q: How would you manage dependency upgrades?**

A: Use Dependabot or Renovate to open small, isolated dependency update PRs. CI should run Rust tests, clippy, `cargo deny`, and frontend tests on every update. For framework upgrades (Axum, Vite, React), read migration notes and test the HTTP/React integration manually because type-compatible upgrades can still change runtime behaviour.

---

**Q: What dependency would you be most cautious adding?**

A: Anything in the domain layer. Domain dependencies become part of the core model and are harder to remove. I would also be cautious with frontend state libraries, ORMs, and auth frameworks: they can reshape architecture quickly. The rule is: add a dependency only if it removes real complexity or provides a well-tested capability we should not build ourselves.

---

**Q: How would you handle a production incident where access checks start denying everyone?**

A: First, check recent deploys and roll back if the issue started after a release. Then inspect metrics/logs for `AccessDenied` spikes, compare affected resource min tiers and passenger tiers, and use request IDs to trace examples. Because usage events snapshot both tiers, the audit log can reveal whether passengers were downgraded, resources were upgraded, or the access policy changed. After mitigation, add a regression test for the exact failure mode.

---

**Q: How would you handle a production incident where audit events stop appearing?**

A: Treat it as high severity because audit is a core requirement. Check event sink errors, storage capacity, recent changes to audit wiring, and whether services were constructed without audit sinks. Add an alert on event append failures and on suspiciously low event volume. In production I would prefer a fail-closed design for audit-critical operations: if the audit sink cannot record a required event, the mutation should fail rather than silently proceed.

---

**Q: Should the system fail open or fail closed if the audit sink is unavailable?**

A: For production, fail closed for admin mutations and access attempts that require audit. The spec says events are required, so allowing operations without audit would violate the system's accountability model. For a demo in-memory sink, failure is unlikely except poisoned state, which already panics loudly.

---

**Q: How would privacy concerns affect this project?**

A: Passenger usage history can be sensitive. Production design should define who can view personal history, how long records are retained, whether names are needed in logs, and whether reports should aggregate/anonymize data. Audit requirements can conflict with deletion/privacy requirements, so product/legal needs a clear retention policy.

---

**Q: Is there personally identifiable information in the project?**

A: Yes, passenger and crew lead names are personal data, even in a fictional setting. In production, logs and exports should avoid unnecessary names, and APIs should enforce authorization around personal histories.

---

**Q: How would you support GDPR-style deletion while preserving audit integrity?**

A: Separate identity data from audit facts. Keep immutable audit events with stable opaque IDs, but delete or anonymize the personal profile fields (name, contact info) when required. This preserves operational history while reducing personal-data exposure. The policy must be explicit because audit retention and deletion rights can conflict.

---

**Q: How would you internationalize the frontend?**

A: Move user-facing strings into a message catalog, use stable message IDs, format dates/numbers through locale-aware APIs, and avoid embedding English labels directly in business logic. Domain enum values used in APIs (`Silver`, `Gold`, `Platinum`) should remain stable contract values; display labels can be translated separately.

---

**Q: What should happen if the frontend and backend versions are incompatible?**

A: The frontend should fail clearly rather than behaving incorrectly. Options: expose a `/version` endpoint, include API version in OpenAPI, and make the frontend check compatibility on load. For breaking changes, deploy backend in a backward-compatible way first, then frontend, then remove old API support later.

---

**Q: How would you do blue/green deployment for this app?**

A: Build two identical environments, route traffic to blue, deploy green, run health checks and smoke tests against green, then switch traffic. Because this app currently uses in-memory state, blue/green would reset state unless persistence is externalized. Real blue/green requires database-backed state or a maintenance/demo-only expectation.

---

**Q: How would zero-downtime deployment change the design?**

A: The server must be stateless or share external persistence, migrations must be backward compatible, and both old and new API versions may need to run during rollout. In-memory `World` is not enough for zero-downtime production because each process has separate state.

---

**Q: How would you design database migrations for this project?**

A: Use an explicit migration tool (`sqlx migrate`, refinery, diesel migrations, etc.). Migrations should be forward-only in production, reviewed like code, and compatible with rolling deploys. Additive changes first (new nullable column), deploy code that writes both old/new if needed, backfill, then enforce constraints in a later migration.

---

**Q: What backup and restore strategy would you propose?**

A: Regular automated database backups, periodic restore drills, and separate retention for audit/event logs. Restore testing matters: an untested backup is only a hope. For audit logs, append-only exports to object storage can provide extra resilience.

---

**Q: How would you support offline-first usage in the React app?**

A: Cache read-only reference data locally and queue mutations while offline with conflict handling. Because access attempts and admin changes are audit-sensitive, offline writes are tricky: each queued event needs a reliable timestamp/ordering and server reconciliation. I would avoid offline writes unless product strongly requires them.

---

**Q: How would you handle clock skew in a distributed production system?**

A: Use server-side timestamps from a trusted clock source, not client timestamps. For strict event ordering across nodes, rely on database sequence IDs or logical clocks rather than wall-clock time alone. The current injected `Clock` abstraction makes this substitution straightforward.

---

**Q: How would you make reports fast on a large dataset?**

A: Add database indexes on `usage_events(passenger_id)`, `usage_events(resource_id)`, `usage_events(tier_at_attempt)`, and `usage_events(outcome)`. For expensive aggregate reports, maintain materialized views or incremental counters updated as events are appended. Cache top-N reports if slight staleness is acceptable.

---

**Q: What indexes would you add first?**

A: `passengers(id)`, `resources(id)`, `usage_events(passenger_id, timestamp)`, `usage_events(resource_id)`, and `usage_events(outcome, resource_id)` for top resources. For admin audit lookup, `admin_events(actor_id, timestamp)` and `admin_events(target_kind, target_id)`.

---

**Q: How would you detect frontend performance issues?**

A: Use browser performance profiling, React DevTools Profiler, Lighthouse, and real-user monitoring for load time and interaction latency. In this app, the first thing to watch is global context re-rendering on every `version` bump.

---

**Q: What frontend bundle concerns exist?**

A: The current frontend is small, but dependencies can grow. Vite build output should be checked for bundle size, unused dependencies, and code splitting if routes/panels grow. The React demo currently has no heavy UI framework, which keeps bundle size modest.

---

**Q: How would you add feature flags?**

A: For frontend-only flags, use build-time or runtime config. For backend behaviour, use typed config injected at the composition root. Avoid scattering environment variable reads through domain/application code; flags should be resolved at the interface/config layer and passed in as typed values.

---

**Q: How would you handle multi-environment configuration safely?**

A: Keep defaults safe for local development, validate config at startup, and fail fast on invalid production config. Use env vars or config files for bind address, CORS origins, log level, and database URL. Secrets should come from secret management, not checked-in files.

---

**Q: What would you include in a pull request description for a feature in this repo?**

A: Spec IDs covered, user-facing behaviour, tests added, design trade-offs, screenshots for UI changes, and verification commands run. For API changes, include request/response examples and note compatibility impact.

---

**Q: How would you review someone else's PR in this repo?**

A: Start from the spec: does the change match the rule? Then check tests: do they cover happy path, denial path, and edge cases? Then review layering: no domain dependency leaks, no business logic in handlers/components. Finally check reviewer DX: naming, readability, docs, and commands.

---

**Q: What makes a code review comment useful?**

A: It identifies a concrete risk, explains why it matters, and suggests a focused fix or asks a clear question. "This is bad" is not useful; "This handler duplicates tier policy already in `Tier::can_access`, which can drift; can we call the service instead?" is useful.

---

**Q: What technical debt would you record intentionally?**

A: In-memory state, caller-supplied identity, manual TypeScript API types, global React context re-rendering, lack of pagination, no persistent event IDs, and CORS defaulting to any origin. These are acceptable for the demo only because each has a clear production upgrade path.

---

**Q: How would you prioritize that technical debt?**

A: Security first (authentication and protected reset), then persistence, then API contract generation, then pagination and performance, then frontend state refinements. The order follows risk: protect data and identity before optimizing.

---

**Q: How would you answer "what did you learn from building this?"**

A: A strong answer: the project reinforced that small domain rules become much easier to maintain when they are written as specs, encoded as tests, and isolated from adapters. It also showed that full-stack work is mostly contract discipline: backend types, API DTOs, and frontend models must agree, or the user experience breaks.

---

**Q: What is one honest weakness in the project that you can own confidently?**

A: It is not production-authenticated or persistent. That is a deliberate scope choice, but it means the system should be presented as a well-structured assignment/demo rather than a deploy-ready product. The value is in the architecture and testability, not in pretending all production concerns are solved.

---

**Q: How would you make the README stronger for a hiring reviewer?**

A: Add a one-page "review path" section: run tests, inspect one spec-to-code trace, start server, try two curl commands, open React demo. Also add AI usage disclosure, status checklist, screenshots or terminal examples, and a short list of known limitations/future work.

---

**Q: What is the most impressive part of the project to highlight?**

A: The spec-to-test-to-service traceability combined with clean adapter separation. It shows both correctness discipline and full-stack extension ability.

---

**Q: What is the least impressive part that you should not oversell?**

A: The in-memory storage. It is appropriate for the assignment and tests, but it is not a production persistence story. Present it honestly as an adapter-backed starting point.

---

**Q: If the reviewer asks for one future feature, what should you choose?**

A: Authentication with trusted actor derivation. It directly addresses the biggest security limitation and strengthens every admin/access endpoint without changing domain rules.

---

**Q: If the reviewer asks for one refactor, what should you choose?**

A: Generate TypeScript API types from OpenAPI. It reduces real cross-stack risk without changing business logic.

---

**Q: If the reviewer asks for one test improvement, what should you choose?**

A: Add an end-to-end Playwright flow through the React UI and live Rust API. It would validate the actual full-stack path users interact with.

---

**Q: If the reviewer asks for one operational improvement, what should you choose?**

A: Add metrics and alerting around access attempts, error rates, latency, and audit event emission. That gives visibility into whether the system is healthy.

---

**Q: If the reviewer asks for one data-model improvement, what should you choose?**

A: Add persistent storage with append-only event tables and indexed passenger/resource tables. That moves the project from demo state toward a real service.

---

**Q: What final answer should you give if asked "is this done?"**

A: "It is done for the scoped assignment levels: passenger/resource management, dynamic access validation, audit logging, reporting, HTTP adapter, and React demo. It is not production-complete because auth, persistence, and full operational hardening are intentionally out of scope or future work."

---

*End of code review Q&A.*
