import { Play, Trophy } from "lucide-react";

import { sideLabel } from "../../labels";
import type { Side } from "../../types";

export function MatchEndOverlay({
  winner,
  viewerSide,
  onOpenSummary,
  actionLabel = "Match Summary",
}: {
  winner: Side | null;
  viewerSide: Side;
  onOpenSummary: () => void;
  actionLabel?: string;
}) {
  const result = winner === viewerSide ? "Victory" : "Defeat";
  const winnerLabel = winner ? `${sideLabel(winner)} wins` : "Match complete";

  return (
    <section className="match-end-overlay" role="dialog" aria-label="Match complete">
      <div className={`match-end-panel ${winner === viewerSide ? "victory" : "defeat"}`}>
        <span className="match-end-icon">
          <Trophy size={34} />
        </span>
        <p className="eyebrow">{winnerLabel}</p>
        <h2>{result}</h2>
        <button className="primary-button" type="button" onClick={onOpenSummary}>
          <Play size={18} />
          {actionLabel}
        </button>
      </div>
    </section>
  );
}
