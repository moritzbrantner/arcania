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
| `rune-lanes-cli/` | `rune-lanes` dev CLI: deterministic match script runner over the core command/query path |
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
cargo run -q -p rune-lanes-cli --bin rune-lanes -- play move-unit script.jsonl  # check rules by script (README "Match scripts")
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
- `input-bindings` owns normalized device input, binding resolution, repeat policy, text-entry exclusion and runtime lifecycle. Rune Lanes owns its semantic hotkey commands, default bindings, active surface handlers and Account preference storage. Do not add a second key-to-action runtime locally (ADR 0025).
- Do not introduce ECS, physics, or Maps dependencies merely because those foundations exist; add them only when Rune Lanes has a real authority seam that needs them.

## Shared conventions

General engineering rules (git and merging, commits, testing, ADRs, docs, dependencies, Rust style, …) come from `coding-agent-conventions`, installed in `.conventions/`. Read the rule briefing in `.conventions/index.md` before implementing and open the linked source when a rule applies. Do not edit `.conventions/`; refresh it with `coding-tooling conventions update`. Rules below are repository-specific additions or exceptions.

## Shared foundations

- Shared foundations (`game-server`, `physics-engine`, `3d-lab`, …) are checked out beside this repo under `~/privat/`. Fix defects there and bump the pin here (DEP-003).
- `input-bindings` packages are pinned in `frontend/package.json` to exact commits of its `dist/*` branches, never a branch name or the Pages bundle URL. To bump, take the commits from the `Package distribution` run summary on `input-bindings` `main` and `bun add` all three packages at matching commits.

## Done means

- CI is green: the `ci` workflow runs `scripts/ci-local.sh` (format, Clippy, tests, build, story tests, Playwright e2e); `Validate` runs the fast subset.
- Rule changes have a `rune-lanes-core` test; user-visible flows touched have their e2e/story coverage updated.
- `CONTEXT.md` / ADRs are updated when vocabulary or a decision changed.
