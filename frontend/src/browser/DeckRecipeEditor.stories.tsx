import { useState } from "react";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { catalogCards } from "../components/board.fixtures";
import { DeckRecipeEditor } from "./DeckRecipeEditor";
import type { WorkshopSystemDeckRecipe } from "./engine";

const sample: WorkshopSystemDeckRecipe = {
  id: "starter-sample", name: "Starter sample", heroType: "runekeeper",
  cards: [{ templateId: "ember-squire", count: 4 }, { templateId: "spark-jolt", count: 4 }],
};
const meta = {
  title: "Workshop/DeckRecipeEditor",
  component: DeckRecipeEditor,
  args: {
    playerRecipe: sample.cards, opponentRecipe: sample.cards,
    catalog: catalogCards.filter((card) => ["ember-squire", "spark-jolt"].includes(card.templateId)),
    systemDecks: [sample], onChange: () => {},
  },
  render: (args) => {
    const [recipes, setRecipes] = useState({ player: args.playerRecipe, opponent: args.opponentRecipe });
    return <main className="workshop-shell"><DeckRecipeEditor {...args}
      playerRecipe={recipes.player} opponentRecipe={recipes.opponent}
      onChange={(side, recipe) => setRecipes((current) => ({ ...current, [side]: recipe }))} />
    </main>;
  },
} satisfies Meta<typeof DeckRecipeEditor>;
export default meta;
type Story = StoryObj<typeof meta>;
export const SystemRecipe: Story = {};
export const Empty: Story = { args: { playerRecipe: [] } };
