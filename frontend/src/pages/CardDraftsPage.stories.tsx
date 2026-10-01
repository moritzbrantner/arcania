import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";
import { storyAccount, withMockApi } from "../storybook/fixtures";
import { storyCardDrafts } from "./CardDraftsPage.fixtures";
import { CardDraftsPage } from "./CardDraftsPage";


const meta = {
  title: "Pages/CardDraftsPage", component: CardDraftsPage,
  args: { currentUser: storyAccount, onNavigate: fn(), onSignOut: fn() },
} satisfies Meta<typeof CardDraftsPage>;
export default meta;
type Story = StoryObj<typeof meta>;

export const Ready: Story = {
  decorators: [withMockApi({ "/api/card-drafts": { drafts: storyCardDrafts } })],
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.click(await canvas.findByRole("button", { name: "Select Granite Draft" }));
    const details = within(canvas.getByRole("region", { name: "Draft details" }));
    await expect(details.getByText("7 Mana", { exact: true })).toBeVisible();
    await expect(details.getByRole("alert")).toHaveTextContent("armor cannot be negative (saved value: -1)");
  },
};

export const Empty: Story = {
  decorators: [withMockApi({ "/api/card-drafts": { drafts: [] } })],
};
