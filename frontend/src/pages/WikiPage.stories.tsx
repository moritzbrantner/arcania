import type { Meta, StoryObj } from "@storybook/react-vite";
import { fn } from "storybook/test";
import { storyAccount } from "../storybook/fixtures";
import { WikiPage } from "./WikiPage";

const meta = {
  title: "Pages/WikiPage",
  component: WikiPage,
  args: {
    topicSlug: null,
    currentUser: storyAccount,
    onNavigate: fn(),
    onSignOut: fn(),
  },
} satisfies Meta<typeof WikiPage>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Overview: Story = {};

export const Topic: Story = {
  args: {
    topicSlug: "action-points",
  },
};
