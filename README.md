# Arcania

Arcania is a small vertical slice for a card game / board game hybrid.

Players control heroes on a radius-3 hex arena. Cards cost mana and hero action points to summon units or cast spells; units then spend their own action points to move across adjacent hexes and attack enemies within their attack range.

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
- `/match/<match-id>` opens the public playable Arcania board for a persisted
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
such as `arc-lx5n2w`, and the full match snapshot is stored in SQLite after
creation and after each successful action.

SQLite data defaults to `data/arcania.sqlite3`, which is ignored by git. Set
`ARCANIA_DB_PATH=/path/to/arcania.sqlite3` to use a different database,
including isolated temporary databases for tests or local experiments.

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

```sh
bun run ci:local
```

`bun install` configures Git to run `.githooks/pre-push`, which executes the
same local CI script before a push reaches GitHub. If GitHub Actions cannot run
because of a billing or spending limit, a passing `bun run ci:local` is the
project's local signal that the shared CI workflow would have passed.
