# Configured Deck Recipes And Player System Loadouts

Rune Lanes treats the home-screen choice as a match loadout: a Hero, a deck recipe or system deck recipe, and rune IDs. Account deck recipes now store their Hero and rune configuration so they can appear as complete custom loadouts, while system deck recipes can be selected by players as well as AI opponents. Active skills are still derived from current hero mastery and frozen at match creation, rather than being stored on deck recipes.

The configured system recipes, exact recipe lookup, starter counts and Card materialization are owned by `rune-lanes-core::deck_library`. Hosted player loadouts and the AI lab consume that same immutable recipe set; the backend adds account legality and transport projections. Recipe entry order and per-side Card instance IDs are retained because they determine historical seeded openings and command-journal restoration. Editing-input normalization is a separate authoring concern.
