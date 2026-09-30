import { Sparkles } from "lucide-react";
import { useEffect, useState } from "react";
import { loadProgression, updatePreferredHero } from "../api";
import type { AccountProps, ProgressionLoadState } from "../appTypes";
import { TopNav } from "../components/common";
import { HERO_OPTIONS } from "../heroes";
import { ProgressionPanel } from "../profile";
import type { AccountProfile, HeroType } from "../types";

type HeroesPageProps = AccountProps & {
  currentUser: AccountProfile;
  onProfileUpdated: (profile: AccountProfile) => void;
};

export function HeroesPage({ currentUser, onNavigate, onSignOut, onProfileUpdated }: HeroesPageProps) {
  const [state, setState] = useState<ProgressionLoadState>({ status: "loading" });
  const [selectedHero, setSelectedHero] = useState<HeroType>(currentUser.preferredHeroType);
  const [savingPreferred, setSavingPreferred] = useState(false);
  const [preferredError, setPreferredError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    loadProgression()
      .then((progression) => !cancelled && setState({ status: "ready", progression }))
      .catch((error: unknown) => !cancelled && setState({ status: "error", message: error instanceof Error ? error.message : "Could not load heroes" }));
    return () => { cancelled = true; };
  }, []);

  const selectedOption = HERO_OPTIONS.find((hero) => hero.id === selectedHero);
  const selectedProgression = state.status === "ready" ? state.progression.heroes.find((hero) => hero.heroType === selectedHero) : null;

  async function makePreferred() {
    setSavingPreferred(true);
    setPreferredError(null);
    try {
      onProfileUpdated(await updatePreferredHero(selectedHero));
    } catch (error: unknown) {
      setPreferredError(error instanceof Error ? error.message : "Could not update preferred Hero. Please try again.");
    } finally {
      setSavingPreferred(false);
    }
  }

  return (
    <main className="app-shell dashboard-shell">
      <TopNav currentUser={currentUser} onNavigate={onNavigate} onSignOut={onSignOut} activePath="/heroes" />
      <section className="dashboard-layout heroes-layout" aria-label="Heroes">
        <header className="home-heading">
          <h1>Heroes</h1>
          <p>Choose a Hero, shape their mastery, and configure their default runes.</p>
        </header>
        {state.status === "loading" ? <p className="empty-state">Loading heroes.</p> : null}
        {state.status === "error" ? <p className="notice">{state.message}</p> : null}
        {state.status === "ready" ? (
          <>
            <div className="hero-roster" role="list" aria-label="Hero roster">
              {HERO_OPTIONS.map((hero) => {
                const progression = state.progression.heroes.find((candidate) => candidate.heroType === hero.id);
                return (
                  <button key={hero.id} type="button" className={`hero-roster-card ${hero.id === selectedHero ? "selected" : ""}`} onClick={() => setSelectedHero(hero.id)} aria-pressed={hero.id === selectedHero} role="listitem">
                    <span>{hero.role}</span>
                    <strong>{hero.name}</strong>
                    <small>Mastery {progression?.level ?? 0}</small>
                  </button>
                );
              })}
            </div>
            <section className="heroes-detail" aria-label={`${selectedOption?.name ?? "Selected"} mastery`}>
              <div className="heroes-detail-heading">
                <div><Sparkles size={18} /><span>{selectedOption?.role}</span><h2>{selectedOption?.name}</h2></div>
                <button className="secondary-link" type="button" onClick={() => void makePreferred()} disabled={savingPreferred || selectedHero === currentUser.preferredHeroType}>
                  {selectedHero === currentUser.preferredHeroType ? "Preferred Hero" : "Make preferred"}
                </button>
              </div>
              {preferredError ? <p className="notice" role="alert">{preferredError}</p> : null}
              {selectedProgression ? <ProgressionPanel progression={state.progression} selectedHero={selectedHero} onSelectHero={setSelectedHero} onProgressionChanged={(progression) => setState({ status: "ready", progression })} /> : null}
            </section>
          </>
        ) : null}
      </section>
    </main>
  );
}
