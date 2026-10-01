import type { Meta, StoryObj } from "@storybook/react-vite";
import { WikiScenePanel } from "./WikiScenePanel";
import { WIKI_SCENES, wikiSceneById } from "./wikiScenes";

const meta = {
  title: "Components/WikiScenePanel",
  component: WikiScenePanel,
  args: {
    scene: WIKI_SCENES[0],
  },
} satisfies Meta<typeof WikiScenePanel>;

export default meta;

type Story = StoryObj<typeof meta>;

export const BoardScene: Story = {};

export const MovementCards: Story = {
  args: { scene: wikiSceneById("action-points-budget") ?? WIKI_SCENES[0] },
};

export const MovementResponses: Story = {
  args: { scene: wikiSceneById("cards-priority-stack") ?? WIKI_SCENES[0] },
};

export const DeckRulesScene: Story = {
  args: {
    scene: WIKI_SCENES.find((scene) => scene.type === "deckRules") ?? WIKI_SCENES[0],
  },
};
