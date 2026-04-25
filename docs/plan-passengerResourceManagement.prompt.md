# Plan — Spaceship X26: Passenger Resource Management System (PRMS)

## 1. Problem Summary
Build a **Passenger Resource Management System** for Spaceship X26 (Earth → Mars settlement mission).

- **Crew Leads** (admins): strictly capped at **exactly 3**. Manage passengers, resources, tier changes, and reports.
- **Passengers**: assigned a membership tier. Can discover and use resources allowed by their tier.
- **Resources**: each has a minimum required tier. Higher tiers inherit lower-tier access.

### Membership tiers (higher inherits lower)
| Tier | Adds | Inherits |
|---|---|---|
| Silver | Food Stations, Sleeping Pods, Basic Hygiene | — |
| Gold | Private Cabins, Adv. Medical Bay | Silver |
| Platinum | Luxury O2 Pods, VIP Rec Deck | Gold + Silver |

---

## 2. Deliverables by Level

### Level 1 — Basic Passenger & Resource Management
- Enforce **exactly 3 Crew Leads** (cannot add a 4th; cannot operate with < 3 if required).
- Crew Leads can CRUD passengers (name, tier).
- Crew Leads can CRUD resources (name, category, min required tier).
- Passengers can **list accessible resources** filtered by their tier (with inheritance).

### Level 2 — Dynamic Access & Validation
- `useResource(passengerId, resourceId)` performs **real-time permission check**; rejects if tier insufficient.
- Crew Leads can **upgrade/downgrade** passenger tiers; changes take effect immediately.
- **Audit log** records every attempted interaction: passenger, resource, timestamp, outcome (allowed/denied), actor if admin action.

### Level 3 — Advanced Reporting & Insights
- **Personal history**: per-passenger list of their resource usage over time.
- **Aggregated reports**: usage grouped by passenger tier (Silver/Gold/Platinum) for Crew Leads.
- **Usage analytics**: top-N high-demand resources (e.g., Luxury O2 Pods) to flag shortages.

---

## 3. Domain Model (planned)

```rust
struct CrewLead   { id: Id, name: String }
struct Passenger  { id: Id, name: String, tier: Tier, deleted_at: Option<DateTime<Utc>> }
struct Resource   { id: Id, name: String, category: String, min_tier: Tier,
                    deleted_at: Option<DateTime<Utc>> }
struct UsageEvent { id: Id, passenger_id: Id, resource_id: Id,
                    tier_at_attempt: Tier, min_tier_at_attempt: Tier, // snapshots — history never rewrites
                    timestamp: DateTime<Utc>, outcome: Outcome }       // Allowed | Denied
struct AdminEvent { id: Id, actor_id: Id, action: AdminAction,
                    target: TargetRef, timestamp: DateTime<Utc>,
                    details: Option<String> }

enum Tier      { Silver, Gold, Platinum }
enum Outcome   { Allowed, Denied }
enum Actor     { CrewLead(Id), Passenger(Id) }            // auth boundary input

// std::result::Result<T, DomainError> is used directly — no custom Result wrapper.
#[non_exhaustive]
enum DomainError {                                         // closed sum, derives thiserror::Error
    UnauthorizedActor, CrewLeadCountInvalid,
    PassengerNotFound, ResourceNotFound, AccessDenied, /* … */
}
```

### Invariants
- `crew_leads.len() == 3` at all times after bootstrap.
- `Passenger.tier` is a `Tier` variant (unrepresentable otherwise).
- `Resource.min_tier` is a `Tier` variant.
- Access rule: `passenger.tier.rank() >= resource.min_tier.rank()`.

---

## 4. Architecture (clean, testable)

```
┌─────────────────────────────────────────────────┐
│ Interface Layer (CLI / REST — pick one)         │
├─────────────────────────────────────────────────┤
│ Application Services                            │
│  - CrewLeadService   - PassengerService         │
│  - ResourceService   - AccessService            │
│  - ReportingService                             │
├─────────────────────────────────────────────────┤
│ Domain                                          │
│  - Entities, Value Objects (Tier), Policies     │
│  - AccessPolicy (tier inheritance rule)         │
├─────────────────────────────────────────────────┤
│ Infrastructure                                  │
│  - In-memory repositories (swappable)           │
│  - Clock abstraction (for deterministic tests)  │
└─────────────────────────────────────────────────┘
```

