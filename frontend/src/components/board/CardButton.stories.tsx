import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import { createCardArtworkCatalog } from "../../cardArtworkCatalog";
import { createMatchVisualCatalog } from "../../matchVisualIdentity";
import { emberSquireCard } from "../board.fixtures";
import { CardButton } from "./Board";

const revision = { cardId: emberSquireCard.templateId, revision: 1 };
const prepared = createCardArtworkCatalog({
  identities: [
    { id: "original", cardRevision: revision, artworkAssetId: "red" },
    { id: "alternate", cardRevision: revision, artworkAssetId: "blue" },
    { id: "missing", cardRevision: revision, artworkAssetId: "unavailable" },
  ],
  assets: [
    { id: "red", path: "/card-art/ember-squire.svg" },
    { id: "blue", path: "/card-art/stoneguard.svg" },
  ],
}, [revision]);
if (!prepared.ok) {
  throw new Error("Card artwork story catalog should prepare.");
}
const visualCatalog = createMatchVisualCatalog([], prepared.catalog);

function ArtworkChoiceStory({ initialArtworkId = "original" }: { initialArtworkId?: string }) {
  const [artworkId, setArtworkId] = useState(initialArtworkId);
  const [selected, setSelected] = useState(false);
  return (
    <section aria-label="Card artwork choices" style={{ padding: 24, maxWidth: 360 }}>
      <label>
        Artwork
        <select value={artworkId} onChange={(event) => setArtworkId(event.currentTarget.value)}>
          <option value="original">Original artwork</option>
          <option value="alternate">Alternate artwork</option>
          <option value="missing">Missing artwork reference</option>
        </select>
      </label>
      <CardButton
        card={emberSquireCard}
        visualIdentity={visualCatalog.card(emberSquireCard, {
          cardRevision: revision,
          visualIdentityId: artworkId,
        })}
        selected={selected}
        disabled={false}
        onClick={() => setSelected((current) => !current)}
      />
    </section>
  );
}

const meta = {
  title: "Board/Card artwork",
  component: ArtworkChoiceStory,
  args: { initialArtworkId: "original" },
} satisfies Meta<typeof ArtworkChoiceStory>;
export default meta;
type Story = StoryObj<typeof meta>;

export const AlternateArtwork: Story = {};
export const MissingArtworkReference: Story = { args: { initialArtworkId: "missing" } };
