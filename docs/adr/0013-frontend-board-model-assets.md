# Frontend Board Model Assets

Rune Lanes keeps Board model asset metadata in the frontend presentation layer rather than the backend catalog or match state. The 3D board resolves Board model assets and procedural miniatures from Hero or Unit visual identity, so real `.glb` files can be added later without changing rules, replay frames, shared-match payloads, deck recipes, or card APIs. This preserves the existing Board visual mode boundary: 3D rendering is presentation-only and falls back independently from authoritative match data.

The shared Hero picker reuses this manifest, the cached GLB loader, and procedural miniature recipes for its selected Hero stage. Missing or failed GLB assets retain a procedural miniature. The stage follows the existing visual-mode preference and device defaults, renders on demand, and uses a DOM Hero portrait when WebGL is unavailable or renderer startup fails. Hero selection remains in accessible DOM buttons in every presentation mode.
