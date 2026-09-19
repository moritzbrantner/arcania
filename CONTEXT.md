# Arcania

Arcania is a card/board game hybrid about heroes summoning units and casting spells on a shared hex arena.

## Language

**Hero**:
The player's avatar on the board. A hero is both the source of card play and the defeat condition.
_Avoid_: wizard, commander, base

**Unit**:
A card-created board piece with attack, armor, and unit action points.
_Avoid_: creature, minion, troop

**Unit armor**:
A Unit's renewable durability. Damage lowers current Unit armor; at the beginning of its owner's turn, surviving damaged Units refresh to their max armor.
_Avoid_: health, hero armor, shield

**Spell**:
A card that creates an immediate effect from the caster hero instead of creating a unit.
_Avoid_: tactic

**Item card**:
A Card that equips a Carried Item to an eligible allied Carrier.
_Avoid_: equipment spell

**Carried Item**:
An Item attached to a Unit or Hero, contributing passive effects and optional activations.
_Avoid_: inventory card

**Carrier**:
A Unit or Hero that can carry Items.
_Avoid_: holder, bearer

**Item activation**:
Spending Carrier action points to use a carried Item's active effect.
_Avoid_: item trigger, equipment action

**Stat marker**:
A visible persistent counter on a Carrier that modifies combat stats.
_Avoid_: hidden buff, invisible stat mutation

**Card interaction**:
Playing a Card by spending Mana and resolving its card-created effect through the match rules, including target legality, stack entry, and Unit, Spell, or Item effect logic.
_Avoid_: card handler, play-card plumbing

**Mana**:
The resource spent to play cards. A side refreshes Mana from its Hero and occupied Mana sources at the beginning of that side's turn, while unspent Mana remains available for reactions until that side's next turn begins.
_Avoid_: energy

**Movement Phase**:
The first step of an active side's turn, when that side may move Heroes and Units before committing to attacks or card play.
_Avoid_: planning phase

**Attack Phase**:
The turn step after Movement Phase, when the active side may attack with Heroes and Units until choosing to finish attacks or no legal attacks remain.
_Avoid_: combat phase

**Card Play step**:
The post-attack turn step when the active side may play Cards and then end the turn.
_Avoid_: main phase, planning phase

**Mana source**:
A board hex marker that grants Mana to the side occupying it at the beginning of that side's turn.
_Avoid_: mana tile, controlled hex

**Building**:
A neutral, permanent board feature on a hex. A Building may grant an effect to the occupying side, project an aura, or be activated by its occupying Hero or Unit.
_Avoid_: structure, owned building

**Building visual identity**:
The player-facing presentation that makes a Building recognizable across 2D and 3D Board visual modes, including icon, silhouette, colors, tile base, and fallback treatment. Building visual identity is presentation-only and never changes match rules, replay data, card legality, or shared-match protocol semantics.
_Avoid_: building rules, building state, cosmetic data

**Building card**:
A Card that creates a Building on an adjacent empty hex.
_Avoid_: building spell, structure card

**Building activation**:
Spending 1 action point from a Hero or Unit occupying a Building to use that Building's activated effect.
_Avoid_: building trigger, tower action

**Hero shield**:
A persistent damage buffer on a Hero, created by shield-granting buffs and consumed before Hero HP.
_Avoid_: hero armor, temporary health

**Hero action points**:
The Hero's per-turn action budget for Hero movement, Hero attacks, and Hero-based activations.
_Avoid_: hero movement counter

**Unit action points**:
A unit's per-turn action budget for moving and attacking.
_Avoid_: movement counter

**Melee advance**:
The movement of a surviving adjacent attacker into a destroyed Unit's hex after combat resolution.
_Avoid_: charge, push forward

**Hex**:
One tile on the arena, addressed by its position on the hex grid.
_Avoid_: cell, square

**Adjacent**:
The relationship between two neighboring hexes that share an edge.
_Avoid_: bordering tile

**Attack range**:
The maximum hex distance at which a Hero or Unit can attack an enemy piece. Range 1 means adjacent combat.
_Avoid_: reach, weapon range

**Hero passive**:
A built-in Hero identity rule active in a match without spending cards, runes, or skill points.
_Avoid_: base skill, innate perk

**Radius-3 arena**:
The 37-hex board used by the first hex version of Arcania.
_Avoid_: lane board

**Starter deck**:
The system-provided legal deck recipe copied to new accounts and used by anonymous or default play.
_Avoid_: beginner deck

**Deck recipe**:
A saved list of card-template counts used to create a shuffled match deck.
_Avoid_: deck, pile

**Configured deck recipe**:
An account-owned deck recipe with a saved Hero and rune loadout for use as a match loadout.
_Avoid_: deck settings, deck profile

