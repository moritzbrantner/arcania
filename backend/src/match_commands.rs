use std::error::Error;
use std::fmt;

use crate::match_access::{Actor, MatchAccess};
use crate::match_session::{
    MatchActionRequest, MatchError, MatchState, RecordedReplayFrame, Side,
};
use crate::match_store::{
    MatchStoreError, SharedMatchStatus, SqliteMatchStore, StoredMatch, StoredSharedMatch,
};
use crate::rules_kernel::{self, GameCommand, NonGameCommand};

pub struct MatchCommands<'a> {
    store: &'a mut SqliteMatchStore,
}

#[derive(Clone, Debug)]
pub struct AppliedSoloAction {
    pub stored_match: StoredMatch,
    pub replay_frames: Vec<RecordedReplayFrame>,
}

#[derive(Clone, Debug)]
pub struct AppliedSharedAction {
    pub shared_match: StoredSharedMatch,
}

#[derive(Debug)]
pub enum MatchCommandError {
    NotFound,
    Forbidden,
    NotActiveSharedMatch,
    SharedMatchNotStarted,
    OpponentStillConnected,
    ForfeitNotClaimable,
    Rule(MatchError),
    Store(MatchStoreError),
}

impl fmt::Display for MatchCommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => write!(f, "Shared match seat was not found"),
            Self::Forbidden => write!(f, "Match was not found"),
            Self::NotActiveSharedMatch => write!(f, "Shared match is not active"),
            Self::SharedMatchNotStarted => write!(f, "Shared match has not started"),
            Self::OpponentStillConnected => write!(f, "Opponent is still connected"),
            Self::ForfeitNotClaimable => write!(f, "Forfeit is not claimable yet"),
            Self::Rule(error) => write!(f, "{error}"),
            Self::Store(error) => write!(f, "{error}"),
        }
    }
}

impl Error for MatchCommandError {}

impl From<MatchStoreError> for MatchCommandError {
    fn from(error: MatchStoreError) -> Self {
        Self::Store(error)
    }
}

impl From<MatchError> for MatchCommandError {
    fn from(error: MatchError) -> Self {
        Self::Rule(error)
    }
}

fn apply_action_through_rules_kernel(
    state: &mut MatchState,
    side: Side,
    request: MatchActionRequest,
    action_index: u32,
) -> Result<Vec<RecordedReplayFrame>, MatchError> {
    match request {
        MatchActionRequest::AdvanceAi => state.apply_action_recording_for_side(
            side,
            MatchActionRequest::AdvanceAi,
            action_index,
        ),
        player_request => {
            let command = match GameCommand::try_from(player_request) {
                Ok(command) => command,
                Err(NonGameCommand::AdvanceAi) => {
                    unreachable!("AdvanceAi is handled before game-command conversion")
                }
            };
            rules_kernel::actions::apply_recording(state, side, command, action_index)
                .map_err(|violation| violation.into_match_error())
        }
    }
}

impl<'a> MatchCommands<'a> {
    pub fn new(store: &'a mut SqliteMatchStore) -> Self {
        Self { store }
    }

    pub fn apply_solo_action(
        &mut self,
        actor: Actor,
        match_id: &str,
        request: MatchActionRequest,
    ) -> Result<AppliedSoloAction, MatchCommandError> {
        let mut stored_match = self
            .store
            .load_match(match_id)?
            .ok_or(MatchCommandError::NotFound)?;

        if !MatchAccess::new(self.store).can_apply_solo_action(&actor, match_id) {
            return Err(MatchCommandError::Forbidden);
        }

        let action_index = self.store.next_action_index(match_id)?;
        let replay_frames = apply_action_through_rules_kernel(
            &mut stored_match.state,
            Side::Player,
            request.clone(),
            action_index,
        )?;
        self.store.save_action_and_replay_frames(
            &stored_match.id,
            action_index,
            &request,
            &stored_match.state,
            &replay_frames,
        )?;

        Ok(AppliedSoloAction {
            stored_match,
            replay_frames,
        })
    }

