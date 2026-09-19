# Hero Domain Migration

Arcania now treats Hero as the canonical player-avatar concept instead of Wizard, and the API, database schema, UI, assets, deck recipes, and progression model use hero language. Durable account, deck, mastery, skill, and rune-loadout data is migrated from the old wizard-named fields and tables to hero-named equivalents, but wizard-era stored matches, shared-match setup rows, actions, and replay frames are discarded because their snapshots encode old field names and `player-wizard` / `opponent-wizard` piece IDs.

This intentionally supersedes the stable-replay expectation from ADR 0003 for the one-time Hero migration: preserving old replays would require a broad compatibility layer around obsolete match snapshots and event payloads, while the player-facing and durable progression data is more valuable to preserve.
