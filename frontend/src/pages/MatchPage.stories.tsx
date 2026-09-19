import type { Meta, StoryObj } from "@storybook/react-vite";
import { fn } from "storybook/test";
import { storyAccount, storyVisualPreferences, withMockApi } from "../storybook/fixtures";
import { MatchPage } from "./MatchPage";

const meta = {
  title: "Pages/MatchPage",
  component: MatchPage,
  decorators: [withMockApi()],
  args: {
    matchId: "arc-story",
    currentUser: storyAccount,
    onNavigate: fn(),
    onSignOut: fn(),
    allowSignOut: true,
    loginNextPath: "/match/arc-story",
    visualPreferences: storyVisualPreferences,
  },
} satisfies Meta<typeof MatchPage>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Ready: Story = {};