### Key policies
- **Tier policy** (`domain/tier.rs`): `Silver < Gold < Platinum` ranking via
  `Tier::rank()`; `Tier::can_access(passenger_tier, resource_min_tier)`.
- **Crew Lead count invariant** (CL-I1): enforced inside
  `CrewLeadService::bootstrap` — bootstrap-only, no runtime add/remove.
- **Ports as traits**: `AdminEventSink`, `UsageEventSink`, `UsageEventSource`,
  `Clock` are `trait`s in the application layer; infrastructure provides
  in-memory `struct` adapters implementing them.
- **Audit**: split across two append-only sinks — `AdminEvent` for admin
  mutations (emitted by `AuditEmitter`) and `UsageEvent` for every access
  attempt allowed or denied (emitted by `AccessService`).

---

## 5. Tech Stack (chosen)

- **Rust** stable, edition 2024. `#![forbid(unsafe_code)]` in the domain
  crate; `#![deny(warnings)]` workspace-wide.
- Toolchain pinned via `rust-toolchain.toml` (channel = stable, components
  = `rustfmt`, `clippy`, `llvm-tools-preview`).
- **Testing**: built-in `cargo test` + `cargo nextest` for fast parallel
  runs; coverage via `cargo llvm-cov` with 100% line/region thresholds
  enforced in CI.
- **Lint/format**: `cargo fmt --check` and `cargo clippy --all-targets
  --all-features -- -D warnings -W clippy::pedantic`.
- **GitHub Actions** CI: `fmt` + `clippy` + `nextest` + `llvm-cov` on the
  toolchain from `rust-toolchain.toml`.
- Core path has only minimal deps (`thiserror`, `chrono`/`time`, `uuid`).
  Optional extras (JSON persistence via `serde`/`serde_json`, REST via
  `axum` + `tokio`, React UI) are additive adapters behind cargo
  features, kept out of the core quickstart.

---

## 6. TDD Plan (red → green → refactor)

Unit tests live in `#[cfg(test)] mod tests` blocks alongside the code
they cover; cross-module flows live as integration tests in `tests/`.

### Level 1 tests
- [ ] `Tier::can_access` / `Tier::rank` — matrix of tier vs min_tier
  (in `domain/tier.rs`).
- [ ] `CrewLeadService::bootstrap` — rejects ≠ 3 leads
  (in `application/crew_lead_service.rs`).
- [ ] `PassengerService::create` — assigns tier; rejects non-Crew-Lead actors.
- [ ] `ResourceService::create` — sets `min_tier`; rejects duplicates.
- [ ] `ResourceService::list_accessible_for(tier)` — filtered set with inheritance.

### Level 2 tests
- [ ] `AccessService::use_resource` — Platinum allowed on Platinum-min resource.
- [ ] `AccessService::use_resource` — Silver denied on Gold/Platinum resources.
- [ ] `PassengerService::change_tier` — upgrade/downgrade takes effect immediately.
- [ ] Audit — `UsageEvent` emitted per attempt (allowed + denied); `AdminEvent`
  emitted per successful admin mutation (`tests/audit.rs`).

### Level 3 tests
- [ ] `ReportingService::personal_history(passenger_id)` — insertion-order history.
- [ ] `ReportingService::aggregate_by_tier()` — counts grouped by
  `tier_at_attempt` (snapshot, not current tier).
- [ ] `ReportingService::top_resources(n)` — ranking + deterministic tie-break
  by id; denied attempts ignored.

Target: 100% coverage across all four layers.

---

