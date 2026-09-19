import { History, Layers, Play, Sparkles, Swords } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { loadDecks, loadProfileMatches, loadProgression } from "../api";
import type { AccountProps, DeckLoadState, MatchArchiveLoadState, ProgressionLoadState } from "../appTypes";
import { TopNav } from "../components/common";
import { HeroPreview3D } from "../HeroPreview3D";
import { HERO_OPTIONS } from "../heroes";
import { formatMatchStatus, formatUnixTime, heroOptionByType } from "../labels";
import type { DeckRecipeSummary, MatchSummary, ProgressionResponse } from "../types";

export function DashboardPage({ currentUser, onNavigate, onSignOut }: AccountProps) {
  const [progressionState, setProgressionState] = useState<ProgressionLoadState | null>(currentUser ? { status: "loading" } : null);
  const [matchesState, setMatchesState] = useState<MatchArchiveLoadState | null>(currentUser ? { status: "loading" } : null);
  const [deckState, setDeckState] = useState<DeckLoadState | null>(currentUser ? { status: "loading" } : null);
  useEffect(() => {
    if (!currentUser) return;
    let cancelled = false;
    loadProgression().then((progression) => !cancelled && setProgressionState({ status: "ready", progression })).catch((error: unknown) => !cancelled && setProgressionState({ status: "error", message: error instanceof Error ? error.message : "Could not load progression" }));
    loadProfileMatches().then(({ matches }) => !cancelled && setMatchesState({ status: "ready", matches })).catch((error: unknown) => !cancelled && setMatchesState({ status: "error", message: error instanceof Error ? error.message : "Could not load matches" }));
    loadDecks().then((response) => !cancelled && setDeckState({ status: "ready", response })).catch((error: unknown) => !cancelled && setDeckState({ status: "error", message: error instanceof Error ? error.message : "Could not load decks" }));
    return () => { cancelled = true; };
  }, [currentUser]);

  const progression = progressionState?.status === "ready" ? progressionState.progression : null;
  const decks = deckState?.status === "ready" ? deckState.response.decks : [];
  const featuredDeck = decks.find((deck) => deck.isDefault && deck.legality.legal) ?? decks.find((deck) => deck.legality.legal) ?? null;
  const matches = matchesState?.status === "ready" ? [...matchesState.matches].sort((a, b) => b.updatedAt - a.updatedAt) : [];
  const activeMatches = matches.filter((match) => match.phase !== "matchOver");

  return <main className="app-shell dashboard-shell">
    <TopNav currentUser={currentUser} onNavigate={onNavigate} onSignOut={onSignOut} activePath="/" />
    <section className="dashboard-layout" aria-label="Dashboard">
      {currentUser ? <>
        <header className="home-heading dashboard-heading"><h1>Dashboard</h1><p>Pick up where you left off or prepare your next match.</p></header>
        <CurrentLoadout deck={featuredDeck} progression={progression} activeMatches={activeMatches} deckState={deckState} onNavigate={onNavigate}/>
        <section className="dashboard-overview" aria-label="Account overview">
          <HeroPreview progressionState={progressionState} onNavigate={onNavigate}/>
          <AccountProgression progressionState={progressionState} onNavigate={onNavigate}/>
          <DeckPreview decks={decks} state={deckState} onNavigate={onNavigate}/>
          <MatchPreview matches={matches} state={matchesState} onNavigate={onNavigate}/>
        </section>
      </> : (
        <GuestDashboard onNavigate={onNavigate}/>
      )}
    </section>
  </main>;
}

