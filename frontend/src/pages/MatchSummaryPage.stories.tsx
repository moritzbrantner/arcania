import type { Meta, StoryObj } from "@storybook/react-vite";
import { fn } from "storybook/test";
import { storyAccount, storySecondSeatSummaryResponse, withMockApi } from "../storybook/fixtures";
import { MatchSummaryPage } from "./MatchSummaryPage";

const meta = {
  title: "Pages/MatchSummaryPage",
  component: MatchSummaryPage,
  decorators: [withMockApi()],
  args: {
    matchId: "rl-story",
    currentUser: storyAccount,
    onNavigate: fn(),
    onSignOut: fn(),
    allowSignOut: true,
    loginNextPath: "/matches/rl-story/summary",
  },
} satisfies Meta<typeof MatchSummaryPage>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Ready: Story = {};

export const WinningSecondSeat: Story = {
  args: { matchId: "rl-shared", seatToken: "player-two-seat" },
  decorators: [withMockApi({
    "/api/shared-matches/rl-shared/seats/player-two-seat/summary": storySecondSeatSummaryResponse,
  })],
};
