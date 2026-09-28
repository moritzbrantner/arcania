# Agent Instructions

Rune Lanes: a card/board game hybrid. Heroes summon Units and cast Spells on a shared hex arena.
Rust rules engine + Axum backend + React/Vite frontend.

## Read first

- `CONTEXT.md` — domain glossary. Use its terms exactly (Hero, Unit, Unit armor, Carrier, …) and avoid the listed alternatives.
- `docs/adr/` — read the ADRs touching the area you change before changing it.

## Layout

| Path | Role |
| --- | --- |
| `rune-lanes-core/` | Deterministic game rules: state, commands, queries, legality, events, rulesets |
| `backend/` | Axum HTTP/WebSocket adapter, auth, SQLite persistence, projections |
| `frontend/` | React/Vite client, 3D board with complete 2D fallback, Storybook |
| `e2e/` | Playwright end-to-end tests |

## Commands

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
bun run test          # cargo test --workspace + frontend typecheck + vitest (what CI runs)
bun run build         # cargo build --workspace + frontend build (what CI runs)
bun run test:stories  # Storybook story tests
bun run test:e2e      # Playwright, single worker, slow
scripts/ci-local.sh   # everything above, in order
bun run dev:backend / bun run dev:frontend
```

## Architecture authority boundaries

Before implementing a new subsystem locally, check whether an existing foundation is authoritative for it.

- `rune-lanes-core` owns deterministic Rune Lanes rules: match state, commands, queries, legality, domain events, ruleset versioning, and deterministic state evolution. It must remain independent of Axum, SQLite, WebSockets, auth, accounts, React, and hosted-session infrastructure.
- The backend is an application/infrastructure adapter. It may map HTTP/WebSocket DTOs, authenticate actors, persist through ports, and build projections, but it must not become a second rules implementation.
- CQRS is a domain boundary, not an HTTP naming convention. Commands express intent and are the only write path; queries are read-only and must not mutate authoritative state.
- Keep the CQRS surface explicit: application/player writes belong in `rune-lanes-core/src/commands.rs`; read-only projections and legality inspection belong in `rune-lanes-core/src/queries.rs`. Do not add a generic bus, mediator, repository wrapper, or second read service.
- `rune-lanes-core/src/rules.rs` is authoritative for typed global/turn/arena/Hero rule configuration. Card-template balance belongs in the card catalog. Do not introduce anonymous key/value rule maps or scatter new gameplay constants through backend/frontend adapters.
- Command-availability UI/tooling must call the query facade, which checks the real `GameCommand` path against a clone. Never maintain a second client/backend legality implementation.
- Match persistence is being migrated to event sourcing with derived snapshots. Domain event streams are authoritative; snapshots are disposable checkpoints and must never silently override or repair incompatible/corrupt streams.
- AI policy may choose a legal command, but AI scheduling/advancement is application orchestration rather than a tabletop/domain command.
- Hosted multiplayer lifecycle, seats, reconnect/recovery, and transport should converge on the shared `game-server` foundation. Rune Lanes remains authoritative for game rules.
- Generic 3D/GPU/camera machinery should converge on the shared 3D/rendering foundation (`3d-lab`). This repository owns Rune Lanes-specific board presentation semantics and the 2D fallback, not a competing general renderer.
- Do not introduce ECS, physics, or Maps dependencies merely because those foundations exist; add them only when Rune Lanes has a real authority seam that needs them.

## Work tracking

- GitHub Issues (`moritzbrantner/arcania`) are the work queue. Triage labels: `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`; plus `bug`, `enhancement`, `prd`, `agent-loop:*`.

## Git and merging

- Work on a branch named `agent/<short-topic>`; never commit directly to `main`.
- Open a PR, wait for CI, and merge it yourself with a merge commit (`gh pr merge --merge --delete-branch`) when all checks are green.
- You may also review and merge Renovate PRs, other agents' PRs and the owner's feature PRs once they are reviewed and green.
- Never weaken, skip, or delete a failing test or check to get green.

## Design decisions

- Implement directly; no planning issue is needed first.
- When you make a real architecture decision (new boundary, dependency, persistence/protocol shape, trade-off that is hard to reverse), record it as an ADR in `docs/adr/NNNN-<slug>.md` in the same PR.

## Shared foundations

- If a task needs a change in a shared foundation repo (`game-server`, `physics-engine`, `3d-lab`, …, checked out beside this repo under `~/privat/`), change it there: PR, merge when green, then bump the pinned rev here in the same task. Do not work around a foundation bug locally.

## Testing

- Every behavior change or bug fix comes with a test. For bugs, write the failing test that reproduces it first, then fix.

## Done means

- CI is green: the `ci` workflow runs `scripts/ci-local.sh` (format, Clippy, tests, build, story tests, Playwright e2e); `Validate` runs the fast subset.
- Rule changes have a `rune-lanes-core` test; user-visible flows touched have their e2e/story coverage updated.
- `CONTEXT.md` / ADRs are updated when vocabulary or a decision changed.
