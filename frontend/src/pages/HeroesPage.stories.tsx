import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";
import { storyAccount, withMockApi } from "../storybook/fixtures";
import { HeroesPage } from "./HeroesPage";

const meta = {
  title: "Pages/HeroesPage",
  component: HeroesPage,
  decorators: [withMockApi()],
  args: {
    currentUser: storyAccount,
    onNavigate: fn(),
    onSignOut: fn(),
    onProfileUpdated: fn(),
  },
} satisfies Meta<typeof HeroesPage>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Ready: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const roster = within(await canvas.findByRole("list", { name: "Hero roster" }));
    await expect(roster.getAllByRole("listitem")).toHaveLength(8);
    await expect(canvas.getByRole("region", { name: "Runekeeper mastery" })).toBeVisible();
    await userEvent.click(roster.getByText("Pyromancer", { exact: true }));
    await expect(canvas.getByRole("region", { name: "Pyromancer mastery" })).toBeVisible();
  },
};
