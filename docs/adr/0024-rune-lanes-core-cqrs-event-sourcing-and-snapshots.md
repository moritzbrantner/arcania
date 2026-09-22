# ADR 0024: Rune Lanes core uses CQRS, event sourcing, and derived snapshots

## Status
Accepted

## Context

Rune Lanes rules currently execute inside the backend process. That couples game authority to Axum/SQLite/WebSocket application concerns and makes it too easy for transport, persistence, AI, or presentation code to acquire business rules of its own.

The project also already records transport actions, replay frames, and serialized snapshots. Those records are useful, but replay events are presentation/audit evidence and do not contain enough information to deterministically reconstruct every part of authoritative match state. They therefore must not be relabeled as domain events.

## Decision

Rune Lanes will use a standalone `rune-lanes-core` crate as the authoritative rules boundary.

The write side follows CQRS: callers submit typed `GameCommand` values to the Match aggregate. The aggregate validates intent using domain rules. The target mutation model is `decide -> domain events -> evolve`: rejected commands emit no events and leave state unchanged; accepted commands emit versioned domain events; authoritative state changes only by applying those events.

The read side is separate. `MatchState::queries()` and
`EventSourcedMatch::queries()` return a read-only facade; serializable
`GameQuery` values provide a transport-safe dispatch surface. Queries and
projections never mutate the aggregate or bypass command validation. Legality
inspection uses `GameQuery::CommandAvailability`, which executes the same typed
command rules against a cloned state instead of duplicating them.

Typed rule configuration is explicit. `rune-lanes-core/src/rules.rs` groups the
current arena, turn, and Hero-base rules; card-template values stay in the card
catalog. Gameplay code consumes those typed values directly so changing a rule
does not require hunting for unrelated backend or presentation constants.
Generic key/value rule engines are deliberately out of scope.

Persistence will be event sourced. Each Match has an append-only event stream with aggregate versions for optimistic concurrency and command/idempotency identity for duplicate suppression. Domain-event schema versioning is distinct from game-rules versioning, and a match pins the ruleset version under which it was created.

Snapshots are derived acceleration checkpoints. A snapshot records state after a specific aggregate version and includes schema/rules metadata. It may be discarded and rebuilt from the event stream. A snapshot is never independently authoritative. Corrupt or incompatible recovery fails closed.

During migration, the current in-place rules implementation may be called through an explicitly named compatibility execution path. This is temporary and must not be represented as completed event sourcing. Existing replay/action persistence remains compatibility data until authoritative domain-event emission and event-store adapters replace it.

## Integration boundaries

- Axum, SQLite, auth/account state, and WebSockets remain outside `rune-lanes-core`.
- New application/player writes go in `commands.rs`; new read models, viewer projections, and rule/legality inspection go in `queries.rs`. The legacy `cqrs` module is only a compatibility re-export.
- Global/turn/arena/Hero rule knobs go in typed `rules.rs`; card balance remains in the card catalog.
- game-server will own hosted-session lifecycle, seats, reconnect/recovery, and transport while hosting the Rune Lanes aggregate.
- The shared 3D/rendering foundation owns generic rendering/camera/GPU mechanics; Rune Lanes owns board-specific presentation semantics and the 2D fallback.
- AI policy may select commands; scheduling `AdvanceAi` is application orchestration and is not a domain command.

## Consequences

Rules can be tested and evolved without starting a server or database. Technical adapters can change independently from gameplay. Historical matches remain reproducible across rule changes because ruleset and event-schema versions are explicit. Migration requires converting existing mutating methods incrementally into event decisions/evolution and introducing event/snapshot store ports before the existing SQLite snapshot/action tables can be retired.
