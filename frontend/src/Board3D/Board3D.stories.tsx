import { Canvas } from "@react-three/fiber";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { fn } from "storybook/test";
import { BOARD_PIECE_VISUAL_MANIFEST } from "../board3dModelManifest";
import { createBoardProjection } from "../boardProjection";
import { deriveBoardSurface } from "../boardSurface";
import { createMatchVisualCatalog } from "../matchVisualIdentity";
import { stackTargetingIndicators } from "../targetingIndicators";
import { catalogCards, storyMatch } from "../components/board.fixtures";
import { Board3DRenderer } from "./Board3DRenderer";
import { Board3DHitTarget } from "./HitTargets";
import { PieceMesh } from "./PieceMesh";
import { ProceduralMiniature } from "./ProceduralMiniature";
import { HexTileMesh } from "./TileMesh";
import { TargetingIndicatorLayer } from "./TargetingIndicatorLayer";
import type { Board3DPiece } from "./types";

const match = storyMatch();
const visualCatalog = createMatchVisualCatalog(catalogCards);
const surface = deriveBoardSurface({
  match,
  viewerSide: "player",
  selectedCard: null,
  selectedPiece: null,
  focusedCoord: null,
  hoveredCoord: null,
  readOnly: false,
  disabled: false,
  tutorialHighlights: [],
  animation: null,
  activeStackItemId: null,
});
const pieces: Board3DPiece[] = surface.pieces;
const interaction =
  surface.tiles.find((tile) => tile.coord.q === 0 && tile.coord.r === 0) ?? surface.tiles[0]!;

const meta = {
  title: "Board3D/Components",
  parameters: {
    layout: "fullscreen",
  },
} satisfies Meta;

export default meta;

type Story = StoryObj<typeof meta>;

export const Renderer: Story = {
  render: () => (
    <main className="app-shell">
      <section className="board-3d-shell" style={{ height: 520 }}>
        <Board3DRenderer
          surface={surface}
          visualCatalog={visualCatalog}
          onFatalRenderError={fn()}
        />
      </section>
    </main>
  ),
};

export const HexTile: Story = {
  render: () => (
    <main className="app-shell centered">
      <div style={{ width: 360, height: 280 }}>
        <Canvas camera={{ position: [0, 5, 5], fov: 45 }}>
          <ambientLight intensity={1} />
          <HexTileMesh
            coord={{ q: 0, r: 0 }}
            interaction={interaction}
            readOnly={false}
            disabled={false}
            onClick={fn()}
            onContextMenu={fn()}
          />
        </Canvas>
      </div>
    </main>
  ),
};

export const Piece: Story = {
  render: () => (
    <main className="app-shell centered">
      <div style={{ width: 360, height: 320 }}>
        <Canvas camera={{ position: [0, 5, 5], fov: 45 }}>
          <ambientLight intensity={1} />
          <directionalLight position={[2, 5, 3]} intensity={1.4} />
          <PieceMesh
            piece={pieces[0]}
            visualCatalog={visualCatalog}
            manifest={BOARD_PIECE_VISUAL_MANIFEST}
            interaction={interaction}
            readOnly={false}
            disabled={false}
            onClick={fn()}
            onContextMenu={fn()}
          />
        </Canvas>
      </div>
    </main>
  ),
};

export const Miniature: Story = {
  render: () => (
    <main className="app-shell centered">
      <div style={{ width: 360, height: 320 }}>
        <Canvas camera={{ position: [0, 4, 5], fov: 45 }}>
          <ambientLight intensity={1} />
          <directionalLight position={[2, 5, 3]} intensity={1.4} />
          <ProceduralMiniature
            position={[0, 0, 0]}
            side="player"
            pieceType="hero"
            visualIdentity={visualCatalog.hero(pieces[0] as Extract<Board3DPiece, { pieceType: "hero" }>)}
            recipe={BOARD_PIECE_VISUAL_MANIFEST.heroes.runekeeper.procedural}
            selected
            legalTarget={false}
            coord={{ q: 0, r: 0 }}
            readOnly={false}
            disabled={false}
            onClick={fn()}
            onContextMenu={fn()}
          />
        </Canvas>
      </div>
    </main>
  ),
};

export const HitTarget: Story = {
  render: () => (
    <main className="app-shell centered">
      <div className="board-3d-overlay" style={{ position: "relative", width: 360, height: 260 }}>
        <Board3DHitTarget
          interaction={interaction}
          projectedPosition={{ x: 180, y: 120, visible: true }}
          readOnly={false}
          disabled={false}
          onClick={fn()}
          onDrop={fn()}
          onContextMenu={fn()}
          onHoverChange={fn()}
        />
      </div>
    </main>
  ),
};

export const TargetingLayer: Story = {
  render: () => (
    <main className="app-shell centered">
      <div className="board-3d-overlay" style={{ position: "relative", width: 360, height: 260 }}>
        <TargetingIndicatorLayer
          indicators={stackTargetingIndicators(storyMatch())}
          projection={createBoardProjection([
            { coord: { q: 0, r: 0 }, position: { x: 100, y: 100, visible: true } },
            { coord: { q: 1, r: 1 }, position: { x: 260, y: 150, visible: true } },
          ])}
        />
      </div>
    </main>
  ),
};
