import { ArrowRight, BookOpen, Download, Layers, Play, Settings, Sparkles, Swords, Upload } from "lucide-react";
import { useEffect, useState, type ReactNode } from "react";
import { HeroPicker } from "../components/loadoutControls";
import { HERO_OPTIONS } from "../heroes";
import type { DeckCardCount, HeroType } from "../types";
import { CardEditor } from "./CardEditor";
import { DeckRecipeEditor, deckRecipeTotal, matchingSystemDeckId } from "./DeckRecipeEditor";
import {
  BROWSER_MATCH_ID,
  browserCatalog,
  browserSystemDecks,
  defaultSetup,
  hasBrowserMatch,
  loadSetup,
  saveSetup,
  startBrowserMatch,
  validateSetup,
  type WorkshopSetup,
  type WorkshopSystemDeckRecipe,
} from "./engine";
import { NumberField, RuleEditor } from "./RuleEditor";

export function WorkshopPage({
  tab,
  onNavigate,
}: {
  tab: string | null;
  onNavigate: (path: string) => void;
}) {
  const [setup, setSetup] = useState<WorkshopSetup | null>(null);
  const [catalog, setCatalog] = useState<Awaited<ReturnType<typeof browserCatalog>>["cards"]>([]);
  const [systemDecks, setSystemDecks] = useState<WorkshopSystemDeckRecipe[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [resume, setResume] = useState(false);
  const [dirty, setDirty] = useState(false);
  const activeTab = tab === "cards" || tab === "decks" || tab === "rules" ? tab : "play";

  useEffect(() => {
    let cancelled = false;
    Promise.all([loadSetup(), browserCatalog(), browserSystemDecks()])
      .then(([loaded, loadedCatalog, loadedSystemDecks]) => {
        if (!cancelled) {
          setSetup(loaded);
          setCatalog(loadedCatalog.cards);
          setSystemDecks(loadedSystemDecks);
          setResume(hasBrowserMatch());
        }
      })
      .catch((reason: unknown) => {
        if (!cancelled) {
          setError(String(reason));
        }
      });
    return () => {
      cancelled = true;
    };
  }, []);

  function edit(value: WorkshopSetup) {
    setSetup(value);
    setDirty(true);
    setNotice(null);
  }

  async function run(action: () => Promise<void>) {
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      await action();
      return true;
    } catch (reason) {
      setError(String(reason));
      return false;
    } finally {
      setBusy(false);
    }
  }

  function reset() {
    return run(async () => {
      const defaults = await defaultSetup();
      await saveSetup(defaults);
      setSetup(defaults);
      setDirty(false);
      setNotice("Default workshop preset restored. Your saved match is unchanged.");
    });
  }

  async function save() {
    if (!setup) {
      return;
    }
    setSetup(await saveSetup(setup));
    setDirty(false);
    setNotice("Workshop preset saved in this browser.");
  }

  async function start() {
    if (!setup) {
      return;
    }
    await save();
    await startBrowserMatch(setup);
    onNavigate(`/match/${BROWSER_MATCH_ID}`);
  }

  async function exportPreset() {
    if (!setup) {
      return;
    }
    const validated = await validateSetup(setup);
    const url = URL.createObjectURL(
      new Blob([JSON.stringify(validated, null, 2)], { type: "application/json" }),
    );
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = "rune-lanes-preset.json";
    anchor.click();
    window.setTimeout(() => URL.revokeObjectURL(url), 1000);
  }

  let activePanel: ReactNode = null;
  if (setup && activeTab === "play") {
    activePanel = (
      <section className="workshop-play">
        <div className="workshop-play-copy">
          <p className="eyebrow">
            <Sparkles size={15} /> Solo match
          </p>
          <h2>
            Your next move
            <br />
            starts here.
          </h2>
          <p>Move your Hero, summon Units, and claim Mana sources. Defeat the opposing Hero to win.</p>
          <div className="workshop-fields">
            <DeckSelect
              label="Your deck recipe"
              recipe={setup.playerDeckRecipe}
              systemDecks={systemDecks}
              onChange={(playerDeckRecipe) => edit({ ...setup, playerDeckRecipe })}
            />
            <HeroSelect
              label="Bot Hero"
              value={setup.opponentHero}
              onChange={(opponentHero) => edit({ ...setup, opponentHero })}
            />
            <DeckSelect
              label="Bot deck recipe"
              recipe={setup.opponentDeckRecipe}
              systemDecks={systemDecks}
              onChange={(opponentDeckRecipe) => edit({ ...setup, opponentDeckRecipe })}
            />
          </div>
          <button
            className="text-link workshop-deck-edit-link"
            type="button"
            onClick={() => onNavigate("/workshop?tab=decks")}
          >
            <Swords size={15} />
            Modify deck recipes
          </button>
          <div className="workshop-loadout">
            <span>{setup.ruleset.turn.baseHeroMana} base Mana</span>
            <span>{deckRecipeTotal(setup.playerDeckRecipe)} deck cards</span>
            <span>{setup.cards.length} custom cards</span>
          </div>
          <button className="primary-button workshop-start" type="submit">
            {busy ? "Preparing arena…" : "Play against bot"}
            <ArrowRight size={20} />
          </button>
          {resume ? (
            <button
              className="secondary-link"
              type="button"
              onClick={() => onNavigate(`/match/${BROWSER_MATCH_ID}`)}
            >
              Resume saved match
            </button>
          ) : null}
          <small>
            No account needed. Heroes, deck recipes, rules, cards and your latest match are saved in
            this browser. Starting a match replaces the previous one.
          </small>
          <button className="text-link" type="button" onClick={() => onNavigate("/tutorial")}>
            Learn to play with the tutorial
          </button>
        </div>
        <div className="workshop-hero-picker">
          <HeroPicker
            legend="Your Hero"
            selectedHeroType={setup.playerHero}
            busy={busy}
            onSelect={(playerHero) => edit({ ...setup, playerHero })}
          />
        </div>
      </section>
    );
  } else if (setup && activeTab === "rules") {
    activePanel = <RuleEditor setup={setup} onChange={edit} />;
  } else if (setup && activeTab === "decks") {
    activePanel = (
      <DeckRecipeEditor
        setup={setup}
        catalog={catalog}
        systemDecks={systemDecks}
        onChange={edit}
      />
    );
  } else if (setup) {
    activePanel = (
      <CardEditor
        cards={setup.cards}
        catalog={catalog}
        onChange={(cards) =>
          run(async () => {
            const next = await validateSetup({ ...setup, cards });
            edit(next);
            setNotice("Card changes ready. Save the preset or start a match to keep them.");
          })
        }
      />
    );
  }

  return (
    <main className="app-shell workshop-shell">
      <header className="workshop-nav">
        <button className="workshop-brand" onClick={() => onNavigate("/workshop")}>
          <span aria-hidden="true">⬡</span> Rune Lanes
        </button>
        <div className="actions">
          <button className="secondary-link" onClick={() => onNavigate("/wiki")}>
            <BookOpen size={17} />
            Rules wiki
          </button>
          <button className="secondary-link" onClick={() => onNavigate("/settings")}>
            <Settings size={17} />
            Settings
          </button>
        </div>
      </header>

      <section className="workshop-heading">
        <p className="eyebrow">The arena is yours</p>
        <h1>Play. Create. Experiment.</h1>
        <p>A tactical card game on a shared hex arena. Challenge a bot, shape the rules, and bring your own cards.</p>
      </section>

      <nav className="workshop-tabs" aria-label="Workshop">
        <button
          aria-current={activeTab === "play" ? "page" : undefined}
          onClick={() => onNavigate("/workshop")}
        >
          <Play size={18} />
          Play
        </button>
        <button
          aria-current={activeTab === "decks" ? "page" : undefined}
          onClick={() => onNavigate("/workshop?tab=decks")}
        >
          <Swords size={18} />
          Deck recipes
        </button>
        <button
          aria-current={activeTab === "rules" ? "page" : undefined}
          onClick={() => onNavigate("/workshop?tab=rules")}
        >
          <Settings size={18} />
          Match rules
        </button>
        <button
          aria-current={activeTab === "cards" ? "page" : undefined}
          onClick={() => onNavigate("/workshop?tab=cards")}
        >
          <Layers size={18} />
          Card editor
        </button>
      </nav>

      {error ? (
        <div className="notice" role="alert">
          <p>{error}</p>
          {!setup ? (
            <button className="secondary-link" onClick={reset}>
              Use default rules
            </button>
          ) : null}
        </div>
      ) : null}
      {notice ? (
        <p role="status" className="workshop-status">
          {notice}
        </p>
      ) : null}
      {!setup && !error ? <p role="status">Loading the arena…</p> : null}

      {setup ? (
        <form
          onSubmit={(event) => {
            event.preventDefault();
            void run(activeTab === "play" ? start : save);
          }}
        >
          <fieldset disabled={busy} className="workshop-content">
            {activePanel}
            {activeTab !== "play" ? (
              <div className="workshop-save">
                <span>{dirty ? "Unsaved changes" : "Saved preset"}</span>
                <button className="primary-button" type="submit">
                  Save preset
                </button>
                <button className="secondary-link" type="button" onClick={() => void run(start)}>
                  Save & play against bot
                  <ArrowRight size={18} />
                </button>
              </div>
            ) : null}
            <details className="workshop-tools">
              <summary>Preset tools & repeatable matches</summary>
              <p>
                Export rules, deck recipes and custom cards to share or keep a backup. Importing
                validates the preset before replacing your draft.
              </p>
              <NumberField
                label="Match seed"
                min={0}
                max={4294967295}
                value={setup.seed}
                onChange={(seed) => edit({ ...setup, seed })}
              />
              <div className="actions">
                <button
                  className="secondary-link"
                  type="button"
                  onClick={() => void run(exportPreset)}
                >
                  <Download size={16} />
                  Export preset
                </button>
                <label className="secondary-link workshop-import">
                  <Upload size={16} />
                  Import preset
                  <input
                    type="file"
                    accept="application/json,.json"
                    aria-label="Import preset"
                    onChange={(event) => {
                      const file = event.target.files?.[0];
                      event.target.value = "";
                      if (file) {
                        void run(async () => {
                          if (file.size > 100_000) {
                            throw new Error("Preset files must be smaller than 100 KB.");
                          }
                          edit(await validateSetup(JSON.parse(await file.text())));
                          setNotice("Preset imported. Save or start a match to keep it.");
                        });
                      }
                    }}
                  />
                </label>
                <button className="secondary-link" type="button" onClick={reset}>
                  Restore defaults
                </button>
              </div>
            </details>
          </fieldset>
        </form>
      ) : null}
    </main>
  );
}

