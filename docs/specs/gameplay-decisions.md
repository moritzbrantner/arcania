# Gameplay direction and approved rules

Status: **owner-approved design decisions**, consolidated 2026-11-09. This is a
target specification, **not a claim that all decisions are implemented**.
`rune-lanes-core` remains authoritative for currently playable match rules.
Keep historical rulesets and event streams compatible when implementing changes.

## Game identity and match structure

| Area | Approved decision | Implementation state |
| --- | --- | --- |
| Game identity | Card strategy and tactical hex-board positioning have equal importance. Players directly control Heroes and Units. | Existing direction |
| Victory | Defeat the opposing Hero; Mana sources and Buildings are strategic resources, not alternative victory conditions. | Existing |
| Heroes | Heroes move on the board. Their combat/support roles may differ by Hero and deck. Any Hero can use any legal deck recipe; no Hero-specific deck restriction. | Existing direction |
| Turns | Preserve Movement Phase → Attack Phase → final Card Play step. Movement allows proactive Cards; Attack does not. Cards spend Mana, not action points. Existing response-stack priority remains in force. | Existing; [ADR 0027](../adr/0027-movement-card-interleaving.md) |
| Action points | Each Hero and Unit has its own **shared** action-point budget for movement, attacks and applicable activations. No separate attack/movement budget. | Existing |
| Summoning | Summon adjacent to the Hero by default. A Card may explicitly grant additional summon locations through a Building effect. New Units keep half their maximum action points, usable during the current Movement/Attack phases. | Adjacent Hero/half-AP existing; extra Building-supplied locations are an approved extension |
| Counterattacks | Defenders deal automatic counterdamage when the attacker is within the defender's own attack range. | Existing; [ADR 0012](../adr/0012-piece-attack-range-and-range-gated-counters.md) |

## Resources, cards and durability

| Area | Approved decision | Implementation state |
| --- | --- | --- |
| Mana sources | Only an occupying Hero or Unit yields the source's Mana at the start of that side's turn. Leaving the hex loses that benefit; no persistent capture ownership. | Existing; [ADRs 0014](../adr/0014-natural-mana-sources.md) and [0016](../adr/0016-neutral-buildings-and-mana-well-migration.md) |
| Mana refresh | Turn-start Mana comes from the Hero and occupied sources; it replaces, rather than accumulates with, leftovers. Unspent Mana remains available for responses during the opposing turn. | Existing |
| Deck cycling | When a draw pile empties, reshuffle the discard pile; Cards may cycle indefinitely. | Existing |
| Mulligan | Before play begins, a player may select **up to three** opening-hand Cards for replacement **once**. The baseline opening hand has seven Cards; existing loadout modifiers still apply. Both sides get the same opportunity. | **Not implemented** |
| Hand limit | Maximum **10 Cards** after completing a turn. If above 10, the player chooses excess Cards to discard before the turn can end. Draw effects may temporarily exceed 10. | **Not implemented** |
| Unit armor | Surviving Units refresh their Unit armor to maximum at the beginning of their owner's turn. Hero HP does not follow this rule. | Existing |
| Unit count | No fixed per-player Unit cap. Empty board spaces, card availability and Mana constrain summoning. | Existing |

## Positioning and combat

| Area | Approved decision | Implementation state |
| --- | --- | --- |
| Melee advance | After destroying an adjacent enemy Unit, a surviving **melee** attacker automatically advances into its vacated hex if available. Applies equally to melee Heroes and melee Units. **Ranged** attackers never auto-advance, even when attacking adjacent targets. Hybrid identities must state their advance behavior explicitly. | **Change required**: current logic advances any adjacent attacker after a Unit kill |
| Ranged line of sight | Range still bounds attacks, but **explicitly designated blocking obstacles** interrupt ranged line of sight. Ordinary Units never block shots. Buildings/terrain block only if their authoritative definition says so. | **Change required**: current attack legality uses distance only, per [ADR 0012](../adr/0012-piece-attack-range-and-range-gated-counters.md) |

The melee/ranged identity must come from a stable Hero/Unit attack definition,
not from the distance of a particular attack. A ranged attacker standing next
to an enemy is still ranged for melee-advance purposes. Any hybrid override is
explicit and inspectable. Obstacle blocking is likewise a typed rule, not a
visual-only inference from a 2D icon or 3D model.

## Card effects and inspectability

- A special behavior must be visible on the Card, Hero passive, rune, or
  Building that supplies it. Avoid invisible global exceptions.
- The initial reusable bounded status effect is **Rooted**, specified in
  [issue #224](https://github.com/moritzbrantner/arcania/issues/224#issuecomment-6718919306).
  Rooted blocks voluntary movement only; attacks, pushes/pulls and teleportation
  remain legal. Each source Card specifies valid targets (Hero, Unit or both)
  and duration in the affected player's Movement Phases. Application during the
  current affected-player Movement Phase takes effect immediately and counts
  that phase. Reapplication keeps the longer remaining duration, not a sum.
  Implementation remains dependent on [#219](https://github.com/moritzbrantner/arcania/issues/219).

## Delivery boundaries

1. Treat **Existing** entries as confirmation of present behavior, not as a
   request to rewrite it. Treat **Not implemented**, **Change required**, and
   **approved extension** entries as approved future work, not shipped features.
2. Implement rule changes through `rune-lanes-core` commands, queries, typed
   definitions, events and ruleset/version handling. Backend and frontend are
   adapters, not alternative rule engines.
3. Validate both Solo and Shared match legality, stack interactions, event
   replay and 2D/3D player presentation. Show legality/reasons and source
   identity rather than deriving gameplay from UI heuristics.
4. Specify deterministic Mulligan card replacement, reinsertion and RNG order,
   obstacle line-tracing/tie rules, and explicit hybrid attack identities before
   implementing them. Those technical details were **not** settled in this
   design discussion; do not silently invent their gameplay semantics.
5. Update relevant ADRs, Rules wiki, tutorials and automated tests alongside
   actual behavior changes. In particular, line-of-sight implementation must
   explicitly revise or supersede ADR 0012's distance-only decision.
