# Agent Instructions — Spaceship X26 PRMS (Rust)

These are the house rules for any AI coding agent (Copilot, Cursor, etc.) or
human contributor working in this repository. Read this file before making
changes.

## 0. Agent Operating Loop
- Start from the user's requested behavior, then locate the nearest spec,
  test, service, or interface boundary that describes it.
- Use the project vocabulary from `specs/00-glossary.md` in test names,
  issue notes, explanations, and new domain concepts.
- Prefer a thin vertical slice over broad horizontal work. A good slice is
  independently verifiable and touches only the layers needed for that behavior.
- Before changing code, identify the fastest reliable feedback loop: focused
  unit/integration test, HTTP test, CLI command, or existing acceptance flow.
- If you do not understand the area, zoom out first: map the relevant modules,
  callers, ports, adapters, specs, and tests before proposing edits.
- Keep working notes tied to evidence. If a hypothesis is not falsifiable by a
  command, test, or code read, sharpen it before acting on it.
- Do not leave temporary debug code, prototype code, or generated artifacts in
  the main path unless the user explicitly asks to keep them.

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
- Tests should verify observable behavior through public interfaces, not private
  implementation details. Integration-style tests are preferred when they cover
  the real behavior without excessive setup.
- Work in red-green-refactor cycles: write one failing test for one behavior,
  implement the minimum production code to pass, then repeat. Do not write a
  batch of speculative tests before implementation has taught you the shape of
  the code.
- Never refactor while tests are red. Refactor only after the current behavior
  is green, and run the relevant tests after each meaningful refactor step.
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
2. Choose one tracer-bullet behavior that can pass through the necessary layers
  end to end.
3. Generate one failing test named after the scenario ID.
4. Implement the minimum code to make that test pass.
5. Refactor with tests green.
6. Repeat for the next behavior.
7. Commit with the spec ID in the message.

If the requested behavior is not covered by an existing spec, propose the spec
change first. Do not rewrite specs merely to match existing code.

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

## 11. Debugging & Diagnosis
For bugs, regressions, flaky behavior, or performance issues, follow a
diagnosis loop before fixing:

1. Build a fast, deterministic feedback loop that reproduces the reported
   symptom. Prefer a failing test; otherwise use an HTTP script, CLI command,
   captured fixture, or minimal harness.
2. Confirm the loop reproduces the user's actual failure, not a nearby failure.
3. List 3-5 ranked, falsifiable hypotheses. Each hypothesis should predict what
   observation or change would confirm or disprove it.
4. Instrument only at decision points that distinguish the hypotheses. Use
   uniquely tagged temporary logs such as `[DEBUG-access-001]`, then remove them
   before finishing.
5. Convert the minimized reproduction into a regression test at the correct
   public seam before applying the fix whenever such a seam exists.
6. Re-run the original reproduction and the new regression test before declaring
   the bug fixed.

If no correct test seam exists, document that as an architectural finding rather
than adding a misleading shallow test.

## 12. Architecture Improvement
- Treat a module as its interface plus implementation. The interface includes
  types, invariants, error modes, ordering constraints, and configuration that
  callers must understand.
- Prefer deep modules: small, stable interfaces that hide meaningful behavior
  and concentrate change. Be suspicious of shallow pass-through modules whose
  interface is as complex as their implementation.
- Use the deletion test before proposing abstractions: if deleting a module only
  moves the same complexity into callers, it was not earning its keep.
- Add or preserve seams where they improve locality, testability, or adapter
  substitution. One concrete adapter is only a possible seam; two adapters make
  the seam real.
- Do not perform architecture rewrites while delivering a behavior slice unless
  the rewrite is necessary for that slice. Surface larger opportunities as
  follow-up work with files involved, problem, proposed change, and expected
  testing benefit.

## 13. Planning, Issues, and Prototypes
- Break larger plans into independently verifiable vertical slices. Each slice
  should describe end-to-end behavior, acceptance criteria, dependencies, and
  whether it needs human input.
- For PRD or issue text, avoid fragile file-path-heavy implementation plans.
  Capture durable decisions: affected modules, public interfaces, domain rules,
  API contracts, schemas, and testing decisions.
- Prototype only to answer a specific question. Mark prototype code clearly as
  throwaway, keep it close to the relevant module or UI route, and provide one
  command to run it.
- Prototype state should be in-memory by default. Skip production polish,
  persistence, broad error handling, and abstractions unless they are the thing
  being tested.
- When the question is answered, delete the prototype or absorb the validated
  decision into production code, tests, specs, or an ADR.

## 14. Reviewer DX
The reviewer must be able to:

```bash
rustup show && cargo nextest run
```

…in under 60 seconds (warm cache) and see all tests green. README must
list this exact sequence. No interactive setup, no env vars required
for the core path.
