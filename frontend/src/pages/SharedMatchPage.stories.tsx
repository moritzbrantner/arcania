import type { Meta, StoryObj } from "@storybook/react-vite";
import { fn } from "storybook/test";
import { storyAccount, storyVisualPreferences, withMockApi } from "../storybook/fixtures";
import { SharedMatchPage } from "./SharedMatchPage";

const meta = {
  title: "Pages/SharedMatchPage",
  component: SharedMatchPage,
  decorators: [withMockApi()],
  args: {
    matchId: "arc-shared",
    seatToken: "player-seat",
    currentUser: storyAccount,
    onNavigate: fn(),
    onSignOut: fn(),
    allowSignOut: true,
    loginNextPath: "/match/arc-shared/player-seat",
    visualPreferences: storyVisualPreferences,
  },
} satisfies Meta<typeof SharedMatchPage>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Lobby: Story = {};
