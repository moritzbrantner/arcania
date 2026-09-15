import type { Meta, StoryObj } from "@storybook/react-vite";
import { createBoardProjection } from "./boardProjection";
import { TargetingOverlay } from "./targetingOverlay";
import type { TargetingIndicator } from "./targetingIndicators";

const indicators: TargetingIndicator[] = [
  {
    id: "story-targeting",
    source: { type: "selection" },
    actionType: "attack",
    tone: "attack",
    label: "Attack target",
    sourcePieceId: "player-hero",
    sourceCoord: { q: 0, r: 0 },
    primaryTargetPieceId: "opponent-hero",
    primaryTargetCoord: { q: 1, r: 0 },
    secondaryFootprintCoords: [{ q: 1, r: 1 }],
  },
];

const projection = createBoardProjection([
  { coord: { q: 0, r: 0 }, position: { x: 100, y: 100, visible: true } },
  { coord: { q: 1, r: 0 }, position: { x: 240, y: 140, visible: true } },
  { coord: { q: 1, r: 1 }, position: { x: 260, y: 230, visible: true } },
]);

const meta = {
  title: "Components/TargetingOverlay",
  component: TargetingOverlay,
  args: {
    indicators,
    projection,
  },
  decorators: [
    (Story) => (
      <main className="app-shell centered">
        <div style={{ position: "relative", width: 360, height: 280, border: "1px solid #43524d" }}>
          <Story />
        </div>
      </main>
    ),
  ],
} satisfies Meta<typeof TargetingOverlay>;

export default meta;

type Story = StoryObj<typeof meta>;

export const AttackIndicator: Story = {};
