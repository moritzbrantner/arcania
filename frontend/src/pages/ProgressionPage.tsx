import { Sparkles } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { loadProgression } from "../api";
import type { AccountProps, ProgressionLoadState } from "../appTypes";
import { TopNav } from "../components/common";
import type { RuneDefinition } from "../types";

export function ProgressionPage({ currentUser, onNavigate, onSignOut }: AccountProps) {
  const [state, setState] = useState<ProgressionLoadState>({ status: "loading" });
  useEffect(() => {
    let cancelled = false;
    loadProgression().then((progression) => !cancelled && setState({ status: "ready", progression })).catch((error: unknown) => !cancelled && setState({ status: "error", message: error instanceof Error ? error.message : "Could not load progression" }));
    return () => { cancelled = true; };
  }, []);
  const nextRune = useMemo(() => state.status === "ready" ? state.progression.runes.filter((rune) => !rune.unlocked).sort((a, b) => a.unlockLevel - b.unlockLevel)[0] : null, [state]);
  return <main className="app-shell dashboard-shell">
    <TopNav currentUser={currentUser} onNavigate={onNavigate} onSignOut={onSignOut} activePath="/progression" />
    <section className="dashboard-layout progression-layout" aria-label="Account progression">
      <header className="home-heading"><h1>Progression</h1><p>Track account experience and unlocked runes.</p></header>
      {state.status === "loading" ? <p className="empty-state">Loading progression.</p> : null}
      {state.status === "error" ? <p className="notice">{state.message}</p> : null}
      {state.status === "ready" ? <>
        <section className="dashboard-panel progression-overview"><div className="dashboard-panel-heading"><Sparkles size={18}/><h2>Account level</h2></div><div className="dashboard-stat-grid"><Stat label="Level" value={state.progression.account.level}/><Stat label="Total XP" value={state.progression.account.totalXp}/><Stat label="Rune slots" value={state.progression.account.runeSlots}/></div><Progress value={state.progression.account.xpIntoLevel} max={state.progression.account.nextLevelXp - state.progression.account.currentLevelXp} label={`Next level in ${state.progression.account.xpToNextLevel} XP`}/>{nextRune ? <p className="dashboard-muted-line">Next rune: {nextRune.name} at level {nextRune.unlockLevel}</p> : state.progression.runes.length > 0 ? <p className="dashboard-muted-line">All runes unlocked.</p> : null}</section>
        <section className="dashboard-panel"><div className="dashboard-panel-heading"><Sparkles size={18}/><h2>Rune unlocks</h2></div>{state.progression.runes.length > 0 ? <>
          <RuneGroup title="Unlocked runes" runes={state.progression.runes.filter((rune) => rune.unlocked)} emptyMessage="No runes unlocked yet." />
          <RuneGroup title="Locked runes" runes={state.progression.runes.filter((rune) => !rune.unlocked)} emptyMessage="No locked runes." />
        </> : <p className="empty-state">No rune unlocks available.</p>}</section>
      </> : null}
    </section>
  </main>;
}

function RuneGroup({ title, runes, emptyMessage }: { title: string; runes: RuneDefinition[]; emptyMessage: string }) {
  return <section aria-label={title}><h3>{title}</h3>{runes.length > 0 ? <div className="rune-grid">{runes.map((rune) => <article key={rune.id} className={rune.unlocked ? "selected" : ""}><strong>{rune.name}</strong><span>{rune.unlocked ? rune.text : `Unlocks at level ${rune.unlockLevel}`}</span></article>)}</div> : <p className="empty-state">{emptyMessage}</p>}</section>;
}

function Stat({ label, value }: { label: string; value: string | number }) { return <div className="dashboard-stat"><span>{label}</span><strong>{value}</strong></div>; }
function Progress({ value, max, label }: { value: number; max: number; label: string }) { const percent = max > 0 ? Math.min(100, Math.max(0, value / max * 100)) : 0; return <div className="progress-bar"><div><span style={{ width: `${percent}%` }}/></div><small>{label}</small></div>; }
