# Physical-Rules-First Command/Query Kernel

Rune Lanes has one authoritative game: the tabletop rules and the software rules are the same domain semantics. The backend acts as an executor and referee for those rules rather than defining a second software-only interpretation.

Player intent is expressed as typed `GameCommand` values. Application orchestration, including advancing the Solo AI, is not a game command; AI chooses and submits the same player commands that human actors use.

Rule evaluation is split into a non-mutating decision step and an explicit state commit/evolution step. A rejected command therefore cannot mutate authoritative match state or reach persistence. Read-only legality queries reuse the same decision path on cloned state instead of reimplementing rules separately.

Stable `RuleId` values identify rule failures across engine tests, future UI explanations, and the physical rulebook. Rule identity is derived from command context when legacy errors conflate mechanics—for example, movement adjacency and attack range—so `MatchError` remains a compatibility surface rather than becoming the normative rule vocabulary. Existing `MatchActionRequest`, replay frames, snapshots, persistence, and HTTP contracts remain compatibility boundaries during the staged migration.

The first migration slice routes existing player actions through this kernel while delegating their underlying mechanics to the established deterministic `MatchState` implementation. Turn-phase transitions, movement, and attacks are the first mechanics covered by kernel invariants. Cards, stack resolution, effects, and physical component generation can migrate in later slices without requiring a rewrite.