## 7. Edge Cases & Gotchas
- Attempt to remove a Crew Lead when count is already 3 and none to replace → reject vs require swap op.
- Downgrade a passenger who previously had access — past `UsageEvent`s remain valid history (do not mutate).
- Resource `minTier` changed after provisioning — future access checks use current value; history unaffected.
- Deleting a resource/passenger — soft-delete vs hard-delete? Recommend **soft-delete** so audit trail is intact.
- Concurrency — wrap shared in-memory repos in `Arc<Mutex<…>>` (or
  `parking_lot::Mutex`) and keep critical sections small; guard invariants
  (Crew Lead count) inside the lock.
- Unknown tier string in input — reject at boundary via `TryFrom<&str>`,
  not deep in domain. Use `serde` `#[serde(deny_unknown_fields)]` at HTTP edges.
- Empty reports — return `Vec::new()`, never `Option::None` for collections.
- Clock — inject a `trait Clock` abstraction; never call
  `SystemTime::now()` / `Utc::now()` in the domain.
- No `unwrap()` / `expect()` / `panic!` in `src/domain` or `src/application`
  — all fallible paths return `Result<_, DomainError>`.
- Mark public error enums `#[non_exhaustive]` so adding variants is not a
  breaking change for downstream `match` arms.

---

## 8. Project Layout (planned)

```
passenger_resource_management/
├── Cargo.toml                           # workspace manifest
├── Cargo.lock
├── rust-toolchain.toml                  # stable, rustfmt, clippy, llvm-tools
├── rustfmt.toml
├── clippy.toml
├── src/
│   ├── lib.rs                           # re-exports module tree
│   ├── domain/                          # pure, no I/O, no Utc::now()
│   │   ├── mod.rs
│   │   ├── actor.rs                     # enum Actor (tagged union)
│   │   ├── tier.rs                      # Tier, rank(), can_access()
│   │   ├── passenger.rs
│   │   ├── resource.rs
│   │   ├── crew_lead.rs
│   │   ├── usage_event.rs
│   │   ├── admin_event.rs
│   │   └── errors.rs                    # #[non_exhaustive] enum DomainError
│   ├── application/                     # services + port traits
│   │   ├── mod.rs
│   │   ├── crew_lead_service.rs
│   │   ├── passenger_service.rs
│   │   ├── resource_service.rs
│   │   ├── access_service.rs
│   │   ├── reporting_service.rs
│   │   ├── audit_emitter.rs             # shared admin-event emitter
│   │   ├── guards.rs                    # require_crew_lead()
│   │   └── ports.rs                     # AdminEventSink, UsageEventSink,
│   │                                    # UsageEventSource traits
│   ├── infrastructure/                  # in-memory adapters + Clock
│   │   ├── mod.rs
│   │   ├── clock.rs                     # trait Clock, SystemClock, FakeClock
│   │   ├── in_memory_admin_event_sink.rs
│   │   └── in_memory_usage_event_sink.rs
│   ├── interface/
│   │   ├── mod.rs
│   │   ├── demo.rs                      # scripted scenario (testable)
│   │   └── composition_root.rs          # build_app() — all DI here
│   └── bin/
│       ├── cli.rs                       # thin executable entrypoint
│       └── serve.rs                     # axum HTTP entrypoint (feature-gated)
├── tests/                               # integration tests (one per flow)
│   ├── tier_policy.rs
│   ├── crew_lead.rs
│   ├── passenger.rs
│   ├── resource.rs
│   ├── access.rs
│   ├── audit.rs
│   ├── reporting.rs
│   ├── guards.rs
│   └── demo.rs
├── specs/                               # 01..07 — drive implementation
├── docs/                                # plan + IMPROVEMENTS notes
├── .github/workflows/ci.yml             # fmt + clippy + nextest + llvm-cov
└── README.md
```

---

## 9. README Requirements (must-have for submission)
- Problem statement + assumptions.
- How to run & test (`cargo build`, `cargo nextest run` (or `cargo test`),
  `cargo run --bin cli`).
- Architecture overview + diagram.
- Design decisions & trade-offs.
- Level 1 / 2 / 3 feature checklist with status.
- Example commands or API calls.
- CI badge.

