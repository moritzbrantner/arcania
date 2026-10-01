import type { CardRevisionId, CatalogCard } from "./index";

export type CardDefinition = Pick<CatalogCard, "id" | "name" | "rarity" | "cost" | "text" | "kind"> & {
  taxonomy?: { faction?: string; element?: string; traits?: string[]; families?: string[] };
};

export type CardDefinitionValidationError =
  | { code: "invalidId"; value: string }
  | { code: "blankName" }
  | { code: "invalidTaxonomyTag" | "duplicateTaxonomyTag"; field: string; value: string }
  | { code: "negativeValue" | "nonPositiveValue"; field: string; value: number }
  | { code: "zeroValue" | "emptyStatChange"; field: string };

export type CardDraft = {
  id: number;
  version: number;
  catalogId: string;
  definition: CardDefinition;
  validationErrors: CardDefinitionValidationError[];
  sourceRevision: number | null;
  createdAt: number;
  updatedAt: number;
};

export type CardRevisionTransfer = {
  schemaVersion: 1;
  revision: { id: CardRevisionId; definition: CardDefinition };
};
