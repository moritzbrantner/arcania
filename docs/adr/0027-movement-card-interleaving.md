# ADR 0027: Movement interleaves positioning and Card play

## Status

Accepted. Supersedes the turn timing decision in [ADR 0023](0023-turn-phases-and-mana-only-card-play.md). Its historical timing and compatibility record remain readable.

## Context

Restricting proactive Cards to the post-attack Card Play step separates summoning from positioning: a newly summoned Unit cannot use its partial entry action points to move that turn. Hero action points already pay for board actions while Mana pays for Cards. Movement should let players combine those separate budgets before committing to Attack.

## Decision

New matches use the Movement Card window already shipped in issue #122:

- Movement Phase permits piece movement and otherwise-legal proactive Cards with an empty action stack. Cards spend Mana only, including when the Hero has zero action points.
- Summoned Units retain their existing partial entry action points. A Unit summoned during Movement may use them to move before Attack and, if points remain, attack during Attack.
- Starting Attack commits the side to Attack Phase. Proactive Cards are unavailable there; eligible higher-priority responses remain available over a pending stack.
- Finishing attacks, or exhausting legal attacks, reaches the retained final Card Play step. The side may spend remaining Mana on otherwise-legal Cards, then End Turn with an empty stack. Neither transition returns to an earlier phase.
- The response stack remains authoritative. While it is pending, ordinary movement, attacks, proactive Cards, phase changes, and End Turn are blocked. Priority passing resolves from the top; once the stack empties, the underlying phase resumes subject to normal phase completion.

The existing `movement`, `attack`, and `cardPlay` wire values and command shapes stay unchanged. The core command/query facade remains the rules authority. Tutorial and Wiki scenes are authored learning examples, and live clients consume projected command legality.

## Rollout and compatibility

Movement/Card interleaving is immediately available in new Solo, Shared, and browser matches under the existing current ruleset, without a feature rollout switch. This documentation change introduces no additional ruleset version, migration, replay rewrite, or wire change.

The `TurnRules.movement_card_play` flag and event-stream compatibility delivered by #122 remain in place. Older matches with historical pinned rules retain proactive Cards in Card Play only and their historical phase transitions. Their event streams and replays are preserved rather than rewritten to the new timing.

## Consequences

Players may reposition, summon, and reposition the new Unit during one Movement Phase before choosing to commit to Attack. Mana and Hero/Unit action points remain separate budgets. The default Solo AI follows the same legal phase windows, re-evaluating movement after setup Cards; configured policies may choose a different legal order.

README, Rules wiki, Hero descriptions, the domain glossary, authored scenes, and the tutorial must explain these windows and the one-way turn sequence consistently.
