# Headless AI Balance Lab

Arcania uses a headless Rust CLI and checked-in policy config for dev-only AI self-play. Keeping simulations inside the authoritative rules engine avoids player/archive persistence and makes AI promotion a small reviewable config diff; player-visible spectator mode, arbitrary card-effect DSLs, external model calls, and automatic rule-balance promotion are intentionally out of scope.
