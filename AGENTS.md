# Agent Instructions — Spaceship X26 PRMS (Rust)

These are the house rules for any AI coding agent (Copilot, Cursor, etc.) or
human contributor working in this repository. Read this file before making
changes.

## 1. Language & Toolchain
- **Rust**, edition **2024**, stable channel pinned via `rust-toolchain.toml`.
- `#![forbid(unsafe_code)]` in the domain module; no `unsafe` anywhere
  outside infrastructure adapters with a written justification.
- Components: `rustfmt`, `clippy`, `llvm-tools-preview` (for `cargo llvm-cov`).

## 2. Architecture
Layered, dependency points **inward only**:

```
interface  →  application  →  domain
                 ↑
           infrastructure
```

- **`src/domain/`**: pure. No I/O, no `SystemTime::now()` / `Utc::now()`,
  no `println!`, no external crate imports beyond `thiserror` / `chrono`.
- **`src/application/`**: services that orchestrate domain + ports. Depend
  on port **traits**, never concrete adapters.
- **`src/infrastructure/`**: concrete adapters (in-memory repos, clock,
  loggers). Implements port traits.
- **`src/interface/`**: CLI / HTTP adapters. Thin. No business logic.
- **`src/bin/`**: executable entrypoints (`cli.rs`, `serve.rs`).

## 3. Core Conventions
- **No panics for expected failures.** Services return
  `Result<T, DomainError>` (built-in `std::result::Result`). `panic!` /
  `unwrap` / `expect` are reserved for unreachable invariants and are
  forbidden in `domain/` and `application/`.
- **Tier comparison** uses `Tier::rank()` — never `==` between variants
  for ordering (use `PartialOrd` if it derives meaningfully, otherwise
  go through `rank()`).
- **Clock** is a trait injected at the composition root. Never call
  `SystemTime::now()` / `Utc::now()` inside `domain/` or `application/`.
- **IDs** are newtype wrappers (`PassengerId(Uuid)`, `ResourceId(Uuid)`, …)
  to prevent mix-ups at the type level.
- **Enums** are `#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]` for
  value objects; `#[non_exhaustive]` on public error enums.
- Every access attempt (allowed **or** denied) emits a `UsageEvent`.
- Port traits live with their consumers (`application/ports.rs`).

## 4. Testing
- **`cargo nextest`** (preferred) or `cargo test`.
- **Test names mirror spec IDs**:
  `#[test] fn tp_r1_s10_rank_silver_is_one() { … }`.
- Unit tests live in `#[cfg(test)] mod tests` blocks alongside the code
  they cover.
- Integration tests live in `tests/` (one file per flow).
- No network, no real filesystem, no real clock in tests.
- **Red → green → refactor.** Commit after green.

## 5. Error Handling
- Validate input at the **boundary** (interface layer) — reject unknown
  tiers, malformed IDs, etc. before they reach services. Use `TryFrom`
  impls for parsing.
- Domain errors are a single `#[non_exhaustive]` enum in
  `domain/errors.rs`, deriving `thiserror::Error`.
- Never swallow errors silently. `let _ = result;` is forbidden.

## 6. Commits
- Conventional Commits: `feat`, `fix`, `test`, `docs`, `refactor`,
  `chore`, `ci`.
- Scope where useful: `feat(tier): …`, `test(access): …`.
- **Each commit should map to one spec ID** where possible (e.g. `TP-R1`).
- Commits are GPG-signed (`git commit -S`).

## 7. Security / Secure Coding
- No secrets in code or history.
- Input validation at boundaries via `TryFrom` / `serde` with
  `#[serde(deny_unknown_fields)]`.
- `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic`
  must pass with zero warnings.
- `cargo fmt --check` is enforced in CI.

## 8. What NOT to do
- Do not add a database, auth system, HTTP server, or UI unless
  explicitly scoped.
- Do not introduce runtime dependencies beyond what's in `Cargo.toml`.
  Justify any new dep in the commit message.
- Do not rewrite the spec files to match code. Update the spec **first**,
  then code.
- Do not add docstrings / comments to unchanged code.
- Do not use `unwrap()` / `expect()` / `panic!` outside test code or
  `infrastructure/` (and only with justification).

## 9. Working with specs
Specs live in `specs/`. Each file has numbered rules (`R1`, `R2`),
invariants (`I1`), and scenarios (`S1`). Workflow per slice:

1. Open the spec file.
2. Generate failing tests named after the scenario IDs.
3. Implement the minimum code to make them pass.
4. Refactor with tests green.
5. Commit with the spec ID in the message.

## 10. SOLID & design patterns
Apply where they reduce coupling. Do not introduce a pattern without a
concrete reason.

- **SRP**: one service per aggregate (`CrewLeadService`, `PassengerService`,
  `ResourceService`, `AccessService`, `ReportingService`). Domain types
  stay free of orchestration.
- **OCP**: extend behaviour by adding new ports/adapters, not by editing
  existing services. New tier? Add the variant; the compiler points at
  every non-exhaustive `match`.
- **LSP**: trait implementors honour the contract — no surprising side
  effects in adapters.
- **ISP**: keep traits small (`PassengerRepo`, `ResourceRepo`,
  `UsageEventSink`) — services depend only on what they use.
- **DIP**: services depend on **traits** (defined in `application/`),
  not on infrastructure structs. Wiring happens at the composition root
  (`src/interface/composition_root.rs`) and is injected into binaries.

Patterns we expect to use:
- **Repository** — abstracts persistence (in-memory now, JSON later).
- **Strategy / Policy** — `AccessPolicy` (tier rule); future audit policies.
- **Result** — `std::result::Result<T, DomainError>` over panics.
- **Composition root** — single place wires services + adapters; injected
  into the CLI / HTTP entrypoint.

Patterns we will **not** use unless justified: ambient singletons,
service locators, deep generic-bound towers, inheritance simulation via
trait objects when a plain enum suffices.

## 11. Reviewer DX
The reviewer must be able to:

```bash
rustup show && cargo nextest run
```

…in under 60 seconds (warm cache) and see all tests green. README must
list this exact sequence. No interactive setup, no env vars required
for the core path.
