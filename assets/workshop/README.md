# Workshop visual assets

`rune-field.asset.json` generates the shared teal/gold backdrop used by the workshop and custom card previews. Existing Hero portraits provide the game-specific identity. Runtime visuals have no rules authority.

Generated with `asset-tooling` at **d0585174de734ba3e0ff407197dcdaa06a252c4d**. Its public CLI generated `rune-field.receipt.json`; a separate `verify` invocation rebuilt the bytes and reported `exact`. The distributed SVG hash is `7f3cc6aaddb98770afaa66b7538da2b8102a19375164661718c3cb16143739f8`.

From an asset-tooling checkout at that revision, run from this repository:

```sh
bun "$ASSET_TOOLING_ROOT/src/entry.ts" generate assets/workshop/rune-field.asset.json --receipt rune-field.receipt.json
bun "$ASSET_TOOLING_ROOT/src/entry.ts" verify assets/workshop/rune-field.asset.json --receipt rune-field.receipt.json
cp assets/workshop/.asset-tooling/rune-field.svg frontend/public/workshop/rune-field.svg
```

The SVG is intentionally committed as a Pages distribution asset; the spec and receipt retain provenance. The `.asset-tooling` cache/output tree is disposable and ignored. No acquired or model-generated third-party payloads are involved.
