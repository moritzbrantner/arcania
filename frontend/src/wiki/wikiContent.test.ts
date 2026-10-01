import { describe, expect, it } from "vitest";
import { WIKI_TOPICS, wikiTopicById } from "./wikiContent";

describe("wiki content", () => {
  it("has unique topic IDs", () => {
    const ids = WIKI_TOPICS.map((topic) => topic.id);

    expect(new Set(ids).size).toBe(ids.length);
  });

  it("only links to existing related topics", () => {
    const ids = new Set(WIKI_TOPICS.map((topic) => topic.id));

    for (const topic of WIKI_TOPICS) {
      expect(topic.relatedTopicIds.length).toBeGreaterThan(0);
      for (const relatedTopicId of topic.relatedTopicIds) {
        expect(ids.has(relatedTopicId)).toBe(true);
      }
    }
  });

  it("gives every topic enough player-facing reference content", () => {
    for (const topic of WIKI_TOPICS) {
      expect(topic.summary.trim().length).toBeGreaterThan(0);
      expect(topic.keyRules.length).toBeGreaterThanOrEqual(3);
      expect(topic.example.trim().length).toBeGreaterThan(0);
      expect(topic.commonMistakes.length).toBeGreaterThan(0);
      expect(topic.relatedTopicIds.length).toBeGreaterThan(0);
    }
  });

  it("documents current Deck recipe rule values", () => {
    const deckRules = wikiTopicById("deck-rules");

    expect(deckRules).not.toBeNull();
    const content = deckRules?.keyRules.join(" ") ?? "";
    expect(content).toContain("60");
    expect(content).toContain("5");
    expect(content).toContain("4");
    expect(content).toContain("3");
    expect(content).toContain("24");
    expect(content).toContain("12");
  });

  it("teaches both proactive Card windows, independent budgets, and stack resumption", () => {
    const turnFlow = wikiTopicById("turn-flow");
    const actions = wikiTopicById("action-points");
    const cards = wikiTopicById("cards-and-priority");

    expect(turnFlow?.keyRules).toContain(
      "A turn proceeds through Movement Phase, Attack Phase, then final Card Play.",
    );
    expect(cards?.keyRules).toContain(
      "Proactive Card play means the active side plays a Card with an empty stack during Movement or final Card Play, never during Attack.",
    );
    expect(cards?.keyRules).toContain(
      "After a Card and its responses resolve, play resumes in the phase where the stack began, unless the match ends or normal phase completion advances play.",
    );
    expect(actions?.example).toContain("Ember Squire enters with 1/2 Unit action points");
    expect(actions?.keyRules.join(" ")).toContain(
      "a Hero with no action points can still play otherwise-legal Cards using Mana",
    );
  });
});
