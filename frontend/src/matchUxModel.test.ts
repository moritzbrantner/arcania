import { describe, expect, it } from "vitest";
import {
  actionPreviewForCard,
  actionRecapFromReplayEvent,
  actionRecapFromSnapshotDiff,
  actionTrayEntriesForSelection,
  cardAvailability,
  turnChecklistForMatch,
} from "./matchUxModel";
import {
  emberFlaskCard,
  emberSquireCard,
  pendingAttackStack,
  sparkJoltCard,
  storyMatch,
  storyCardProjection,
  storyUnit,
  thunderRailCard,
} from "./components/board.fixtures";
import type { Card, MatchState, Side } from "./types";

describe("match UX model", () => {
  it.each(["movement", "cardPlay"] as const)("uses the projected Card command in %s even with spent Hero AP", (phase) => {
    const match = storyMatch({ phase });
    match.player.hero.apRemaining = 0;
    expect(cardAvailability(match, "player", emberSquireCard).playable).toBe(true);
    expect(actionPreviewForCard(match, "player", emberSquireCard).title).toBe("Ember Squire");
    expect(actionTrayEntriesForSelection({ match, viewerSide: "player", selection: { type: "card", card: emberSquireCard } })[0].enabled).toBe(true);
    expect(turnChecklistForMatch(match, "player").find((entry) => entry.id === "playable-cards")?.value).toBe("2");
  });

  it.each<Side>(["player", "opponent", "playerTwo", "opponentTwo"])("uses only the %s viewer's projection", (side) => {
    const match = storyMatch({ hand: [emberSquireCard] });
    const participant = { ...match.player, side, hero: { ...match.player.hero, side } };
    if (side === "opponent") match.opponent = participant;
    if (side === "playerTwo") match.playerTwo = participant;
    if (side === "opponentTwo") match.opponentTwo = participant;
    match.activeSide = side;
    match.commandProjection = { ...storyCardProjection([{ card: emberSquireCard, targets: [{ type: "hex", coord: { q: 0, r: 0 } }] }]), viewerSide: side };
    expect(cardAvailability(match, side, emberSquireCard).playable).toBe(true);
    const other = side === "player" ? "opponent" : "player";
    expect(cardAvailability(match, other, emberSquireCard).playable).toBe(false);
  });

  it("explains current and historical Card windows from the projection", () => {
    const match = storyMatch({ phase: "attack", commandProjection: storyCardProjection([{ card: emberSquireCard, rejection: "wrongPhase" }]) });
    expect(cardAvailability(match, "player", emberSquireCard).primaryReason?.message).toContain("Movement and Card Play");
    match.phase = "movement";
    match.commandProjection = { ...storyCardProjection([{ card: emberSquireCard, rejection: "wrongPhase" }]), proactiveCardPhases: ["cardPlay"] };
    expect(cardAvailability(match, "player", emberSquireCard).primaryReason?.message).toBe("Cards can be played during Card Play, after attacks.");
  });

  it.each(["movement", "attack", "cardPlay"] as const)("retains projected priority responses during %s", (phase) => {
    const match = storyMatch({
      phase, hand: [sparkJoltCard, emberSquireCard],
      actionStack: [pendingAttackStack], prioritySide: "player",
      commandProjection: storyCardProjection([
        { card: sparkJoltCard, targets: [{ type: "piece", pieceId: "opponent-hero" }] },
        { card: emberSquireCard, rejection: "stackPending" },
      ]),
    });
    match.activeSide = "opponent";
    expect(cardAvailability(match, "player", sparkJoltCard).playable).toBe(true);
    expect(cardAvailability(match, "player", emberSquireCard).primaryReason?.code).toBe("notAResponse");
  });

  it("keeps connection, turn, and priority explanations ahead of lower-priority rejections", () => {
    const match = storyMatch({ commandProjection: storyCardProjection([{ card: emberSquireCard, rejection: "notEnoughMana" }]) });
    match.activeSide = "opponent";
    expect(cardAvailability(match, "player", emberSquireCard).primaryReason?.code).toBe("waitingForTurn");
    const options = { connectionReady: false };
    expect(cardAvailability(match, "player", emberSquireCard, options).primaryReason?.code).toBe("connectionBusy");
    expect(actionPreviewForCard(match, "player", emberSquireCard, undefined, options).body).toBe("The live connection is not ready.");
    match.actionStack = [pendingAttackStack];
    match.prioritySide = "opponent";
    expect(cardAvailability(match, "player", emberSquireCard).primaryReason?.code).toBe("waitingForPriority");
    match.commandProjection = undefined;
    expect(cardAvailability(match, "player", emberSquireCard).playable).toBe(false);
  });

  it("explains unavailable cards by priority", () => {
    const match = storyMatch({ hand: [expensiveCard()], phase: "cardPlay", commandProjection: storyCardProjection([{ card: expensiveCard(), rejection: "notEnoughMana" }]) });
    match.player.mana = 1;
    match.player.hero.apRemaining = 0;

    const availability = cardAvailability(match, "player", expensiveCard());

    expect(availability.playable).toBe(false);
    expect(availability.primaryReason?.code).toBe("insufficientMana");
    expect(availability.reasons.map((reason) => reason.code)).not.toContain("pieceApEmpty");
  });

  it("explains wrong-turn and stack response restrictions", () => {
    const wrongTurn = storyMatch({ hand: [emberSquireCard] });
    wrongTurn.activeSide = "opponent";
    wrongTurn.commandProjection = storyCardProjection([{ card: emberSquireCard, rejection: "notActiveSide" }]);

    expect(cardAvailability(wrongTurn, "player", emberSquireCard).primaryReason?.code).toBe(
      "waitingForTurn",
    );

    const pending = storyMatch({ hand: [emberSquireCard], commandProjection: storyCardProjection([{ card: emberSquireCard, rejection: "stackPending" }]), actionStack: [pendingAttackStack], prioritySide: "player" });
    expect(cardAvailability(pending, "player", emberSquireCard).primaryReason?.code).toBe(
      "notAResponse",
    );

    const lowPriority = storyMatch({
      hand: [thunderRailCard],
      commandProjection: storyCardProjection([{ card: thunderRailCard, rejection: "priorityTooLow" }]),
      actionStack: [{ ...pendingAttackStack, priority: 3 }],
      prioritySide: "player",
    });
    expect(cardAvailability(lowPriority, "player", thunderRailCard).primaryReason?.code).toBe(
      "priorityTooLow",
    );
  });

  it("explains cards with no legal targets", () => {
    const match = storyMatch({ hand: [sparkJoltCard], commandProjection: storyCardProjection([{ card: sparkJoltCard, rejection: "invalidTarget" }]), opponentHero: { q: 3, r: -3 }, phase: "cardPlay" });
    match.board.units = [];

    expect(cardAvailability(match, "player", sparkJoltCard).primaryReason?.code).toBe(
      "noLegalTargets",
    );
  });

  it("creates intent previews for card kinds", () => {
    const unitPreview = actionPreviewForCard(storyMatch({ hand: [emberSquireCard], phase: "cardPlay" }), "player", emberSquireCard);
    expect(unitPreview.details).toContain("Adjacent empty hex");
    expect(unitPreview.body).toContain("Summons");

    const spellPreview = actionPreviewForCard(storyMatch({ hand: [sparkJoltCard], phase: "cardPlay" }), "player", sparkJoltCard);
    expect(spellPreview.tone).toBe("attack");
    expect(spellPreview.body).toContain("Deals 1 damage");

    const itemPreview = actionPreviewForCard(
      storyMatch({ hand: [emberFlaskCard], commandProjection: storyCardProjection([{ card: emberFlaskCard, targets: [{ type: "piece", pieceId: "story-player-unit" }] }]), units: [storyUnit({ q: -1, r: 1 })], phase: "cardPlay" }),
      "player",
      emberFlaskCard,
    );
    expect(itemPreview.tone).toBe("support");
    expect(itemPreview.details).toContain("Friendly Unit within range 2");
  });

  it("lists action tray entries for selected pieces with items and buildings", () => {
    const unit = storyUnit({ q: 0, r: 0 }, {
      items: [
        {
          id: "item-1",
          templateId: "ember-flask",
          name: "Ember Flask",
          passive: { type: "statBonus", attack: 1, armor: 0, maxAp: 0 },
          active: { type: "healCarrier", amount: 2 },
          activeUsedThisTurn: false,
        },
      ],
    });
    const match = storyMatch({ units: [unit] });
    match.board.buildings = [
      {
        id: "building-1",
        templateId: "healing-font",
        name: "Healing Font",
        position: unit.position,
        effect: { type: "activatedHeal", range: 2, amount: 2, targets: "unitsAndHeroes" },
        activatedThisTurn: false,
      },
    ];

    const entries = actionTrayEntriesForSelection({
      match,
      viewerSide: "player",
      selection: { type: "piece", piece: { ...unit, pieceType: "unit" } },
    });

    expect(entries.map((entry) => entry.icon)).toEqual(
      expect.arrayContaining(["move", "attack", "info", "item", "building"]),
    );
    expect(entries.find((entry) => entry.icon === "item")?.enabled).toBe(true);
    expect(entries.find((entry) => entry.icon === "building")?.enabled).toBe(true);
  });

  it("creates compact turn checklist counts", () => {
    const match = storyMatch({
      hand: [emberSquireCard, sparkJoltCard],
      units: [storyUnit({ q: -1, r: 1 })],
      phase: "cardPlay",
    });

    const checklist = turnChecklistForMatch(match, "player", true);

    expect(checklist).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ id: "hero-ap", value: "3/3" }),
        expect.objectContaining({ id: "playable-cards", value: "2" }),
        expect.objectContaining({ id: "units-able", value: "1" }),
      ]),
    );
  });

  it("creates recaps from replay events", () => {
    const recap = actionRecapFromReplayEvent(
      {
        type: "pieceAttacked",
        side: "opponent",
        attackerId: "opponent-hero",
        targetId: "player-hero",
        damageToTarget: 2,
        counterDamageToAttacker: 1,
      },
      "player",
    );

    expect(recap).toMatchObject({
      title: "Opponent attacked",
      tone: "attack",
    });
    expect(recap?.details.join(" ")).toContain("counter damage");
  });

  it("creates recaps from simple snapshot diffs", () => {
    const previous = storyMatch({ units: [storyUnit({ q: -1, r: 1 })] });
    const next: MatchState = {
      ...previous,
      board: {
        ...previous.board,
        units: [storyUnit({ q: 0, r: 0 })],
      },
    };

    const recap = actionRecapFromSnapshotDiff(previous, next, "player");

    expect(recap).toMatchObject({
      title: "You moved Rune Runner",
      tone: "neutral",
    });
  });
});

function expensiveCard(): Card {
  return {
    ...emberSquireCard,
    id: "expensive",
    cost: 9,
  };
}
