# ADR 0025: Hotkeys run on the shared input-bindings runtime

## Status
Accepted

## Context

Rune Lanes had its own browser hotkey runtime: it matched `keydown` events against saved Account preference bindings, ignored editable targets and modifier shortcuts, and called the handler of the matching command. `input-bindings` is the shared foundation for normalized input, binding resolution, repeat policy, text-entry exclusion and runtime lifecycle, so keeping a local key-to-action runtime duplicates that authority.

An earlier attempt (#138) loaded the shared runtime at startup from the `input-bindings` GitHub Pages bundle. That URL is mutable and unpinned, so any Pages deploy would silently change Rune Lanes controls.

## Decision

The frontend dispatches hotkeys through `@moritzbrantner/input-bindings-runtime` and `@moritzbrantner/input-bindings-web` (`frontend/src/hotkeyRuntime.ts`).

- The packages are installed from the `dist/*` branches that `input-bindings` publishes from `main`, pinned by exact commit in `frontend/package.json`. `bun.lock` records their integrity. Nothing is loaded from a URL at runtime. Advancing a pin is an explicit change to `frontend/package.json` and `bun.lock`.
- Rune Lanes still owns its semantic commands (`runeLanes.<commandId>` actions), default bindings, the active surface's handlers, and Account preference storage (ADR 0008). Only commands with a handler on the current surface are registered.
- A handler returns whether it handled the command. The browser default is prevented only for handled commands. That decision is made in the same `keydown` listener that dispatches the command, so it does not depend on the order in which listeners fire.

## Consequences

- There is no second key-to-action runtime or conflict engine in this repository. Behavior changes to key normalization or dispatch belong in `input-bindings`, followed by a pin bump here (DEP-003).
- Shift-modified keys no longer trigger single-key hotkeys, matching the shared modifier semantics.
- Generic settings from the `settings` foundation are not adopted yet: Rune Lanes has no backend consumer for a settings registry, and the frontend preference definitions remain the single source of their defaults.
