# Agent Instructions

## Agent skills

This repository is configured for the Matt Pocock workflow skills and the agent-loop control plane.

- Issue tracker: `docs/agents/issue-tracker.md`
- Triage labels: `docs/agents/triage-labels.md`
- Domain context: `docs/agents/domain.md`
- Planning workflow: `docs/agents/planning-workflow.md`

### Planning workflow

Substantial new work should be planned into GitHub PRD issues instead of implemented directly. See `docs/agents/planning-workflow.md`.

## Architecture authority boundaries

Before implementing a new subsystem locally, check whether an existing foundation is authoritative for it.

- `rune-lanes-core` owns deterministic Rune Lanes rules: match state, commands, queries, legality, domain events, ruleset versioning, and deterministic state evolution. It must remain independent of Axum, SQLite, WebSockets, auth, accounts, React, and hosted-session infrastructure.
- The backend is an application/infrastructure adapter. It may map HTTP/WebSocket DTOs, authenticate actors, persist through ports, and build projections, but it must not become a second rules implementation.
- CQRS is a domain boundary, not an HTTP naming convention. Commands express intent and are the only write path; queries are read-only and must not mutate authoritative state.
- Match persistence is being migrated to event sourcing with derived snapshots. Domain event streams are authoritative; snapshots are disposable checkpoints and must never silently override or repair incompatible/corrupt streams.
- AI policy may choose a legal command, but AI scheduling/advancement is application orchestration rather than a tabletop/domain command.
- Hosted multiplayer lifecycle, seats, reconnect/recovery, and transport should converge on the shared game-server foundation. Rune Lanes remains authoritative for game rules.
- Generic 3D/GPU/camera machinery should converge on the shared 3D/rendering foundation. This repository owns Rune Lanes-specific board presentation semantics and the 2D fallback, not a competing general renderer.
- `input-bindings` owns normalized device input, binding resolution, runtime lifecycle, conflict semantics, profile/persistence semantics, and reusable binding configuration UI. Card Board Hybrid owns Rune Lanes semantic action ids, default bindings, active game/UI contexts, and the functions that execute those actions. Do not add a second key-to-action runtime or conflict engine locally. The current browser integration may use the shared Pages ESM bridge until versioned packages are available.
- `settings` owns generic setting declaration, validation, scope, override, diff, apply-mode, migration, and persistence semantics. Card Board Hybrid owns what Rune Lanes settings mean and how rendering, audio, accessibility, or gameplay-facing systems apply them. Input bindings remain authoritative for keybinding semantics even when controls are presented inside a unified settings screen.
- Existing account-preference DTOs/database rows are compatibility adapters, not a competing settings foundation. Migrate them incrementally and preserve existing saved values while `settings` gains its consumer-facing web/WASM and adapter surfaces.
- Do not introduce ECS, physics, or Maps dependencies merely because those foundations exist; add them only when Rune Lanes has a real authority seam that needs them.