    pub fn apply_shared_seat_action(
        &mut self,
        match_id: &str,
        seat_token: &str,
        request: MatchActionRequest,
    ) -> Result<AppliedSharedAction, MatchCommandError> {
        self.store.mark_shared_seat_seen(match_id, seat_token)?;
        let shared = self
            .store
            .load_shared_match_for_seat(match_id, seat_token)?
            .ok_or(MatchCommandError::NotFound)?;
        if shared.status != SharedMatchStatus::Active {
            return Err(MatchCommandError::NotActiveSharedMatch);
        }
        let mut match_state = shared
            .state
            .ok_or(MatchCommandError::SharedMatchNotStarted)?;
        let action_index = self.store.next_action_index(match_id)?;
        let frames = apply_action_through_rules_kernel(
            &mut match_state,
            shared.viewer_seat.side,
            request.clone(),
            action_index,
        )?;
        self.store.save_action_and_replay_frames(
            match_id,
            action_index,
            &request,
            &match_state,
            &frames,
        )?;

        let shared_match = self
            .store
            .load_shared_match_for_seat(match_id, seat_token)?
            .ok_or(MatchCommandError::NotFound)?;
        Ok(AppliedSharedAction { shared_match })
    }

    pub fn claim_shared_forfeit(
        &mut self,
        match_id: &str,
        seat_token: &str,
        now: i64,
    ) -> Result<AppliedSharedAction, MatchCommandError> {
        let shared = self
            .store
            .load_shared_match_for_seat(match_id, seat_token)?
            .ok_or(MatchCommandError::NotFound)?;
        if shared.status != SharedMatchStatus::Active {
            return Err(MatchCommandError::NotActiveSharedMatch);
        }
        let opposing_disconnect_times: Vec<_> = shared
            .seats
            .iter()
            .filter(|seat| seat.side.team() != shared.viewer_seat.side.team())
            .map(|seat| seat.disconnected_at)
            .collect();
        if opposing_disconnect_times.iter().any(Option::is_none) {
            return Err(MatchCommandError::OpponentStillConnected);
        }
        let claimable_at = opposing_disconnect_times
            .into_iter()
            .flatten()
            .map(|disconnected_at| disconnected_at + 120)
            .max()
            .ok_or(MatchCommandError::OpponentStillConnected)?;
        if now < claimable_at {
            return Err(MatchCommandError::ForfeitNotClaimable);
        }

        let mut match_state = shared
            .state
            .ok_or(MatchCommandError::SharedMatchNotStarted)?;
        let action_index = self.store.next_action_index(match_id)?;
        let frames = match_state.forfeit_recording(shared.viewer_seat.side, action_index);
        self.store.save_forfeit_action_and_replay_frames(
            match_id,
            action_index,
            r#"{"type":"claimForfeit"}"#,
            &match_state,
            &frames,
            shared.viewer_seat.side,
        )?;

        let shared_match = self
            .store
            .load_shared_match_for_seat(match_id, seat_token)?
            .ok_or(MatchCommandError::NotFound)?;
        Ok(AppliedSharedAction { shared_match })
    }
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use rusqlite::params;

    use super::*;
    use crate::deck_library::starter_deck_snapshot;
    use crate::match_session::{HeroType, MatchProgressionLoadout, ReplayEvent, Side};
    use crate::match_store::SharedMatchFormat;

