# Headless AI Balance Lab

Rune Lanes uses a headless Rust CLI and checked-in policy config for dev-only AI self-play. Keeping simulations inside the authoritative rules engine avoids player/archive persistence and makes AI promotion a small reviewable config diff; player-visible spectator mode, arbitrary card-effect DSLs, external model calls, and automatic rule-balance promotion are intentionally out of scope.

AI selection filters configured rule candidates through the core command query facade for the current phase. The baseline policy repositions before Movement setup Cards and re-evaluates movement after each summon; the aggressive policy retains its configured summon-first preference. Both use the same phase-aware decision for command selection and paced compatibility advancement, preserve historical Card windows, and can choose useful higher-priority Spell responses over a pending stack.
