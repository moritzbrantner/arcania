# Card Board Hybrid

Rune Lanes is a small vertical slice for a card game / board game hybrid.

Players control Heroes on a radius-3 hex arena. Cards spend Mana only, including when the Hero has no action points left. Heroes and Units spend their own action points to move across adjacent Hexes, attack, or activate effects.

A turn proceeds through Movement Phase → Attack Phase → Card Play. During Movement, the active side may move pieces and play otherwise-legal Cards; a newly summoned Unit can move with its partial entry action points. Advancing commits to Attack, where proactive Cards are unavailable. Final Card Play lets the side spend remaining Mana on Cards before ending the turn.

Pending stack actions freeze ordinary movement, attacks, proactive Cards, and phase changes. The priority holder may make an eligible higher-priority response or pass; when the stack empties, play resumes in the same phase, subject to normal phase completion. Older matches retain the Card windows pinned to their ruleset.

## GitHub Pages

The Pages workflow publishes the frontend as a static preview at
`https://moritzbrantner.github.io/arcania/`.

The public dashboard, rules/wiki, and fully local tutorial work without the Rust
backend. Account authentication, the backend-driven catalog, persisted matches,
and shared multiplayer still require the backend and are intentionally not
reimplemented in the Pages build.

## Stack

- Rust backend with Axum
- React frontend with Vite and TypeScript
- Bun for frontend package scripts

## Commands, queries, and rules

Rune Lanes uses CQRS as an in-process domain boundary around the event-sourced
match aggregate. It does not use a generic command bus, mediator, repository
layer, or separate read service.

- Player/application writes are typed `GameCommand` values in
  `rune-lanes-core/src/commands.rs`. Persisted matches enter through
  `EventSourcedMatch::decide`, append a domain event, and change authoritative
  state only through `evolve`.
- Reads live in `rune-lanes-core/src/queries.rs`. Call `state.queries()` or
  `aggregate.queries()` for the read-only facade, or dispatch a serializable
  `GameQuery`. The same query side owns viewer-scoped match projection and
  command-availability checks.
- Global arena/turn rules and Hero base stats live together in
  `rune-lanes-core/src/rules.rs`. Card/template balance remains in the card
  catalog. These typed files are the normal places to edit when experimenting
  with gameplay rather than scattering numeric rules through handlers.
- `GameQuery::CommandAvailability` evaluates the real command rules against a
  cloned state. Frontend/backend adapters should use that instead of recreating
  legality rules for buttons, hints, or AI tooling.
- `GameQuery::LegalCommands` lists every concrete command a side could submit
  now. It enumerates candidates structurally (Cards against every Hex and
  piece, pieces against every Hex and piece, Items, Buildings, turn-flow
  commands) and keeps only those that `CommandAvailability` accepts.
- AI-lab rule presets remain an experiment/scenario layer. They consume the core
  ruleset defaults and may override a test setup without becoming a second
  production rules authority.

For a quick rules experiment, edit `CURRENT_RULESET` in
`rune-lanes-core/src/rules.rs`, run the core tests, and use the ruleset and
command-availability queries to inspect the resulting behavior.

## Match scripts

`rune-lanes play` runs a few lines of JSON against the real rules engine, which
is quicker than clicking through the UI to check a rule:

```sh
cargo run -q -p rune-lanes-cli --bin rune-lanes -- play move-unit script.jsonl
cargo run -q -p rune-lanes-cli --bin rune-lanes -- play script.jsonl --json
cargo run -q -p rune-lanes-cli --bin rune-lanes -- setups
```

The setup is a dev match scenario id (`move-unit`, `adjacent-attack`,
`play-unit-card`, ...; ADR 0010) or `seed:<n>` for a fresh seeded Solo match. A
script can name its own setup on its first line (`{"setup": "move-unit"}`). A
setup given on the command line takes precedence.

Each remaining line is one of:

- a `GameCommand` in its normal serde form, for example
  `{"type": "movePiece", "pieceId": "player-unit", "to": {"q": -1, "r": 0}}`.
  It is submitted for the acting side (the priority holder while a stack is
  pending, otherwise the Active side). Add `"side": "opponent"` to submit it as
  another side.
- `{"advanceAi": "opponent"}`, which takes one baseline Solo AI policy step the
  way the backend's `advanceAi` does. Add `"untilDone": true` to repeat until
  the AI finishes its turn or has to wait for another side.