---

## 10. Implementation Milestones
1. **Bootstrap**: repo, `cargo new`, `rust-toolchain.toml`, rustfmt, clippy,
   nextest, llvm-cov, CI pipeline, one trivial passing test.
2. **Level 1**: domain + services + in-memory repo + CLI or seed script; full test suite green.
3. **Level 2**: access validation + tier mutation + audit log; tests green.
4. **Level 3**: reporting services + analytics; tests green.
5. **Polish**: README, diagrams, sample demo script, final review.

---

## 11. Resolved Decisions
- **Interface:** CLI — scripted `run_demo()` + `src/bin/cli.rs` entrypoint.
- **Persistence:** in-memory sinks behind port traits; JSON/DB can be added as
  an adapter without touching domain or application layers.
- **Authentication:** simulated via an `Actor` enum (tagged union)
  passed into every service method; validated at service boundary.
- **Language:** Rust, edition 2024, stable toolchain;
  `#![forbid(unsafe_code)]` in domain; clippy `-D warnings -W pedantic`.
- **Crew Lead lifecycle:** bootstrap-only (exactly 3). No runtime
  add/remove — the invariant can never be violated.
- **Resource capacities:** out of scope (not in the brief).

---

## 12. "Done" Criteria — Status
- [ ] All three levels have passing tests.
- [ ] CI green on a fresh clone (`fmt` + `clippy` + `nextest` + `llvm-cov`).
- [ ] README quickstart: `rustup show && cargo nextest run` (< 60 seconds
  on a warm cargo cache).
- [ ] Code demonstrates: TDD (spec-ID-tagged commits), clean
  architecture (domain → application → infrastructure → interface),
  clear naming, small focused modules, no leaky abstractions.
- [ ] All three §13 above-and-beyond extras delivered (persistence,
  REST API, React UI).

---

## 13. Above-and-beyond — Status
Three optional extras are **planned**. Each will be added on its own
feature branch, merged via `--no-ff`, and landed without changing the
domain layer.

- [ ] **JSON file persistence adapter** —
  `specs/08-persistence.md` (`PE-R1..R6`).
  JSONL admin + usage event sinks (via `serde` + `serde_json`) behind the
  port traits; wired through `build_app(BuildAppOpts {
  admin_sink, usage_sink, .. })`.
- [ ] **REST layer (axum)** —
  `specs/09-http.md` (`HT-R1..R6`). Thin `axum` +
  `tokio` adapter over the application services; `cargo run --bin serve`
  starts the API.
- [ ] **Interactive React UI against the live REST API** —
  `specs/11-web-interactive.md` (`WB-R1..R6`). A full SPA that drives
  every administrative and access-check action over HTTP (CORS enabled
  on the server). Isolated `web/` sub-project (Vite + React 18); Vite
  dev server proxies `/api/*` to `http://localhost:3000`.
- [ ] **Built-in demo world** —
  `specs/12-demo-seed.md` (`DS-R1..R5`).
  Canonical population from the glossary: 3 Crew Leads, 3 Passengers
  across every tier, 6 onboard facilities. Exposed as `seed_demo_world`
  (reused by `cargo run --bin serve -- --seed` / `PRMS_SEED=1`) and as
  a “Load demo data” button in the React bootstrap screen that composes
  the existing REST endpoints — no new server route.

Guardrails:
- Core (Levels 1–3) must be green before any of these start.
- Each addition gets its own spec file (`specs/08..12`) written first.
- Domain tests must not be modified; 100% coverage maintained throughout.

---

## 14. Submission requirements (from challenge email)
The reviewer expects a ZIP (or Drive link) and will judge the code
**as if written by an experienced engineer** — AI usage is allowed and
expected, but does not lower the bar.

### Explicit grading values
- **SOLID principles** — apply where they reduce coupling; do not
  over-engineer.
- **Test-driven development** — red → green → refactor visible in commits.
- **Clean, readable code** — small functions, clear names, no dead code.
- **OOP & design patterns where appropriate** — services as classes with
  injected ports; Repository, Strategy/Policy, and Result patterns are
  natural fits here. Do not introduce patterns for their own sake.
