//! Shared match lifecycle (#108): the one server-owned module for Shared matches.
//!
//! It owns setup, joining a seat, the active transition (through `join`), private seat
//! capability, presence, heartbeat, disconnect, the forfeit claim, side-specific snapshots and
//! the decision when other seats are notified. HTTP and WebSocket handlers in `routes` are thin
//! adapters: they parse requests, call this module and send what it returns.
//!
//! A seat token is a capability for exactly one side: every read and write here is scoped to the
//! seat it was opened with. Hosted-session leases come from the shared `game-server` foundation
//! through `AppState`; Rune Lanes rules stay in `rune_lanes_core` via `MatchCommands`.

use std::time::Duration;

use game_server::{SessionError, SessionLease};

use crate::app_state::SharedState;
use crate::http_types::{
    DeckChoiceRequest, SharedClientMessage, SharedMatchResponse, SharedServerMessage,
};
use crate::loadout_resolution::{
    LoadoutResolutionError, SharedSeatLoadoutRequest, resolve_shared_seat_loadout,
};
use crate::match_commands::MatchCommands;
use crate::match_session::{HeroType, MatchActionRequest};
use crate::match_store::{
    CreatedSharedMatch, MatchStoreError, SharedMatchFormat, StoredSharedMatch,
};

/// A seat connection closes when its client has been silent this long.
pub(crate) const SHARED_SEAT_SILENCE_LIMIT: Duration = Duration::from_secs(45);
/// How often an open seat connection checks its lease and silence.
pub(crate) const SHARED_SEAT_CHECK_INTERVAL: Duration = Duration::from_secs(5);

#[derive(Debug)]
pub(crate) enum SharedLifecycleError {
    NotFound,
    Store(MatchStoreError),
    Loadout(LoadoutResolutionError),
}

impl From<MatchStoreError> for SharedLifecycleError {
    fn from(error: MatchStoreError) -> Self {
        Self::Store(error)
    }
}

/// What a seat asks for when it joins.
pub(crate) struct SharedSeatJoin<'a> {
    pub(crate) match_id: &'a str,
    pub(crate) seat_token: &'a str,
    pub(crate) user_id: Option<i64>,
    pub(crate) hero_type: HeroType,
    pub(crate) deck_choice: Option<DeckChoiceRequest>,
    pub(crate) legacy_deck_id: Option<i64>,
    pub(crate) requested_rune_ids: Option<Vec<String>>,
}

pub(crate) struct SharedMatchLifecycle {
    state: SharedState,
}

impl SharedMatchLifecycle {
    pub(crate) fn new(state: SharedState) -> Self {
        Self { state }
    }

    /// Setup: creates a Shared match with one private seat link per side.
    pub(crate) fn create(
        &self,
        creator_user_id: Option<i64>,
        format: SharedMatchFormat,
    ) -> Result<CreatedSharedMatch, MatchStoreError> {
        self.store().create_shared_match(creator_user_id, format)
    }

    /// Loads the seat's side-specific view and records that the seat was seen.
    pub(crate) fn open_seat(
        &self,
        match_id: &str,
        seat_token: &str,
    ) -> Result<StoredSharedMatch, SharedLifecycleError> {
        let mut store = self.store();
        store.mark_shared_seat_seen(match_id, seat_token)?;
        store
            .load_shared_match_for_seat(match_id, seat_token)?
            .ok_or(SharedLifecycleError::NotFound)
    }

    /// Whether the seat link names a seat of this match.
    pub(crate) fn seat_exists(
        &self,
        match_id: &str,
        seat_token: &str,
    ) -> Result<bool, MatchStoreError> {
        Ok(self
            .store()
            .load_shared_match_for_seat(match_id, seat_token)?
            .is_some())
    }

    /// Joins a seat with its frozen loadout. The match becomes active once every seat is ready;
    /// the other seats are notified.
    pub(crate) fn join(
        &self,
        join: SharedSeatJoin<'_>,
    ) -> Result<StoredSharedMatch, SharedLifecycleError> {
        let shared = {
            let mut store = self.store();
            let loadout = resolve_shared_seat_loadout(
                &mut store,
                SharedSeatLoadoutRequest {
                    match_id: join.match_id,
                    seat_token: join.seat_token,
                    user_id: join.user_id,
                    hero_type: join.hero_type,
                    deck_choice: join.deck_choice,
                    legacy_deck_id: join.legacy_deck_id,
                    requested_rune_ids: join.requested_rune_ids,
                },
            )
            .map_err(SharedLifecycleError::Loadout)?;
            store
                .join_shared_match(
                    join.match_id,
                    join.seat_token,
                    loadout.hero_type,
                    loadout.deck_recipe,
                    loadout.progression,
                    join.user_id,
                )?
                .ok_or(SharedLifecycleError::NotFound)?
        };
        self.state.notify_match(join.match_id);
        Ok(shared)
    }

    /// Presence: opens a live connection for the seat. A newer connection for the same seat
    /// replaces the previous one.
    pub(crate) fn connect(
        &self,
        match_id: &str,
        seat_token: &str,
    ) -> Result<SharedSeatConnection, SessionError> {
        let lease = self.state.connect_shared_seat(match_id, seat_token)?;
        let _ = self.store().mark_shared_seat_seen(match_id, seat_token);
        Ok(SharedSeatConnection {
            state: self.state.clone(),
            match_id: match_id.to_string(),
            seat_token: seat_token.to_string(),
            lease,
        })
    }

    fn store(&self) -> std::sync::MutexGuard<'_, crate::match_store::SqliteMatchStore> {
        self.state
            .store
            .lock()
            .expect("store lock should not be poisoned")
    }
}