**Deck library**:
The set of named deck recipes owned by an account.
_Avoid_: collection

**Legal deck recipe**:
A deck recipe that satisfies Arcania deck-building rules and can be used to start a match.
_Avoid_: valid deck

**Draft deck recipe**:
A saved deck recipe that does not currently satisfy deck-building rules.
_Avoid_: invalid deck

**System deck recipe**:
A predefined legal recipe shipped by Arcania, used for starter/default play and AI opponent choices.
_Avoid_: AI-only deck

**Match loadout**:
The pre-match choice for one side, combining a Hero, a deck recipe or system deck recipe, and a rune loadout.
_Avoid_: build, preset

**Player dashboard**:
The root account-oriented surface that summarizes player status and routes into play, Heroes, account progression, deck, profile, and match-history workflows.
_Avoid_: match picker, landing page

**Solo match**:
A match where one human controls Player and the backend AI controls Opponent.
_Avoid_: offline match

**Solo AI policy**:
The backend-owned decision policy that chooses the Opponent's next intent during a Solo match.
_Avoid_: bot logic, enemy AI, opponent automation

**Self-play match**:
An in-memory simulation where both sides are controlled by AI policies for balance evaluation.
_Avoid_: persisted AI match, spectator match

**AI policy**:
A named ordered list of engine-owned heuristic rules that chooses legal match actions.
_Avoid_: model, bot script

**AI policy variant**:
A candidate AI policy evaluated against the default policy.
_Avoid_: experiment bot, AI build

**Rule preset**:
A typed experiment configuration that changes match setup or numeric Card-template values without adding new Card effect semantics.
_Avoid_: rules patch, effect script

**Simulation suite**:
A deterministic set of loadouts, seeds, seat directions, rule presets, and policies to evaluate.
_Avoid_: batch run, benchmark

**Promotion gate**:
The required metrics a candidate AI policy must pass before becoming the default Solo AI policy.
_Avoid_: leaderboard, score cutoff

**Simulation report**:
The CLI output containing aggregate metrics, Promotion gate results, promoted policy status, and sampled replay files.
_Avoid_: replay archive, match history

**Shared match**:
A human-vs-human match reached through private Seat links. A Shared match may be a Duel with one Seat per Team or a 2v2 match with two Seats per Team.
_Avoid_: online match, lobby match

**Account**:
A sign-in identity used for profile settings and owned match history.
_Avoid_: seat

**Account preferences**:
Account-synced controls and presentation preferences such as theme, motion, animation speed, board scale, and hotkey bindings. Account preferences affect the signed-in player's client experience only; they do not change Profile identity, Hero progression, match rules, replay data, or private Seat link semantics.
_Avoid_: profile settings, hero settings, match settings

**Profile**:
The player-facing account identity, including display name, public handle, and generated avatar.
_Avoid_: public player page

**Account experience**:
Total progression earned by a signed-in account from completed matches.
_Avoid_: player score, account points

**Account level**:
A derived profile level from account experience, used for broad account progression such as generic rune unlocks.
_Avoid_: rank

**Hero mastery**:
Hero-specific experience earned by completing matches with that hero.
_Avoid_: class level, hero rank

**Skill point**:
A hero-specific point earned from hero mastery levels and spent in that hero's skill tree.
_Avoid_: talent point

**Skill tree**:
A per-hero set of unlockable passive skills.
_Avoid_: talent tree

**Skill**:
A hero-specific passive upgrade unlocked with skill points.
_Avoid_: talent, perk

**Rune**:
A generic pre-match loadout modifier unlocked by account level and equipped before a match.
_Avoid_: skill, card rune

**Rune loadout**:
The selected runes frozen into a match for one side.
_Avoid_: build, rune page

**Generated avatar**:
A profile avatar made from persisted symbol and color choices, not uploaded media.
_Avoid_: avatar upload

**Visual identity**:
The player-facing presentation that makes a Card, Unit, or Hero recognizable across match surfaces, including art, labels, colors, rarity treatment, token portrait, and unknown fallback.
_Avoid_: skin, cosmetic data, asset lookup

**Targeting indicator**:
A presentation-only board visual that shows the source, primary target, and optional affected footprint of a selected or queued attack or Spell.
_Avoid_: threat line, action marker, rules target

**Action preview**:
A presentation-only explanation of the currently selected or focused match action intent.
_Avoid_: rules hint, client rule

**Action availability reason**:
A presentation-only explanation for why a Card or action cannot currently be used.
_Avoid_: validation error, rules hint

**Action tray**:
A contextual presentation surface listing available actions for the selected Card, Hero, Unit, Item, or Building.
_Avoid_: action marker, command palette

