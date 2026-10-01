import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import type { HeroType } from "../types";
import { fn } from "storybook/test";
import {
  HeroPicker,
  LobbySeatStatus,
  LoadoutCard,
  LoadoutCarousel,
  LoadoutPeek,
  RuneSelector,
} from "./loadoutControls";
import { accountDeckToLoadout, systemDeckToLoadout } from "../deckHelpers";
import { storyDecks, storyProgression, storySystemDecks } from "../storybook/fixtures";

const systemLoadout = systemDeckToLoadout(storySystemDecks[0]);
const accountLoadout = accountDeckToLoadout(storyDecks[0]);

const meta = {
  title: "Components/LoadoutControls",
  parameters: {
    layout: "fullscreen",
  },
} satisfies Meta;

export default meta;

type Story = StoryObj<typeof meta>;

export const LoadoutCardStory: Story = {
  name: "LoadoutCard",
  render: () => (
    <main className="app-shell picker-shell">
      <LoadoutCard loadout={accountLoadout} selected progression={storyProgression} busy={false} onSelect={fn()} />
    </main>
  ),
};

export const LoadoutCarouselStory: Story = {
  name: "LoadoutCarousel",
  render: () => (
    <main className="app-shell picker-shell">
      <LoadoutCarousel
        loadouts={[accountLoadout, systemLoadout]}
        selectedLoadoutId={accountLoadout.id}
        progression={storyProgression}
        busy={false}
        onSelect={fn()}
      />
    </main>
  ),
};

export const LoadoutPeekStory: Story = {
  name: "LoadoutPeek",
  render: () => (
    <main className="app-shell picker-shell">
      <div className="loadout-carousel">
        <div className="carousel-track">
          <LoadoutPeek loadout={systemLoadout} side="previous" />
        </div>
      </div>
    </main>
  ),
};

export const HeroPickerStory: Story = {
  name: "HeroPicker",
  render: () => (
    <main className="app-shell picker-shell">
      <InteractiveHeroPicker />
    </main>
  ),
};

export const RuneSelectorStory: Story = {
  name: "RuneSelector",
  render: () => (
    <main className="app-shell picker-shell">
      <RuneSelector
        progression={storyProgression}
        heroType="runekeeper"
        selectedRuneIds={["ember-rune"]}
        onChange={fn()}
      />
    </main>
  ),
};

export const LobbySeatStatusStory: Story = {
  name: "LobbySeatStatus",
  render: () => (
    <main className="app-shell picker-shell">
      <LobbySeatStatus label="Player" ready heroName="Runekeeper" />
      <LobbySeatStatus label="Opponent" ready={false} heroName={null} />
    </main>
  ),
};

function InteractiveHeroPicker() {
  const [hero, setHero] = useState<HeroType>("runekeeper");
  return <HeroPicker selectedHeroType={hero} busy={false} onSelect={setHero} />;
}
