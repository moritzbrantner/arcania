import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, within } from "storybook/test";
import { storyAccount, withMockApi } from "../storybook/fixtures";
import { ProgressionPage } from "./ProgressionPage";

const meta = {
  title: "Pages/ProgressionPage",
  component: ProgressionPage,
  decorators: [withMockApi()],
  args: {
    currentUser: storyAccount,
    onNavigate: fn(),
    onSignOut: fn(),
  },
} satisfies Meta<typeof ProgressionPage>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Ready: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const unlocked = within(await canvas.findByRole("region", { name: "Unlocked runes" }));
    const locked = within(canvas.getByRole("region", { name: "Locked runes" }));
    await expect(unlocked.getByText("Ember Rune", { exact: true })).toBeVisible();
    await expect(locked.getByText("Chrono Rune", { exact: true })).toBeVisible();
    await expect(canvas.getByText("Next level in 180 XP")).toBeVisible();
  },
};
