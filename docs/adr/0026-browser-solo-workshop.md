# ADR 0026: Browser Solo matches and a local card workshop

## Status
Accepted

## Decision

GitHub Pages cannot host Axum. `rune-lanes-browser` is a WebAssembly application adapter over `rune-lanes-core`, with no server, auth, database, or network dependency. It selects commands with the existing baseline AI policy and executes them through `GameCommand`; its Player legality projection uses the query facade. Existing hosted flows retain their HTTP adapter. The special `browser-solo` match ID selects the browser adapter explicitly; network failures never silently switch an existing match to local play.

Rules are frozen per Match in the typed `RuneLanesRuleset`, including in compatibility snapshots. Existing constructors and historical snapshots use the current default. The workshop validates bounded numeric rule values and custom `CardDefinition` values in Rust. Custom cards use existing effect semantics and are given to both opening hands so users can try them immediately. They do not alter the hosted card catalog or account deck legality.

Browser persistence is a versioned setup plus ordered accepted commands, replayed through the core on reload. This is an application command journal, not the domain event stream described in ADR 0024. There is no authoritative browser snapshot or silent repair of incompatible saves. One browser Solo match is retained; starting another replaces it. Rules/card presets persist separately, support JSON import/export, and affect new matches only. Local storage write failures roll back the attempted move. These experiments do not award account progression.

The Pages workflow builds the WASM with the pinned wasm-bindgen CLI and runs desktop/mobile tests against the built site at `/arcania/` without a backend. After deployment, it checks the published commit in `deployment.json` and repeats those flows on the public Pages URL. Client-route reloads use Pages' `404.html` fallback; tests require restored state and successful assets even though the document has a 404 status. Presentation assets retain their asset-tooling specs, receipts, revision, and replay verification alongside the intentionally distributed SVG.

## Consequences

Browser users can play against bots without signing in or operating a server. Core rule changes must retain default hosted behavior and have core coverage. Seeded shuffles take the modulus before narrowing to a pointer-sized integer so native and wasm32 hosts agree. Spell and Item AP effect fields serialize as `maxAp` for the client; `max_ap` remains an accepted alias for older persisted data. New effect semantics belong in the core, not in editable JavaScript. A future incompatible engine/journal change must version or migrate browser saves explicitly. Hosted multiplayer, account progression, and server archives still require the backend.
