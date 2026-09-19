import type { Meta, StoryObj } from "@storybook/react-vite";
import { fn } from "storybook/test";
import { DetailStat, AccountActions, TopNav, ShellMessage } from "./common";
import { storyAccount } from "../storybook/fixtures";

const meta = {
  title: "Components/Common",
  parameters: {
    layout: "fullscreen",
  },
} satisfies Meta;

export default meta;

type Story = StoryObj<typeof meta>;

export const DetailStatStory: Story = {
  name: "DetailStat",
  render: () => (
    <main className="app-shell centered">
      <div className="detail-stat-row">
        <DetailStat label="Mana" value="4/6" />
        <DetailStat label="Armor" value={3} />
      </div>
    </main>
  ),
};

export const AccountActionsSignedIn: Story = {
  render: () => (
    <main className="app-shell centered">
      <AccountActions currentUser={storyAccount} onNavigate={fn()} onSignOut={fn()} />
    </main>
  ),
};

export const AccountActionsSignedOut: Story = {
  render: () => (
    <main className="app-shell centered">
      <AccountActions currentUser={null} onNavigate={fn()} onSignOut={fn()} />
    </main>
  ),
};

export const TopNavigation: Story = {
  render: () => (
    <main className="app-shell">
      <TopNav currentUser={storyAccount} onNavigate={fn()} onSignOut={fn()} />
    </main>
  ),
};

export const ShellMessageStory: Story = {
  name: "ShellMessage",
  render: () => <ShellMessage title="Arcania" message="Loading board" />,
};