- `{"expect": {...}}`, an assertion. Supported checks: `phase`, `activeSide`,
  `prioritySide`, `round`, `winner`, `mana` (`{"player": 7}`), `pieces` (by
  Hero or Unit id, a subset of its fields, or `null` when it should be gone),
  `state` (a subset of the authoritative snapshot JSON), `legal` / `illegal`
  (commands, optionally `{"command": ..., "rejection": "wrongPhase"}`),
  `lastResult` (`"accepted"` or an error code), and `side` (the side used for
  `legal`/`illegal`).
- blank lines and `#` or `//` comments.

Commands go through `EventSourcedMatch::decide`/`evolve`, the same path the
backend persists. Legality comes from the query facade
(`command_availability` / `legal_commands`). The runner owns no rules. For
each step the output shows the command, whether it was accepted (with the
domain event) or rejected (with the `MatchError`), the replay events it
produced, and a short state digest. The legal commands for the acting side are
listed at the end. Output has no timestamps, and it is deterministic for a
given setup and script. Exit codes: `0` means every expectation passed, `1`
means an expectation failed, and `2` means a usage, script, or setup error.

Example scripts and their expected output are in
`rune-lanes-cli/tests/scripts/`. `cargo test --workspace` replays them. After
an intentional rules or format change, regenerate the expected output with
`UPDATE_EXPECT=1 cargo test -p rune-lanes-cli --test scripts` and review the
diff.

## Run

Install frontend dependencies:

```sh
bun install
```

Start the backend:

```sh
bun run dev:backend
```

Start the frontend in another shell:

```sh
bun run dev:frontend
```

The frontend proxies `/api` to `http://localhost:4000`.

## Routes

- `/` opens the public dashboard.
- `/play` opens the public match picker. Signed-out players can create anonymous
  solo matches, open a match by ID, create private shared-match seat links, or
  link to `/catalog/` when the backend is available.
- `/login` opens the public sign-in route. `/login/` is treated equivalently,
  and successful sign-in returns to a safe same-origin `next` path or `/profile`.
- `/register` opens the public account creation route. `/register/` is treated
  equivalently, and successful registration signs the player in before returning
  to a safe same-origin `next` path or `/profile`.
- `/profile` is account-only. Signed-out players are redirected to
  `/login?next=/profile`.
- `/decks` is the account-only deck library and deck builder. Signed-out
  players are redirected to `/login?next=/decks`.
- `/matches` is the account-only match archive. Signed-out players are
  redirected to `/login?next=/matches`.
- `/match/<match-id>` opens the public playable Rune Lanes board for a persisted
  solo match.
- `/match/<match-id>/<seat-token>` opens a private shared-match seat link. Seat
  links are capability URLs and do not require an account.
- `/matches/<match-id>/replay` opens a public replay route when the backend
  exposes that replay.
- `/catalog/` opens the public backend-driven starter card catalog.

## Deck building

Signed-in accounts can save named deck recipes. Draft recipes can be saved while
they are incomplete, but only legal recipes can be selected for a match. New and
anonymous play can always fall back to the system starter recipe, and solo
matches can choose from predefined AI deck recipes.

## Match persistence

The backend creates matches through `POST /api/matches`, loads them through
`GET /api/matches/:matchId`, and applies playable actions through
`POST /api/matches/:matchId/actions`. New matches receive short readable IDs
such as `rl-lx5n2w`, and the full match snapshot is stored in SQLite after
creation and after each successful action.

SQLite data defaults to `data/rune-lanes.sqlite3`, which is ignored by git. Set
`RUNE_LANES_DB_PATH=/path/to/rune-lanes.sqlite3` to use a different database,
including isolated temporary databases for tests or local experiments.

## Card evidence reports

Run a self-contained paired Card experiment through the existing core AI-lab path:

```sh
cargo run -q -p backend --bin ai-lab -- compare-cards \
  --input backend/config/card-comparison.example.json --out target/ai-lab/cards
```

The example compares exact revisions with costs of one and four Mana at two seeds and writes `card-comparison.json`. Each input includes complete candidate/baseline definitions and source identities, seeded Card test scenarios, the policy configuration and selected policy IDs, the tested Duel side and a positive action limit. Cases must supply an empty authored Card list. The runner adds one input Card to the tested side's hand and uses that exact definition for its matching draw-pile copies.

