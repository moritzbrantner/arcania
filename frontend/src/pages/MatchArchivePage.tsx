import { History, Play, Plus } from "lucide-react";
import { useEffect, useState } from "react";
import { createMatch, loadMatches } from "../api";
import type { AccountProps, MatchArchiveLoadState } from "../appTypes";
import { ShellMessage, TopNav } from "../components/common";
import { formatMatchStatus, formatUnixTime, heroOptionByType } from "../labels";

export function MatchArchivePage({
  onNavigate,
  currentUser,
  onSignOut,
}: {
  onNavigate: (to: string) => void;
} & AccountProps) {
  const [loadState, setLoadState] = useState<MatchArchiveLoadState>({ status: "loading" });
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);

  useEffect(() => {
    loadMatches()
      .then((response) => setLoadState({ status: "ready", matches: response.matches }))
      .catch((error: unknown) =>
        setLoadState({
          status: "error",
          message: error instanceof Error ? error.message : "Could not load match archive",
        }),
      );
  }, []);

  async function handleCreateMatch() {
    setBusy(true);
    setNotice(null);
    try {
      const created = await createMatch();
      onNavigate(`/match/${created.matchId}`);
    } catch (error) {
      setNotice(error instanceof Error ? error.message : "Could not create match");
    } finally {
      setBusy(false);
    }
  }

  if (loadState.status === "loading") {
    return <ShellMessage title="Match Archive" message="Loading matches" />;
  }

  if (loadState.status === "error") {
    return (
      <ShellMessage
        title="Match Archive"
        message={loadState.message}
        actions={
          <button className="primary-button" type="button" onClick={() => onNavigate("/play")}>
            Open play
          </button>
        }
      />
    );
  }

  return (
    <main className="app-shell dashboard-shell">
      <TopNav currentUser={currentUser} onNavigate={onNavigate} onSignOut={onSignOut} activePath="/matches" />
      <section className="archive-layout" aria-label="Match archive">
        <header className="top-bar">
          <div>
            <p className="eyebrow">Arcania</p>
            <h1>Match Archive</h1>
          </div>
          <div className="actions">
            <button
              className="primary-button"
              type="button"
              onClick={() => void handleCreateMatch()}
              disabled={busy}
            >
              <Plus size={18} />
              New Match
            </button>
          </div>
        </header>

        {loadState.matches.length === 0 ? (
          <section className="archive-empty">
            <p>No replayable matches yet.</p>
            <button
              className="primary-button"
              type="button"
              onClick={() => void handleCreateMatch()}
              disabled={busy}
            >
              <Plus size={18} />
              New Match
            </button>
          </section>
        ) : (
          <div className="match-list" role="list" aria-label="Replayable matches">
            {loadState.matches.map((match) => (
              <article className="match-row" role="listitem" key={match.matchId}>
                  <div>
                    <strong>{matchupLabel(match)}</strong>
                    <span>{match.viewerDeckName ? `${match.viewerDeckName} · ` : ""}{formatMatchStatus(match)} · {match.matchId}</span>
                </div>
                <div className="match-row-stat">
                  <span>Round</span>
                  <strong>{match.round}</strong>
                </div>
                <div className="match-row-stat">
                  <span>Frames</span>
                  <strong>{match.frameCount}</strong>
                </div>
                <div className="match-row-date">
                  <span>Updated</span>
                  <strong>{formatUnixTime(match.updatedAt)}</strong>
                </div>
                <div className="match-row-actions">
                  {match.phase !== "matchOver" ? (
                    <button
                      className="icon-button"
                      type="button"
                      onClick={() => onNavigate(`/match/${match.matchId}`)}
                      title="Continue match"
                    >
                      <Play size={18} />
                    </button>
                  ) : null}
                  <button
                    className="primary-button"
                    type="button"
                    onClick={() => onNavigate(`/matches/${match.matchId}/summary`)}
                  >
                    <History size={18} />
                    Summary
                  </button>
                </div>
              </article>
            ))}
          </div>
        )}
        {notice ? <p className="notice">{notice}</p> : null}
      </section>
    </main>
  );
}

function matchupLabel(match: import("../types").MatchSummary) {
  const viewer = (match.viewerHeroTypes ?? []).map((hero) => heroOptionByType(hero).name).join(" + ");
  const opponent = (match.opposingHeroTypes ?? []).map((hero) => heroOptionByType(hero).name).join(" + ");
  return viewer && opponent ? `${viewer} vs ${opponent}` : match.matchId;
}