function CurrentLoadout({ deck, progression, activeMatches, deckState, onNavigate }: { deck: DeckRecipeSummary | null; progression: ProgressionResponse | null; activeMatches: MatchSummary[]; deckState: DeckLoadState | null; onNavigate: (to: string) => void }) {
  if (deckState?.status === "loading") return <section className="dashboard-current-loadout"><p className="empty-state">Loading your current loadout.</p></section>;
  if (deckState?.status === "error") return <section className="dashboard-current-loadout"><div><p className="eyebrow">Current loadout</p><h2>Deck library unavailable</h2><p>{deckState.message}</p></div><button className="secondary-link" type="button" onClick={() => onNavigate("/decks")}>Open Decks</button></section>;
  if (!deck) return <section className="dashboard-current-loadout"><div><p className="eyebrow">Current loadout</p><h2>No legal deck recipe</h2><p>Create a legal configured deck before starting a match.</p></div><button className="primary-button" type="button" onClick={() => onNavigate("/decks")}><Layers size={18}/>Open Decks</button></section>;
  const hero = heroOptionByType(deck.heroType);
  const runes = deck.runeIds.map((id) => progression?.runes.find((rune) => rune.id === id)?.name ?? id);
  const newest = activeMatches[0];
  return <section className="dashboard-current-loadout" aria-label="Current loadout">
    <div className="dashboard-current-copy"><p className="eyebrow">Current loadout</p><h2>{deck.name}</h2><p>{hero.name} · {hero.role} · {deck.legality.totalCards} cards</p><p className="dashboard-muted-line">{runes.length ? runes.join(", ") : "No runes equipped"}</p>{activeMatches.length > 1 ? <button className="text-link" type="button" onClick={() => onNavigate("/matches")}>{activeMatches.length} active matches</button> : null}</div>
    <div className="dashboard-current-model"><HeroPreview3D heroType={deck.heroType} label={hero.name}/></div>
    <div className="dashboard-current-actions"><button className="primary-button" type="button" onClick={() => onNavigate(newest ? `/match/${newest.matchId}` : "/play")}><Play size={18}/>{newest ? "Continue match" : "Start match"}</button><button className="secondary-link" type="button" onClick={() => onNavigate("/decks")}><Layers size={18}/>Manage deck</button></div>
  </section>;
}

function HeroPreview({ progressionState, onNavigate }: { progressionState: ProgressionLoadState | null; onNavigate: (to: string) => void }) {
  const progression = progressionState?.status === "ready" ? progressionState.progression : null;
  return <article className="dashboard-panel dashboard-panel-wide"><PanelHeading icon={<Swords size={18}/>} title="Heroes" action="View all Heroes" onClick={() => onNavigate("/heroes")}/>{progressionState?.status === "error" ? <p className="notice">{progressionState.message}</p> : null}{!progression ? <p className="empty-state">Loading heroes.</p> : <div className="dashboard-hero-preview-list">{HERO_OPTIONS.slice(0, 4).map((hero) => { const value = progression.heroes.find((entry) => entry.heroType === hero.id); return <button key={hero.id} className="dashboard-hero-preview" type="button" onClick={() => onNavigate("/heroes")}><span>{hero.role}</span><strong>{hero.name}</strong><small>Mastery {value?.level ?? 0} · {value?.availableSkillPoints ?? 0} skill points</small></button>; })}</div>}</article>;
}

function AccountProgression({ progressionState, onNavigate }: { progressionState: ProgressionLoadState | null; onNavigate: (to: string) => void }) {
  const progression = progressionState?.status === "ready" ? progressionState.progression : null;
  const nextRune = progression?.runes.filter((rune) => !rune.unlocked).sort((a, b) => a.unlockLevel - b.unlockLevel)[0];
  return <article className="dashboard-panel"><PanelHeading icon={<Sparkles size={18}/>} title="Progression" action="View progression" onClick={() => onNavigate("/progression")}/>{progressionState?.status === "error" ? <p className="notice">{progressionState.message}</p> : null}{progression ? <><div className="dashboard-stat-grid"><Stat label="Level" value={progression.account.level}/><Stat label="Total XP" value={progression.account.totalXp}/><Stat label="Rune slots" value={progression.account.runeSlots}/></div><Progress value={progression.account.xpIntoLevel} max={progression.account.nextLevelXp - progression.account.currentLevelXp} label={`Next level in ${progression.account.xpToNextLevel} XP`}/><p className="dashboard-muted-line">{nextRune ? `Next rune: ${nextRune.name} at level ${nextRune.unlockLevel}` : "All runes unlocked"}</p></> : <p className="empty-state">Loading progression.</p>}</article>;
}

