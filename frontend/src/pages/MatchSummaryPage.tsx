import { Gauge, History, House, LogIn, Play, Sparkles, Trophy } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { ApiRequestError, loadMatchSummary, loadSharedMatchSummary } from "../api";
import type { AccountProps, MatchSummaryLoadState } from "../appTypes";
import { AccountActions, DetailStat, ShellMessage } from "../components/common";
import { formatUnixTime, heroOptionByType, sideLabel } from "../labels";
import { protectedLoginRoute } from "../routes";
import type { MatchRewardSummary, MatchSummaryResponse, MatchUnlockCallout } from "../types";
import { shouldOfferCompletedMatchLogin } from "./privateMatchAccess";

export function MatchSummaryPage({
  matchId,
  seatToken,
  onNavigate,
  currentUser,
  onSignOut,
  allowSignOut,
  loginNextPath,
}: {
  matchId: string;
  seatToken?: string;
} & AccountProps) {
  const [loadState, setLoadState] = useState<MatchSummaryLoadState>({ status: "loading" });

  useEffect(() => {
    setLoadState({ status: "loading" });
    const load = seatToken ? loadSharedMatchSummary(matchId, seatToken) : loadMatchSummary(matchId);
    load
      .then((response) => setLoadState({ status: "ready", response }))
      .catch((error: unknown) =>
        setLoadState({
          status: "error",
          message: error instanceof Error ? error.message : "Could not load match summary",
          httpStatus: error instanceof ApiRequestError ? error.status : undefined,
        }),
      );
  }, [matchId, seatToken]);

  if (loadState.status === "loading") {
    return <ShellMessage title={`Match ${matchId}`} message="Loading match summary" />;
  }

  if (loadState.status === "error") {
    if (shouldOfferCompletedMatchLogin({ currentUser, seatToken, httpStatus: loadState.httpStatus })) {
      const loginPath = protectedLoginRoute(
        loginNextPath ?? `/matches/${encodeURIComponent(matchId)}/summary`,
      );

      return (
        <ShellMessage
          title={`Match ${matchId}`}
          message="Sign in to view this match summary"
          actions={
            <>
              <button className="primary-button" type="button" onClick={() => onNavigate(loginPath)}>
                <LogIn size={18} />
                Sign In
              </button>
              <button className="secondary-link" type="button" onClick={() => onNavigate("/play")}>
                <Play size={18} />
                Play
              </button>
            </>
          }
        />
      );
    }

    return (
      <ShellMessage
        title={`Match ${matchId}`}
        message={loadState.message}
        actions={
          <button className="primary-button" type="button" onClick={() => onNavigate("/play")}>
            <Play size={18} />
            Open Play
          </button>
        }
      />
    );
  }

  const { response } = loadState;
  const replayPath = seatToken
    ? `/match/${encodeURIComponent(matchId)}/${encodeURIComponent(seatToken)}/replay`
    : `/matches/${encodeURIComponent(matchId)}/replay`;

  return (
    <main className="app-shell archive-shell match-summary-shell">
      <section className="match-summary-layout" aria-label="Match summary">
        <header className="top-bar">
          <div>
            <p className="eyebrow">Arcania</p>
            <h1>Match Summary</h1>
          </div>
          <div className="actions">
            <AccountActions
              currentUser={currentUser}
              onNavigate={onNavigate}
              onSignOut={onSignOut}
              allowSignOut={allowSignOut}
              loginNextPath={loginNextPath}
            />
          </div>
        </header>

        <ResultPanel response={response} />
        <RewardPanel reward={response.reward} />

        <section className="match-summary-actions" aria-label="Summary actions">
          <button className="primary-button" type="button" onClick={() => onNavigate(replayPath)}>
            <History size={18} />
            Replay
          </button>
          <button className="secondary-link" type="button" onClick={() => onNavigate("/")}>
            <House size={18} />
            Dashboard
          </button>
          <button className="secondary-link" type="button" onClick={() => onNavigate("/play")}>
            <Play size={18} />
            Play
          </button>
        </section>
      </section>
    </main>
  );
}

