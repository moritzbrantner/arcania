//! Replay and match summary reads (#109): the one read policy for match replays and summaries.
//!
//! Every reader is an [`Actor`]: an anonymous visitor, a signed-in account, or a private
//! Shared match seat link. This module decides whether the actor may read the match, which side
//! the viewer is on, which replay visibility applies, and keeps active Shared matches hidden.
//! Every refusal is [`ReplayReadError::NotFound`], so a hidden match is indistinguishable from a
//! missing one. Route handlers only translate the result into HTTP responses.

use crate::match_access::Actor;
use crate::match_session::{MatchMode, ReplayVisibility, Side};
use crate::match_store::{
    MatchStoreError, SharedMatchStatus, SqliteMatchStore, StoredMatchSummary, StoredReplay,
};
use crate::progression::{MatchRewardSummary, ProgressionError, ProgressionModule};

#[derive(Debug)]
pub(crate) enum ReplayReadError {
    NotFound,
    Store(MatchStoreError),
    Progression(ProgressionError),
}

impl From<MatchStoreError> for ReplayReadError {
    fn from(error: MatchStoreError) -> Self {
        Self::Store(error)
    }
}

impl From<ProgressionError> for ReplayReadError {
    fn from(error: ProgressionError) -> Self {
        Self::Progression(error)
    }
}

/// A replay the actor may watch, with the visibility its frames are projected at.
#[derive(Debug)]
pub(crate) struct ReplayRead {
    pub(crate) replay: StoredReplay,
    pub(crate) visibility: ReplayVisibility,
}

/// A completed match summary seen from the viewer's side.
#[derive(Debug)]
pub(crate) struct MatchSummaryRead {
    pub(crate) summary: StoredMatchSummary,
    /// `None` when the viewer did not play in the match.
    pub(crate) viewer_side: Option<Side>,
    pub(crate) viewer_deck_name: Option<String>,
    pub(crate) reward: Option<MatchRewardSummary>,
}

pub(crate) struct ReplayReads<'a> {
    store: &'a mut SqliteMatchStore,
}

impl<'a> ReplayReads<'a> {
    pub(crate) fn new(store: &'a mut SqliteMatchStore) -> Self {
        Self { store }
    }

    /// The replay of `match_id`. Seat links only read completed Shared matches; other actors
    /// read matches they own (or unowned ones) and completed Shared matches they played in.
    /// Active Shared matches are never readable as replays.
    pub(crate) fn replay(
        &mut self,
        actor: &Actor,
        match_id: &str,
    ) -> Result<ReplayRead, ReplayReadError> {
        let replay = match actor {
            Actor::Seat {
                match_id: seat_match_id,
                seat_token,
            } => {
                self.completed_seat(seat_match_id, seat_token, match_id)?;
                let replay = self.load_replay(match_id)?;
                require_winner(&replay.summary)?;
                replay
            }
            Actor::Anonymous | Actor::Account { .. } => {
                let replay = self.load_replay(match_id)?;
                if !self.actor_can_read(actor, match_id)? {
                    return Err(ReplayReadError::NotFound);
                }
                if replay.summary.state.mode == MatchMode::Shared {
                    require_winner(&replay.summary)?;
                }
                replay
            }
        };
        let visibility = if replay.summary.state.winner.is_some() {
            ReplayVisibility::Revealed
        } else {
            ReplayVisibility::Public
        };
        Ok(ReplayRead { replay, visibility })
    }

    /// The summary of completed match `match_id` from the actor's side, including the account's
    /// match reward. Shared match summaries are only readable by a participant.
    pub(crate) fn summary(
        &mut self,
        actor: &Actor,
        match_id: &str,
    ) -> Result<MatchSummaryRead, ReplayReadError> {
        let (summary, viewer_side, viewer_deck_name, reward_user_id) = match actor {
            Actor::Seat {
                match_id: seat_match_id,
                seat_token,
            } => {
                let seat = self.completed_seat(seat_match_id, seat_token, match_id)?;
                let replay = self.load_replay(match_id)?;
                require_winner(&replay.summary)?;
                (
                    replay.summary,
                    Some(seat.side),
                    seat.deck_recipe_name,
                    seat.participant_user_id,
                )
            }
            Actor::Anonymous | Actor::Account { .. } => {
                let replay = self.load_replay(match_id)?;
                if !self.actor_can_read(actor, match_id)? {
                    return Err(ReplayReadError::NotFound);
                }
                require_winner(&replay.summary)?;
                let user_id = match actor {
                    Actor::Account { user_id } => Some(*user_id),
                    _ => None,
                };
                match (replay.summary.state.mode, user_id) {
                    (MatchMode::Solo, _) => {
                        let deck_name = replay.summary.player_deck_name.clone();
                        (replay.summary, Some(Side::Player), deck_name, user_id)
                    }
                    (MatchMode::Shared, None) => return Err(ReplayReadError::NotFound),
                    (MatchMode::Shared, Some(user_id)) => {
                        let Some(side) = self
                            .store
                            .completed_shared_participant_side(match_id, user_id)?
                        else {
                            return Err(ReplayReadError::NotFound);
                        };
                        let deck_name = self
                            .store
                            .viewer_context_for_user(match_id, user_id)?
                            .and_then(|(_, deck_name)| deck_name);
                        (replay.summary, Some(side), deck_name, Some(user_id))
                    }
                }
            }
        };
        let reward = match (reward_user_id, viewer_side) {
            (Some(user_id), Some(side)) => ProgressionModule::new(self.store.connection_mut())
                .match_reward_summary(user_id, match_id, side)?,
            _ => None,
        };
        Ok(MatchSummaryRead {
            summary,
            viewer_side,
            viewer_deck_name,
            reward,
        })
    }

    fn load_replay(&self, match_id: &str) -> Result<StoredReplay, ReplayReadError> {
        self.store
            .load_replay(match_id)?
            .ok_or(ReplayReadError::NotFound)
    }

    /// The seat behind a seat link, once its Shared match has finished.
    fn completed_seat(
        &self,
        seat_match_id: &str,
        seat_token: &str,
        match_id: &str,
    ) -> Result<crate::match_store::StoredSharedSeat, ReplayReadError> {
        if seat_match_id != match_id {
            return Err(ReplayReadError::NotFound);
        }
        let shared = self
            .store
            .load_shared_match_for_seat(match_id, seat_token)?
            .ok_or(ReplayReadError::NotFound)?;
        match shared.status {
            SharedMatchStatus::Setup | SharedMatchStatus::Active => Err(ReplayReadError::NotFound),
            SharedMatchStatus::Completed | SharedMatchStatus::Forfeited => Ok(shared.viewer_seat),
        }
    }

    /// Owned matches are private to their account; unowned matches are public. Accounts can
    /// also read completed Shared matches they played in.
    fn actor_can_read(&self, actor: &Actor, match_id: &str) -> Result<bool, ReplayReadError> {
        let owner_user_id = self
            .store
            .match_owner_user_id(match_id)?
            .ok_or(ReplayReadError::NotFound)?;
        Ok(match (actor, owner_user_id) {
            (_, None) => true,
            (Actor::Account { user_id }, Some(owner_user_id)) if *user_id == owner_user_id => true,
            (Actor::Account { user_id }, Some(_)) => self
                .store
                .completed_shared_participant_side(match_id, *user_id)?
                .is_some(),
            _ => false,
        })
    }
}

fn require_winner(summary: &StoredMatchSummary) -> Result<(), ReplayReadError> {
    match summary.state.winner {
        Some(_) => Ok(()),
        None => Err(ReplayReadError::NotFound),
    }
}

#[cfg(test)]
mod tests;