**Turn checklist**:
A compact presentation summary of useful actions or resources remaining in the current turn or priority window.
_Avoid_: turn rules, todo list

**Action recap**:
A compact presentation summary of the latest visible accepted action or action sequence.
_Avoid_: replay event, match log

**Board visual mode**:
A player presentation preference that chooses between the complete 2D board and the enhanced 3D board. It is stored on the account profile or locally for anonymous and seat-link play, and never changes match state, replay data, legality, or shared-match protocol semantics.
_Avoid_: board state, match mode, rules mode

**Match chrome**:
Player-facing controls and status overlays around the Board visual mode during match and replay play.
_Avoid_: match rules, board state, HUD state

**Board model asset**:
A presentation-only 3D asset used by Board visual mode to render a Hero or Unit. It is not match state, replay data, card rules, or catalog legality.
_Avoid_: model, skin, piece data

**Board appearance variant**:
A presentation-only Hero Board visual selected by a player and rendered by Board visual mode. It can be unlocked through Hero mastery, but it never changes match rules, replay data, card legality, or frozen loadouts.
_Avoid_: skin, cosmetic data, piece data

**Procedural miniature**:
A code-generated 3D fallback representation for a Hero or Unit when no Board model asset is configured or when that asset cannot load.
_Avoid_: marker, placeholder, token

**Seat**:
One private player slot in a Shared match. A Seat controls one Participant and grants access through one Seat link.
_Avoid_: account, user

**Team**:
One of the two sides competing in a match, Player or Opponent. In 2v2, each Team has two Participants.
_Avoid_: seat, account

**Participant**:
One independent match actor with a Hero, deck, hand, Mana, turn, and owned Units.
_Avoid_: team, account

**Seat link**:
A private URL that grants access to exactly one seat in a shared match.
_Avoid_: public match link

**Match setup**:
The pre-game state where the creator has chosen a hero and the invitee has not joined or has not chosen theirs.
_Avoid_: lobby

**Active side**:
The side whose turn may submit match actions.
_Avoid_: current user

**Knockout**:
The state where a Participant's Hero has been defeated before the match ends. A knocked-out Participant no longer takes turns or controls pieces.
_Avoid_: death, elimination

**Forfeit**:
A match-ending claim available after the opposing seat has been disconnected for at least two minutes.
_Avoid_: surrender

**Basic card**:
A starter deck card with many copies in the deck.
_Avoid_: common card

**Advanced card**:
A starter deck card with fewer copies than a basic card and more copies than a rare card.
_Avoid_: uncommon card

**Rare card**:
A starter deck card with only one copy in the deck.
_Avoid_: legendary card

**Match archive**:
The list of replay-capable matches stored by the backend.
_Avoid_: match picker, database browser

**Match summary**:
The post-match result and reward surface for a completed match, linking the player to replay, the Player dashboard, or the Play section.
_Avoid_: result screen, post-match report

**Replay event**:
A durable record of one meaningful match occurrence, such as a turn start, draw, card play, movement, attack, unit destruction, or match end.
_Avoid_: text log line

**Replay frame**:
The match state captured immediately after a replay event.
_Avoid_: screenshot, animation frame

**Replay visibility**:
The rule that decides whether hidden card information is redacted or revealed in replay responses.
_Avoid_: debug mode

**Match scenario**:
An authored local-development starting match state loaded into the Rust rules engine to exercise a specific interaction or board condition. A match scenario creates a normal playable match instance, but is not a player-facing mode, replay format, deck recipe, or production feature.
_Avoid_: fixture, sandbox match, test deck

**Tutorial mode**:
A player-facing guided learning flow made from scripted tutorial scenes. Tutorial mode teaches mechanics without creating a persisted match, replay, archive entry, or progression.
_Avoid_: onboarding match, practice match, match scenario

**Tutorial scene**:
An authored client-side board, hand, stack, and objective state used by Tutorial mode to demonstrate one or more mechanics.
_Avoid_: fixture, dev scenario, sandbox match

**Tutorial step**:
One teaching beat inside a tutorial scene, consisting of a paused concept introduction, highlighted elements, and an expected interaction that can advance the tutorial.
_Avoid_: tooltip, prompt

**Rules wiki**:
A public player-facing reference surface that explains Arcania match rules and core mechanics through linked topic pages.
_Avoid_: help center, docs site, internal rulebook

**Wiki topic**:
One rules wiki page focused on a single player-facing gameplay concept, such as Mana, Combat, or Buildings.
_Avoid_: article, mechanic page, doc page

**Wiki scene**:
A public client-side interactive demonstration embedded in a Wiki topic to illustrate one player-facing rule concept with authored board or deck recipe states. A Wiki scene does not create a Match, replay, archive entry, or progression.
_Avoid_: match scenario, dev scenario, sandbox match
