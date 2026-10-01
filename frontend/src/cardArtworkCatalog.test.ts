import { describe, expect, it } from "vitest";
import { createCardArtworkCatalog } from "./cardArtworkCatalog";

const revision = { cardId: "ember-squire", revision: 1 };
const data = {
  identities: [
    { id: "ember-original", cardRevision: revision, artworkAssetId: "original" },
    { id: "ember-alternate", cardRevision: revision, artworkAssetId: "alternate" },
  ],
  assets: [
    { id: "original", path: "/card-art/ember-squire.svg" },
    { id: "alternate", path: "/card-art/ember-squire-alternate.svg" },
  ],
};

describe("Card artwork catalog", () => {
  it("keeps prepared identities and assets independent of input and exported data mutations", () => {
    const source = structuredClone(data);
    const known = structuredClone([revision]);
    const prepared = createCardArtworkCatalog(source, known);
    if (!prepared.ok) {
      throw new Error("catalog should prepare");
    }
    source.assets[0].path = "/mutated-input.svg";
    source.identities[0].cardRevision.revision = 9;
    source.identities[0].artworkAssetId = "alternate";
    known[0].revision = 10;
    const exported = prepared.catalog.toJSON();
    const exportedAsset = exported.assets.find((asset) => asset.id === "original");
    const exportedIdentity = exported.identities.find((identity) => identity.id === "ember-original");
    if (!exportedAsset || !exportedIdentity) {
      throw new Error("exported identities should exist");
    }
    // A JavaScript consumer can mutate readonly DTOs at runtime.
    Object.assign(exportedAsset, { path: "/mutated-export.svg" });
    Object.assign(exportedIdentity.cardRevision, { revision: 11 });
    const resolved = prepared.catalog.resolve(revision, "ember-original");
    Object.assign(resolved.cardRevision, { revision: 12 });
    expect(prepared.catalog.resolve(revision, "ember-original")).toEqual({
      status: "resolved", visualIdentityId: "ember-original", cardRevision: revision,
      artPath: "/card-art/ember-squire.svg",
    });
    expect(prepared.catalog.toJSON()).toEqual(data);
  });

  it("reports malformed serializable input without throwing or accepting mechanics fields", () => {
    const inputs: unknown[] = [
      null, [], {}, { identities: {}, assets: [] },
      { ...data, definition: { cost: 99 } },
      { ...data, identities: [{ id: "", cardRevision: revision, artworkAssetId: null }] },
      { ...data, identities: [{ id: "art", cardRevision: { ...revision, revision: 0 }, artworkAssetId: null }] },
      { ...data, identities: [{ id: "art", cardRevision: { ...revision, revision: 1.5 }, artworkAssetId: null }] },
      { ...data, identities: [{ id: "art", cardRevision: { ...revision, revision: 4294967296 }, artworkAssetId: null }] },
      { ...data, identities: [{ id: "art", cardRevision: { ...revision, cardId: "" }, artworkAssetId: null }] },
      { ...data, identities: [{ id: "art", cardRevision: revision, artworkAssetId: 12 }] },
      { ...data, identities: [{ id: "art", cardRevision: revision, artworkAssetId: null, cost: 99 }] },
      { identities: [], assets: [{ id: "asset", path: " " }] },
      { identities: [], assets: [{ id: "asset", path: "/art.svg", definition: {} }] },
    ];
    for (const input of inputs) {
      const result = createCardArtworkCatalog(input, [revision]);
      expect(result.ok).toBe(false);
      if (result.ok) {
        throw new Error("malformed catalog should fail");
      }
      expect(result.errors).toEqual(expect.arrayContaining([
        expect.objectContaining({ code: "invalidInput", path: expect.any(String), message: expect.any(String) }),
      ]));
    }
  });

  it("rejects duplicate visual and asset identities without silently overwriting either", () => {
    const result = createCardArtworkCatalog({
      identities: [...data.identities, { id: "ember-original", cardRevision: revision, artworkAssetId: "alternate" }],
      assets: [...data.assets, { id: "original", path: "/different.svg" }],
    }, [revision]);
    expect(result).toEqual({ ok: false, errors: [
      { code: "duplicateIdentity", id: "ember-original" },
      { code: "duplicateAsset", id: "original" },
    ] });
  });

  it("rejects visual identities referring to an unpublished exact revision", () => {
    const unknown = { cardId: "ember-squire", revision: 2 };
    const result = createCardArtworkCatalog({
      assets: data.assets,
      identities: [{ id: "ember-original", artworkAssetId: "original", cardRevision: unknown }],
    }, [revision]);
    expect(result).toEqual({ ok: false, errors: [{
      code: "unknownRevision", visualIdentityId: "ember-original", cardRevision: unknown,
    }] });
  });

  it("falls back explicitly for mismatched revisions and unavailable identities or artwork", () => {
    const prepared = createCardArtworkCatalog({
      ...data,
      identities: [
        ...data.identities,
        { id: "missing-art", cardRevision: revision, artworkAssetId: "unavailable" },
        { id: "no-art", cardRevision: revision, artworkAssetId: null },
      ],
    }, [revision]);
    if (!prepared.ok) {
      throw new Error("catalog should prepare");
    }
    for (const [cardRevision, visualIdentityId, reason] of [
      [{ cardId: "ember-squire", revision: 2 }, "ember-original", "revisionMismatch"],
      [{ cardId: "other-card", revision: 1 }, "ember-original", "revisionMismatch"],
      [revision, "unknown", "missingIdentity"],
      [revision, null, "missingIdentity"],
      [revision, "missing-art", "missingAsset"],
      [revision, "no-art", "noArtwork"],
    ] as const) {
      expect(prepared.catalog.resolve(cardRevision, visualIdentityId)).toEqual({
        status: "fallback", reason, visualIdentityId, cardRevision, artPath: null,
      });
    }
  });

  it("resolves alternate artwork for one exact gameplay revision and round-trips identities", () => {
    const prepared = createCardArtworkCatalog(data, [revision]);
    expect(prepared.ok).toBe(true);
    if (!prepared.ok) {
      throw new Error("catalog should prepare");
    }
    const original = prepared.catalog.resolve(revision, "ember-original");
    const alternate = prepared.catalog.resolve(revision, "ember-alternate");
    expect(original).toEqual({
      status: "resolved", visualIdentityId: "ember-original", cardRevision: revision,
      artPath: "/card-art/ember-squire.svg",
    });
    expect(alternate).toEqual({
      status: "resolved", visualIdentityId: "ember-alternate", cardRevision: revision,
      artPath: "/card-art/ember-squire-alternate.svg",
    });
    expect(prepared.catalog.toJSON()).toEqual(data);
    const restored = createCardArtworkCatalog(JSON.parse(JSON.stringify(prepared.catalog)), [revision]);
    expect(restored.ok).toBe(true);
    if (!restored.ok) {
      throw new Error("serialized catalog should prepare");
    }
    expect(restored.catalog.resolve(revision, "ember-alternate")).toEqual(alternate);
  });
});