    fn test_db_path(name: &str) -> std::path::PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!("rune-lanes-match-commands-{name}-{suffix}.sqlite3"))
    }

    #[test]
    fn anonymous_ownerless_solo_action_persists_state_and_replay_frames() {
        let path = test_db_path("solo-action");
        let mut store = SqliteMatchStore::new(path).expect("store should open");
        let created = store
            .create_match_for_user(HeroType::Pyromancer, None)
            .expect("match should create");

        MatchCommands::new(&mut store)
            .apply_solo_action(
                Actor::Anonymous,
                &created.id,
                MatchActionRequest::StartCardPlay,
            )
            .expect("ownerless phase action should apply");

        let applied = MatchCommands::new(&mut store)
            .apply_solo_action(Actor::Anonymous, &created.id, MatchActionRequest::EndTurn)
            .expect("ownerless solo action should apply");

        assert!(!applied.replay_frames.is_empty());
        assert_eq!(applied.stored_match.state.active_side, Side::Opponent);
        assert_eq!(
            store
                .next_action_index(&created.id)
                .expect("action index should load"),
            2
        );
        let replay = store
            .load_replay(&created.id)
            .expect("replay should load")
            .expect("replay should exist");
        assert!(replay.frames.len() > 1);
    }

    #[test]
    fn account_owned_solo_action_rejects_anonymous_actor() {
        let path = test_db_path("solo-forbidden");
        let mut store = SqliteMatchStore::new(path).expect("store should open");
        let created = store
            .create_match_for_user(HeroType::Pyromancer, Some(42))
            .expect("match should create");

        let error = MatchCommands::new(&mut store)
            .apply_solo_action(Actor::Anonymous, &created.id, MatchActionRequest::EndTurn)
            .expect_err("anonymous actor should not apply account-owned match action");

        assert!(matches!(error, MatchCommandError::Forbidden));
        assert_eq!(
            store
                .next_action_index(&created.id)
                .expect("action index should load"),
            0
        );
    }

    #[test]
    fn invalid_solo_action_does_not_persist_action_or_replay_frames() {
        let path = test_db_path("solo-invalid");
        let mut store = SqliteMatchStore::new(path).expect("store should open");
        let created = store
            .create_match_for_user(HeroType::Pyromancer, None)
            .expect("match should create");

        let error = MatchCommands::new(&mut store)
            .apply_solo_action(
                Actor::Anonymous,
                &created.id,
                MatchActionRequest::MovePiece {
                    piece_id: "missing-piece".to_string(),
                    to: crate::match_session::HexCoord { q: 0, r: 0 },
                },
            )
            .expect_err("invalid action should fail");

        assert!(matches!(error, MatchCommandError::Rule(_)));
        assert_eq!(
            store
                .next_action_index(&created.id)
                .expect("action index should load"),
            0
        );
        let replay = store
            .load_replay(&created.id)
            .expect("replay should load")
            .expect("replay should exist");
        assert_eq!(replay.frames.len(), 1);
    }

    #[test]
    fn shared_forfeit_persists_status_winner_and_replay_frame() {
        let path = test_db_path("shared-forfeit");
        let mut store = SqliteMatchStore::new(path).expect("store should open");
        insert_user(&mut store, 101, "player@example.com");
        insert_user(&mut store, 102, "opponent@example.com");
        let created = store
            .create_shared_match(None, SharedMatchFormat::Duel)
            .expect("shared match should create");
        let starter = starter_deck_snapshot();
        store
            .join_shared_match(
                &created.match_id,
                &created.player_token,
                HeroType::Pyromancer,
                starter.clone(),
                MatchProgressionLoadout::default(),
                Some(101),
            )
            .expect("player should join");
        store
            .join_shared_match(
                &created.match_id,
                &created.opponent_token,
                HeroType::Runekeeper,
                starter,
                MatchProgressionLoadout::default(),
                Some(102),
            )
            .expect("opponent should join");
        store
            .mark_shared_seat_disconnected(&created.match_id, &created.opponent_token)
            .expect("opponent should disconnect");
        store
            .connection_mut()
            .execute(
                "
                UPDATE match_seats
                SET disconnected_at = 100
                WHERE match_id = ?1 AND side = 'opponent'
                ",
                params![created.match_id],
            )
            .expect("disconnect timestamp should update");

        let applied = MatchCommands::new(&mut store)
            .claim_shared_forfeit(&created.match_id, &created.player_token, 220)
            .expect("forfeit should apply");

        assert_eq!(applied.shared_match.status, SharedMatchStatus::Forfeited);
        assert_eq!(
            applied
                .shared_match
                .state
                .as_ref()
                .expect("match state should exist")
                .winner,
            Some(Side::Player)
        );
        let replay = store
            .load_replay(&created.match_id)
            .expect("replay should load")
            .expect("replay should exist");
        assert!(
            replay
                .frames
                .iter()
                .any(|frame| matches!(frame.event, ReplayEvent::MatchEnded { .. }))
        );
        assert_eq!(total_xp(&mut store, 101), 150);
        assert_eq!(total_xp(&mut store, 102), 100);
    }

    fn insert_user(store: &mut SqliteMatchStore, user_id: i64, email: &str) {
        store
            .connection_mut()
            .execute(
                "
                INSERT INTO users (id, email, email_normalized, password_hash, display_name)
                VALUES (?1, ?2, ?2, 'hash', 'Test User')
                ",
                params![user_id, email],
            )
            .expect("user should insert");
    }

    fn total_xp(store: &mut SqliteMatchStore, user_id: i64) -> i64 {
        store
            .connection_mut()
            .query_row(
                "SELECT total_xp FROM users WHERE id = ?1",
                params![user_id],
                |row| row.get(0),
            )
            .expect("total xp should load")
    }
}