function HeroSelect({
  label,
  value,
  onChange,
}: {
  label: string;
  value: HeroType;
  onChange: (value: HeroType) => void;
}) {
  return (
    <label className="workshop-field">
      <span>{label}</span>
      <select
        value={value}
        onChange={(event) => {
          const hero = HERO_OPTIONS.find((entry) => entry.id === event.target.value);
          if (hero) {
            onChange(hero.id);
          }
        }}
      >
        {HERO_OPTIONS.map((hero) => (
          <option key={hero.id} value={hero.id}>
            {hero.name}
          </option>
        ))}
      </select>
    </label>
  );
}

function DeckSelect({
  label,
  recipe,
  systemDecks,
  onChange,
}: {
  label: string;
  recipe: DeckCardCount[];
  systemDecks: WorkshopSystemDeckRecipe[];
  onChange: (recipe: DeckCardCount[]) => void;
}) {
  const selectedId = matchingSystemDeckId(recipe, systemDecks) ?? "custom";

  return (
    <label className="workshop-field">
      <span>{label}</span>
      <select
        value={selectedId}
        onChange={(event) => {
          const selected = systemDecks.find((deck) => deck.id === event.target.value);
          if (selected) {
            onChange(selected.cards.map((card) => ({ ...card })));
          }
        }}
      >
        {selectedId === "custom" ? <option value="custom">Custom recipe</option> : null}
        {systemDecks.map((deck) => (
          <option key={deck.id} value={deck.id}>
            {deck.name}
          </option>
        ))}
      </select>
    </label>
  );
}