The report preserves the input, core default ruleset version and compiled built-in definitions. It shows paired play timing/counts, surviving new matching Units, outcomes and event-total deltas. Damage/healing totals are recorded amounts across both sides, and draws include turn draws; none is attributed to a source Card. Timeouts and illegal actions are explicit. There is no balance score or automatic promotion. Rerun `input` from the report with a compatible core/catalog to reproduce it. The dev CLI trusts supplied source provenance.

Authenticated Accounts can `POST /api/card-evidence/run` with the same scenario/policy fields, replacing each candidate/baseline snapshot with an exact source reference:

```json
{
  "candidate": { "type": "draft", "draftId": 12, "version": 3 },
  "baseline": { "type": "published", "revisionId": { "cardId": "custom-example", "revision": 1 } }
}
```

This source-reference example omits the required `cases`, `policyConfig`, `playerPolicyId`, `opponentPolicyId`, `testedSide` and `maxActions` fields shown in the CLI input. Either source may reference an owned Draft or an accessible exact published revision (including compiled Cards and imported copies). Definitions come from storage. Drafts use their stable catalog ID in the experiment; all other definition fields are preserved. Stale Draft versions return 409, missing or inaccessible sources return 404, invalid experiments return 400, and incompatible stored revisions fail with 500. The application accepts at most eight cases and 300 actions per game, releases storage before self-play, and returns the same report format without changing source content or hosted matches.

## Shared multiplayer

The match picker can create a solo AI match or a multiplayer match. Multiplayer
matches create private seat links for Player and Opponent. The creator shares the
invite link, the invitee chooses a hero, and both browsers play over a
server-authoritative WebSocket connection. Active multiplayer matches are only
viewable from their seat links; completed matches can be replayed from the
archive.

## Production serving

For a single-origin production run, build the frontend and start the backend:

```sh
bun run --cwd frontend build
cargo run -p backend
```

The backend serves `/api`, WebSocket routes, and built frontend files from
`frontend/dist`, falling back to `index.html` for app routes.

## Checks

E2E tests start their own frontend server and reject an occupied port. Set `ARCANIA_E2E_PORT` to an unused TCP port when another app or a local dev server uses the default 5173:

```sh
ARCANIA_E2E_PORT=18573 scripts/ci-local.sh
```

```sh
bun run ci:local
```

`bun install` configures Git to run `.githooks/pre-push`, which executes the
same local CI script before a push reaches GitHub. If GitHub Actions cannot run
because of a billing or spending limit, a passing `bun run ci:local` is the
project's local signal that the shared CI workflow would have passed.

## GitHub Pages Solo play and card workshop

Open the Pages site to play against a bot without an account or backend. The workshop offers all eight Heroes, editable match/Hero rules, a card editor (Unit/Spell fields plus an advanced editor for existing effects), preset import/export, and automatic resume of the latest match. Settings links to match rules and the card editor. Custom cards appear in both opening hands. Saved presets affect new matches; hosted matches and account progression remain separate.

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129 --locked
bun install --frozen-lockfile
bun run build:pages
VITE_BASE_PATH=/arcania/ bun run --cwd frontend dev:preview
bun run test:pages
```

For local frontend development, run `bun run build:browser-engine` once and open `/workshop`. Rebuild the engine after Rust changes. `build:pages` targets `/arcania/`; the Pages workflow derives the repository name automatically and includes its own WASM build and browser tests. The workflow deploys only from `main`.

The workflow also tests the public site after deployment on desktop and mobile, including bot turns, custom cards, rule persistence, reload, and victory. Run the same checks without a local server:

```sh
ARCANIA_PAGES_URL=https://moritzbrantner.github.io/arcania/ bun run test:pages
```

Set `ARCANIA_PAGES_REVISION` to a full commit SHA to also verify `deployment.json` before testing. GitHub Pages serves the app's `404.html` for client routes; reload checks allow that document status while still rejecting failed assets and requiring the saved match to render.

Browser storage is local to the current browser/origin. Export a preset to keep a portable backup of rules and cards. A new match replaces the previous browser match; malformed or incompatible saves fail visibly. Clear site data only if you intend to remove local saves.

Visual asset provenance and regeneration instructions: [assets/workshop](assets/workshop/README.md). Architecture: [ADR 0026](docs/adr/0026-browser-solo-workshop.md).
