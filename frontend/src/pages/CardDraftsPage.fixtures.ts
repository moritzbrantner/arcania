import type { CardDraft } from "../types/cardWorkshop";

export const storyCardDrafts: CardDraft[] = [
  {
    id: 1, version: 1, catalogId: "custom-ember", sourceRevision: null, createdAt: 1, updatedAt: 1,
    definition: { id: "ember-draft", name: "Ember Draft", rarity: "basic", cost: 1, text: "A saved Unit Draft.", kind: { type: "unit", attack: 1, armor: 2, maxAp: 2 } },
    validationErrors: [],
  },
  {
    id: 2, version: 3, catalogId: "custom-granite", sourceRevision: null, createdAt: 1, updatedAt: 2,
    definition: { id: "granite-draft", name: "Granite Draft", rarity: "rare", cost: 7, text: "A Draft awaiting a correction.", kind: { type: "unit", attack: 3, armor: -1, maxAp: 2 } },
    validationErrors: [{ code: "negativeValue", field: "kind.armor", value: -1 }],
  },
];