function ResultPanel({ response }: { response: MatchSummaryResponse }) {
  const resultLabel = resultTitle(response.viewer.result);
  const winnerLabel = response.summary.winner ? `${sideLabel(response.summary.winner)} wins` : "Match ended";

  return (
    <section className={`match-summary-result ${response.viewer.result}`}>
      <div className="match-summary-result-title">
        <span className="summary-result-icon">
          <Trophy size={24} />
        </span>
        <div>
          <p className="eyebrow">{winnerLabel}</p>
          <h2>{resultLabel}</h2>
        </div>
      </div>
      <div className="summary-stat-grid">
        <DetailStat label="Match" value={response.matchId} />
        <DetailStat label="Mode" value={response.summary.mode === "shared" ? "Shared" : "Solo"} />
        <DetailStat label="Round" value={response.summary.round} />
        <DetailStat label="Frames" value={response.summary.frameCount} />
        <DetailStat label="Updated" value={formatUnixTime(response.summary.updatedAt)} />
      </div>
    </section>
  );
}

function RewardPanel({ reward }: { reward: MatchRewardSummary | null }) {
  if (!reward) {
    return (
      <section className="match-summary-panel">
        <div className="section-heading">
          <Gauge size={19} />
          <h2>Progression Rewards</h2>
        </div>
        <p className="empty-state">No account progression was awarded for this viewer.</p>
      </section>
    );
  }

  const hero = heroOptionByType(reward.heroType);
  const accountLevelDelta = reward.account.after.level - reward.account.before.level;
  const heroLevelDelta = reward.hero.after.level - reward.hero.before.level;

  return (
    <section className="match-summary-panel">
      <div className="section-heading">
        <Gauge size={19} />
        <h2>Progression Rewards</h2>
      </div>
      <div className="reward-grid">
        <RewardDelta
          label="Account XP"
          value={`+${reward.accountXpGained}`}
          before={`Lv. ${reward.account.before.level}`}
          after={`Lv. ${reward.account.after.level}`}
          delta={accountLevelDelta > 0 ? `+${accountLevelDelta} level` : "No level up"}
        />
        <RewardDelta
          label={`${hero.name} Mastery`}
          value={`+${reward.heroXpGained}`}
          before={`Lv. ${reward.hero.before.level}`}
          after={`Lv. ${reward.hero.after.level}`}
          delta={heroLevelDelta > 0 ? `+${heroLevelDelta} level` : "No level up"}
        />
        <RewardDelta
          label="Win Bonus"
          value={`+${reward.winBonusXp}`}
          before={reward.won ? "Won" : "Lost"}
          after={sideLabel(reward.side)}
          delta={reward.won ? "Applied" : "No bonus"}
        />
      </div>
      <UnlockList unlocks={reward.unlocks} />
    </section>
  );
}

function RewardDelta({
  label,
  value,
  before,
  after,
  delta,
}: {
  label: string;
  value: string;
  before: string;
  after: string;
  delta: string;
}) {
  return (
    <article className="reward-delta">
      <span>{label}</span>
      <strong>{value}</strong>
      <small>
        {before} to {after}
      </small>
      <em>{delta}</em>
    </article>
  );
}

function UnlockList({ unlocks }: { unlocks: MatchUnlockCallout[] }) {
  const labels = useMemo(() => unlocks.map(unlockCalloutLabel), [unlocks]);

  return (
    <div className="unlock-list" aria-label="Unlock callouts">
      <div className="section-heading">
        <Sparkles size={18} />
        <h3>Unlocks</h3>
      </div>
      {labels.length === 0 ? (
        <p className="empty-state">No new unlocks.</p>
      ) : (
        <ul>
          {labels.map((label) => (
            <li key={label}>{label}</li>
          ))}
        </ul>
      )}
    </div>
  );
}

function resultTitle(result: MatchSummaryResponse["viewer"]["result"]) {
  switch (result) {
    case "victory":
      return "Victory";
    case "defeat":
      return "Defeat";
    case "spectator":
      return "Match Complete";
  }
}

function unlockCalloutLabel(unlock: MatchUnlockCallout) {
  switch (unlock.type) {
    case "accountLevel":
      return `Account level ${unlock.level}`;
    case "runeUnlocked":
      return `${unlock.name} unlocked`;
    case "runeSlotUnlocked":
      return `${unlock.runeSlots} rune slots`;
    case "heroMasteryLevel":
      return `${heroOptionByType(unlock.heroType).name} mastery level ${unlock.level}`;
    case "skillPointUnlocked":
      return `${unlock.skillPoints} skill point${unlock.skillPoints === 1 ? "" : "s"} earned`;
  }
}
