import { describe, expect, it } from "vitest";

const stories = import.meta.glob("../**/*.stories.tsx");

const requiredStoryFiles = [
  "../Board3D/Board3D.stories.tsx",
  "../HeroPreview3D.stories.tsx",
  "../components/board.stories.tsx",
  "../components/common.stories.tsx",
  "../components/loadoutControls.stories.tsx",
  "../components/match/MatchUxPanels.stories.tsx",
  "../pages/AuthPage.stories.tsx",
  "../pages/CatalogPage.stories.tsx",
  "../pages/CardDraftsPage.stories.tsx",
  "../pages/DashboardPage.stories.tsx",
  "../pages/DecksPage.stories.tsx",
  "../pages/HeroesPage.stories.tsx",
  "../pages/MatchArchivePage.stories.tsx",
  "../pages/MatchPage.stories.tsx",
  "../pages/MatchPicker.stories.tsx",
  "../pages/MatchScenariosPage.stories.tsx",
  "../pages/MatchSummaryPage.stories.tsx",
  "../pages/ProfilePage.stories.tsx",
  "../pages/ProgressionPage.stories.tsx",
  "../pages/PublicDeckPage.stories.tsx",
  "../pages/ReplayPage.stories.tsx",
  "../pages/RouteRedirect.stories.tsx",
  "../pages/SettingsPage.stories.tsx",
  "../pages/SharedMatchPage.stories.tsx",
  "../pages/TutorialPage.stories.tsx",
  "../pages/WikiPage.stories.tsx",
  "../pages/decks/DeckVisualCardGrid.stories.tsx",
  "../pages/match/MatchEndOverlay.stories.tsx",
  "../targetingOverlay.stories.tsx",
  "../tutorial/TutorialOverlay.stories.tsx",
  "../wiki/WikiScenePanel.stories.tsx",
] as const;

const storyCoverageExclusions = [
  ["../components/match/index.ts", "barrel re-export file"],
  ["../Board3D/BoardCameraControls.tsx", "renderless camera controls used only inside Canvas"],
  ["../Board3D/HitTargets.tsx:ProjectedHitTargetSync", "renderless Canvas projection synchronizer"],
  ["../Board3D/ModelLoader.ts", "asset loader helpers, not React UI"],
  ["../pages/match/useMatchBoardController.ts", "hook-only controller"],
  ["../pages/privateMatchAccess.ts", "pure access helper"],
  ["../tutorial/tutorialReducer.ts", "pure reducer state machine"],
  ["../tutorial/tutorialHighlights.ts", "pure highlight helpers"],
  ["../wiki/wikiContent.ts", "content data"],
  ["../wiki/wikiScenes.ts", "fixture/data module for wiki scenes"],
] as const;

describe("Storybook coverage registry", () => {
  it("has required page and component stories", () => {
    for (const path of requiredStoryFiles) {
      expect(stories, path).toHaveProperty(path);
    }
  });

  it("documents intentional visual coverage exclusions", () => {
    expect(storyCoverageExclusions.length).toBeGreaterThan(0);
    for (const [path, reason] of storyCoverageExclusions) {
      expect(path).toMatch(/^\.\.\//);
      expect(reason.length).toBeGreaterThan(8);
    }
  });
});
