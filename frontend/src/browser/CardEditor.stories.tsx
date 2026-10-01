import { useState } from "react";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { CardEditor } from "./CardEditor";
import type { CustomCard } from "./engine";

const guardian: CustomCard = { id: "custom-guardian", name: "Moonstone Guardian", rarity: "basic", cost: 2, text: "Stand firm at the edge of the arena.", kind: { type: "unit", attack: 2, armor: 4, maxAp: 2 } };
const meta = {
  title: "Workshop/CardEditor",
  component: CardEditor,
  args: { cards: [], catalog: [], onChange: () => {} },
  render: (args) => {
    const [cards, setCards] = useState(args.cards);
    return <main className="workshop-shell"><CardEditor {...args} cards={cards} onChange={setCards} /></main>;
  },
} satisfies Meta<typeof CardEditor>;
export default meta;
type Story = StoryObj<typeof meta>;
export const Empty: Story = {};
export const WithCustomCard: Story = { args: { cards: [guardian] } };