/// The reply to one client message and whether the other seats must be notified.
pub(crate) struct SharedClientOutcome {
    pub(crate) reply: Option<SharedServerMessage>,
    pub(crate) notify_others: bool,
    /// `false` when this connection lost its lease and must close.
    pub(crate) keep_open: bool,
}

/// One live seat connection holding a `game-server` session lease.
pub(crate) struct SharedSeatConnection {
    state: SharedState,
    match_id: String,
    seat_token: String,
    lease: SessionLease,
}

impl SharedSeatConnection {
    pub(crate) fn match_id(&self) -> &str {
        &self.match_id
    }

    /// Whether this is still the seat's current connection.
    pub(crate) fn is_current(&self) -> bool {
        self.state
            .owns_shared_seat_connection(&self.match_id, &self.seat_token, self.lease)
    }

    /// Whether the connection must close: replaced by a newer one, or silent for too long.
    pub(crate) fn should_close(&self, silent_for: Duration) -> bool {
        !self.is_current() || silent_for > SHARED_SEAT_SILENCE_LIMIT
    }

    /// The seat's side-specific snapshot.
    pub(crate) fn snapshot(&self) -> Option<SharedServerMessage> {
        let shared = self
            .state
            .store
            .lock()
            .expect("store lock should not be poisoned")
            .load_shared_match_for_seat(&self.match_id, &self.seat_token)
            .ok()
            .flatten()?;
        Some(SharedServerMessage::Snapshot {
            payload: SharedMatchResponse::from(shared),
        })
    }

    /// Announces this seat's presence to the other seats once its first snapshot was sent.
    pub(crate) fn announce(&self) {
        self.state.notify_match(&self.match_id);
    }

    /// Handles one text message from the client.
    pub(crate) fn handle_text(&self, text: &str, now: i64) -> SharedClientOutcome {
        let message = match serde_json::from_str::<SharedClientMessage>(text) {
            Ok(message) => message,
            Err(error) => {
                return SharedClientOutcome {
                    reply: Some(SharedServerMessage::Error {
                        message: format!("Invalid WebSocket message: {error}"),
                    }),
                    notify_others: false,
                    keep_open: true,
                };
            }
        };

        match message {
            SharedClientMessage::Action { request_id, action } => {
                self.respond(request_id, || self.apply_action(action))
            }
            SharedClientMessage::ClaimForfeit { request_id } => {
                self.respond(request_id, || self.claim_forfeit(now))
            }
            SharedClientMessage::Heartbeat => {
                let Some(result) = self.while_current(|| {
                    self.state
                        .store
                        .lock()
                        .expect("store lock should not be poisoned")
                        .mark_shared_seat_seen(&self.match_id, &self.seat_token)
                }) else {
                    return Self::closed();
                };
                SharedClientOutcome {
                    reply: result.err().map(|error| SharedServerMessage::Error {
                        message: error.to_string(),
                    }),
                    notify_others: false,
                    keep_open: true,
                }
            }
        }
    }

    /// Disconnect: releases the lease and, if it was still current, marks the seat
    /// disconnected (which starts the forfeit timer) and notifies the other seats.
    pub(crate) fn finish(self) {
        if !self
            .state
            .disconnect_shared_seat(&self.match_id, &self.seat_token, self.lease)
        {
            return;
        }
        let _ = self
            .state
            .store
            .lock()
            .expect("store lock should not be poisoned")
            .mark_shared_seat_disconnected(&self.match_id, &self.seat_token);
        self.state.notify_match(&self.match_id);
    }

    fn respond(
        &self,
        request_id: String,
        command: impl FnOnce() -> Result<SharedMatchResponse, String>,
    ) -> SharedClientOutcome {
        let Some(result) = self.while_current(command) else {
            return Self::closed();
        };
        match result {
            Ok(payload) => SharedClientOutcome {
                reply: Some(SharedServerMessage::ActionAccepted {
                    request_id,
                    payload,
                }),
                notify_others: true,
                keep_open: true,
            },
            Err(message) => SharedClientOutcome {
                reply: Some(SharedServerMessage::ActionRejected {
                    request_id,
                    message,
                }),
                notify_others: false,
                keep_open: true,
            },
        }
    }

    fn apply_action(&self, action: MatchActionRequest) -> Result<SharedMatchResponse, String> {
        let mut store = self
            .state
            .store
            .lock()
            .expect("store lock should not be poisoned");
        MatchCommands::new(&mut store)
            .apply_shared_seat_action(&self.match_id, &self.seat_token, action)
            .map(|applied| SharedMatchResponse::from(applied.shared_match))
            .map_err(|error| error.to_string())
    }

    /// The forfeit claim: allowed once every opposing seat has been disconnected for the
    /// claim delay (policy in `MatchCommands::claim_shared_forfeit`).
    fn claim_forfeit(&self, now: i64) -> Result<SharedMatchResponse, String> {
        let mut store = self
            .state
            .store
            .lock()
            .expect("store lock should not be poisoned");
        MatchCommands::new(&mut store)
            .claim_shared_forfeit(&self.match_id, &self.seat_token, now)
            .map(|applied| SharedMatchResponse::from(applied.shared_match))
            .map_err(|error| error.to_string())
    }

    fn while_current<T>(&self, action: impl FnOnce() -> T) -> Option<T> {
        self.state
            .with_shared_seat_connection(&self.match_id, &self.seat_token, self.lease, action)
    }

    fn closed() -> SharedClientOutcome {
        SharedClientOutcome {
            reply: None,
            notify_others: false,
            keep_open: false,
        }
    }
}

/// Seconds since the Unix epoch, for the forfeit claim.
pub(crate) fn unix_timestamp() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests;