function DeckPreview({ decks, state, onNavigate }: { decks: DeckRecipeSummary[]; state: DeckLoadState | null; onNavigate: (to: string) => void }) { const legal = decks.filter((deck) => deck.legality.legal).length; return <article className="dashboard-panel"><PanelHeading icon={<Layers size={18}/>} title="Decks" action="Manage decks" onClick={() => onNavigate("/decks")}/>{state?.status === "error" ? <p className="notice">{state.message}</p> : null}{state?.status === "ready" ? <><div className="dashboard-stat-grid"><Stat label="Legal" value={legal}/><Stat label="Draft" value={decks.length - legal}/><Stat label="Total" value={decks.length}/></div><p className="dashboard-muted-line">Default: {decks.find((deck) => deck.isDefault)?.name ?? "No default deck"}</p></> : <p className="empty-state">Loading decks.</p>}</article>; }

function MatchPreview({ matches, state, onNavigate }: { matches: MatchSummary[]; state: MatchArchiveLoadState | null; onNavigate: (to: string) => void }) { return <article className="dashboard-panel dashboard-panel-wide"><PanelHeading icon={<History size={18}/>} title="Recent matches" action="View all Matches" onClick={() => onNavigate("/matches")}/>{state?.status === "error" ? <p className="notice">{state.message}</p> : null}{state?.status === "ready" && matches.length === 0 ? <p className="empty-state">No matches yet.</p> : null}<div className="dashboard-match-list">{matches.slice(0, 4).map((match) => <MatchRow key={match.matchId} match={match} onNavigate={onNavigate}/>)}</div>{state?.status === "loading" ? <p className="empty-state">Loading matches.</p> : null}</article>; }
function MatchRow({ match, onNavigate }: { match: MatchSummary; onNavigate: (to: string) => void }) { const yours = (match.viewerHeroTypes ?? []).map((hero) => heroOptionByType(hero).name).join(" + "); const theirs = (match.opposingHeroTypes ?? []).map((hero) => heroOptionByType(hero).name).join(" + "); return <article className="dashboard-match-row"><div><strong>{yours || match.matchId} vs {theirs || "Opponent"}</strong><span>{match.viewerDeckName ? `${match.viewerDeckName} · ` : ""}{formatMatchStatus(match)} · Round {match.round} · {formatUnixTime(match.updatedAt)}</span></div><button className="secondary-link" type="button" onClick={() => onNavigate(match.phase === "matchOver" ? `/matches/${match.matchId}/summary` : `/match/${match.matchId}`)}>{match.phase === "matchOver" ? "Summary" : "Continue"}</button></article>; }

function GuestDashboard({ onNavigate }: { onNavigate: (to: string) => void }) { return <section className="dashboard-current-loadout"><div><p className="eyebrow">Arcania</p><h1>Build your next match</h1><p>Sign in to save deck recipes, follow Hero mastery, and keep your match history.</p></div><div className="dashboard-current-actions"><button className="primary-button" type="button" onClick={() => onNavigate("/login")}>Sign in to play</button><button className="secondary-link" type="button" onClick={() => onNavigate("/wiki")}>Learn the rules</button></div></section>; }
function PanelHeading({ icon, title, action, onClick }: { icon: React.ReactNode; title: string; action: string; onClick: () => void }) { return <div className="dashboard-panel-heading"><span>{icon}</span><h2>{title}</h2><button className="text-link" type="button" onClick={onClick}>{action}</button></div>; }
function Stat({ label, value }: { label: string; value: string | number }) { return <div className="dashboard-stat"><span>{label}</span><strong>{value}</strong></div>; }
function Progress({ value, max, label }: { value: number; max: number; label: string }) { const percent = max > 0 ? Math.min(100, Math.max(0, value / max * 100)) : 0; return <div className="progress-bar" aria-label={label}><div><span style={{ width: `${percent}%` }}/></div><small>{label}</small></div>; }
