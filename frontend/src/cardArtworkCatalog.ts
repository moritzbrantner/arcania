import type { CardRevisionId } from "./types/cards";

export type CardArtworkIdentity = {
  readonly id: string;
  readonly cardRevision: CardRevisionId;
  readonly artworkAssetId: string | null;
};

export type CardArtworkAsset = {
  readonly id: string;
  readonly path: string;
};

export type CardArtworkCatalogData = {
  readonly identities: readonly CardArtworkIdentity[];
  readonly assets: readonly CardArtworkAsset[];
};

export type CardArtworkResolution =
  | {
      readonly status: "resolved";
      readonly visualIdentityId: string;
      readonly cardRevision: CardRevisionId;
      readonly artPath: string;
    }
  | {
      readonly status: "fallback";
      readonly reason: "missingIdentity" | "revisionMismatch" | "missingAsset" | "noArtwork";
      readonly visualIdentityId: string | null;
      readonly cardRevision: CardRevisionId;
      readonly artPath: null;
    };

export type CardArtworkCatalog = {
  resolve(cardRevision: CardRevisionId, visualIdentityId: string | null): CardArtworkResolution;
  toJSON(): CardArtworkCatalogData;
};

export type CardArtworkCatalogError =
  | {
      readonly code: "invalidInput";
      readonly path: string;
      readonly message: string;
    }
  | {
      readonly code: "unknownRevision";
      readonly visualIdentityId: string;
      readonly cardRevision: CardRevisionId;
    }
  | {
      readonly code: "duplicateIdentity" | "duplicateAsset";
      readonly id: string;
    };

export type CardArtworkCatalogResult =
  | { readonly ok: true; readonly catalog: CardArtworkCatalog }
  | { readonly ok: false; readonly errors: readonly CardArtworkCatalogError[] };

/** Prepare presentation metadata against exact, core-published revision IDs.
 * Missing artwork is allowed and resolves to a fallback; invalid identity data
 * rejects the whole catalog. Prepared data is independent of caller mutations.
 */
export function createCardArtworkCatalog(
  input: unknown,
  knownRevisions: readonly CardRevisionId[],
): CardArtworkCatalogResult {
  const errors: CardArtworkCatalogError[] = [];
  const data = parseCatalog(input, errors);
  if (data === null) {
    return { ok: false, errors };
  }
  const known = new Set(knownRevisions.map(revisionKey));
  const seenIdentities = new Set<string>();
  for (const identity of data.identities) {
    if (seenIdentities.has(identity.id)) {
      errors.push({ code: "duplicateIdentity", id: identity.id });
    }
    seenIdentities.add(identity.id);
    if (!known.has(revisionKey(identity.cardRevision))) {
      errors.push({
        code: "unknownRevision",
        visualIdentityId: identity.id,
        cardRevision: { ...identity.cardRevision },
      });
    }
  }
  const seenAssets = new Set<string>();
  for (const asset of data.assets) {
    if (seenAssets.has(asset.id)) {
      errors.push({ code: "duplicateAsset", id: asset.id });
    }
    seenAssets.add(asset.id);
  }
  if (errors.length > 0) {
    return { ok: false, errors };
  }
  const identities = new Map(data.identities.map((identity) => [identity.id, identity]));
  const assets = new Map(data.assets.map((asset) => [asset.id, asset]));
  return {
    ok: true,
    catalog: {
      resolve(cardRevision, visualIdentityId) {
        const identity = visualIdentityId === null ? undefined : identities.get(visualIdentityId);
        const fallback = (
          reason: Extract<CardArtworkResolution, { status: "fallback" }>["reason"],
        ): CardArtworkResolution => ({
          status: "fallback",
          reason,
          visualIdentityId,
          cardRevision: { ...cardRevision },
          artPath: null,
        });
        if (!identity) {
          return fallback("missingIdentity");
        }
        if (
          identity.cardRevision.cardId !== cardRevision.cardId
          || identity.cardRevision.revision !== cardRevision.revision
        ) {
          return fallback("revisionMismatch");
        }
        if (identity.artworkAssetId === null) {
          return fallback("noArtwork");
        }
        const asset = assets.get(identity.artworkAssetId);
        if (!asset) {
          return fallback("missingAsset");
        }
        return {
          status: "resolved",
          visualIdentityId: identity.id,
          cardRevision: { ...cardRevision },
          artPath: asset.path,
        };
      },
      toJSON() {
        return {
          identities: Array.from(identities.values(), (identity) => ({
            ...identity,
            cardRevision: { ...identity.cardRevision },
          })),
          assets: Array.from(assets.values(), (asset) => ({ ...asset })),
        };
      },
    },
  };
}

function revisionKey(revision: CardRevisionId): string {
  return JSON.stringify([revision.cardId, revision.revision]);
}

function parseCatalog(
  input: unknown,
  errors: CardArtworkCatalogError[],
): CardArtworkCatalogData | null {
  if (
    !isRecord(input)
    || !hasExactKeys(input, ["identities", "assets"])
    || !Array.isArray(input.identities)
    || !Array.isArray(input.assets)
  ) {
    return invalidInput(errors, "catalog", "Expected identities and assets arrays only.");
  }
  const rawIdentities: readonly unknown[] = input.identities;
  const rawAssets: readonly unknown[] = input.assets;
  const identities: CardArtworkIdentity[] = [];
  const assets: CardArtworkAsset[] = [];
  for (const [index, value] of rawIdentities.entries()) {
    const path = `identities[${index}]`;
    if (
      !isRecord(value)
      || !hasExactKeys(value, ["id", "cardRevision", "artworkAssetId"])
      || !isNonBlankString(value.id)
      || (value.artworkAssetId !== null && !isNonBlankString(value.artworkAssetId))
    ) {
      invalidInput(errors, path, "Expected an identity ID, exact Card revision and artwork asset ID or null.");
      continue;
    }
    const reference = value.cardRevision;
    if (
      !isRecord(reference)
      || !hasExactKeys(reference, ["cardId", "revision"])
      || !isNonBlankString(reference.cardId)
      || typeof reference.revision !== "number"
      || !Number.isInteger(reference.revision)
      || reference.revision <= 0
      || reference.revision > 0xffffffff
    ) {
      invalidInput(errors, `${path}.cardRevision`, "Expected a Card ID and positive unsigned 32-bit revision.");
      continue;
    }
    identities.push({
      id: value.id,
      cardRevision: { cardId: reference.cardId, revision: reference.revision },
      artworkAssetId: value.artworkAssetId,
    });
  }
  for (const [index, value] of rawAssets.entries()) {
    if (
      !isRecord(value)
      || !hasExactKeys(value, ["id", "path"])
      || !isNonBlankString(value.id)
      || !isNonBlankString(value.path)
    ) {
      invalidInput(errors, `assets[${index}]`, "Expected a nonblank asset ID and artwork path only.");
      continue;
    }
    assets.push({ id: value.id, path: value.path });
  }
  if (errors.length > 0) {
    return null;
  }
  return { identities, assets };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function hasExactKeys(value: Record<string, unknown>, keys: readonly string[]): boolean {
  const actual = Object.keys(value);
  return actual.length === keys.length && actual.every((key) => keys.includes(key));
}

function isNonBlankString(value: unknown): value is string {
  return typeof value === "string" && value.trim().length > 0;
}

function invalidInput(errors: CardArtworkCatalogError[], path: string, message: string): null {
  errors.push({ code: "invalidInput", path, message });
  return null;
}