- **Reviewer DX** — "assume someone else will read, run, and review your
  solution and make that as easy as possible."

### Mandatory deliverables in the ZIP
- `README.md` with:
  - One-paragraph problem statement.
  - **Quickstart**: `rustup show && cargo nextest run` (≤ 3 commands).
  - Architecture diagram (the layered ASCII block is enough).
  - Spec → test → code traceability table (link to `specs/`).
  - Level 1 / 2 / 3 feature checklist with status.
  - **Design decisions & trade-offs** section.
  - **AI usage disclosure** (see below).
- `AGENTS.md` — engineering conventions.
- `specs/` directory — drives implementation.
- `.github/workflows/ci.yml` — fmt + clippy + nextest + llvm-cov.
- `rust-toolchain.toml` — pins Rust toolchain.
- Clean git history with conventional commits, signed where possible.
- No `target/` in the ZIP. Include `Cargo.lock`.

### AI usage disclosure (required by email)
Add a section to `README.md` titled `## AI Usage Disclosure` covering:
- Tools used (e.g., GitHub Copilot in agent mode, model name).
- Where AI helped most (spec drafting, test scaffolding, boilerplate).
- What was reviewed/rewritten by hand (domain rules, invariants,
  service boundaries).
- What was rejected and why (e.g., over-engineered patterns, unsafe
  shortcuts).
- Verification done independently of the AI (running tests, reading
  diffs, manual edge-case reasoning).

### Pre-submission checklist
- [ ] `rustup show && cargo nextest run` succeeds on a fresh clone.
- [ ] `cargo fmt --check` and
  `cargo clippy --all-targets --all-features -- -D warnings` are clean.
- [ ] `cargo llvm-cov --fail-under-lines 100` passes.
- [ ] CI badge in README is green.
- [ ] All public items have purpose-driven names; no commented-out code.
- [ ] No secrets, tokens, or personal paths committed.
- [ ] README quickstart verified by following it line by line.
- [ ] AI Usage Disclosure section present and honest.
- [ ] ZIP excludes `target/`, `coverage/`, `.DS_Store`,
  `.git/objects/pack/*` if size matters (keep `.git` for history if
  reviewer values it; otherwise strip).

### Future work — drive coverage to 100%
Current snapshot: `cargo llvm-cov --summary-only` reports **lines
99.63% / regions 98.38% / functions 100%**. The remaining gaps are
all *region-level* (branch arms inside otherwise-covered lines), not
unreached functions:

- `application/crew_lead_service.rs` — 96.94% regions. Likely
  uncovered: the duplicate-id branch in `replace_audited` + a
  `Result::Err` propagation path. Add a test where `replace_audited`
  is called on an audited service with a colliding `new_lead.id`.
- `application/passenger_service.rs` — 97.69% regions. Likely
  uncovered: the `audit.is_none()` early-return inside `emit` (no test
  exercises a service constructed *without* `with_audit`). Add unit
  tests in a `#[cfg(test)] mod tests` block covering create / change /
  soft-delete on a sink-less service.
- `application/resource_service.rs` — 97.86% regions. Same gap as
  `passenger_service`; mirror the tests.
- `domain/tier.rs` — 98.04% regions. Likely the `InvalidTier` `Debug`
  / `Error` derive expansion. Either add an assertion that exercises
  `format!("{:?}", InvalidTier(...))` and `Display`, or accept the
  derive overhead as untestable.

Plan to reach 100%:
1. Add the missing-audit-sink unit tests to both passenger and
   resource services (covers ~8 of the 13 missing regions).
2. Add the duplicate-id `replace_audited` test for crew leads.
3. Add a tiny `tier::tests` case asserting
   `format!("{}", InvalidTier("x".into()))` matches the
   `#[error("invalid tier: {0:?}")]` template.
4. Wire `cargo llvm-cov --fail-under-regions 100 --fail-under-lines 100`
   into CI so regressions block merges.


