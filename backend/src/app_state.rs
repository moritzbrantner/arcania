use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use game_server::{SessionError, SessionLease};
use tokio::sync::broadcast;

use crate::match_store::SqliteMatchStore;
use crate::multiplayer_sessions::MultiplayerSessions;

pub(crate) type SharedState = Arc<AppState>;

pub(crate) struct AppState {
    pub(crate) store: Mutex<SqliteMatchStore>,
    live_matches: Mutex<HashMap<String, broadcast::Sender<()>>>,
    multiplayer_sessions: Mutex<MultiplayerSessions>,
}

impl AppState {
    pub(crate) fn new(store: SqliteMatchStore) -> Self {
        Self {
            store: Mutex::new(store),
            live_matches: Mutex::new(HashMap::new()),
            multiplayer_sessions: Mutex::new(MultiplayerSessions::new()),
        }
    }

    pub(crate) fn match_sender(&self, match_id: &str) -> broadcast::Sender<()> {
        let mut live_matches = self
            .live_matches
            .lock()
            .expect("live match lock should not be poisoned");
        live_matches
            .entry(match_id.to_string())
            .or_insert_with(|| {
                let (sender, _) = broadcast::channel(32);
                sender
            })
            .clone()
    }

    pub(crate) fn notify_match(&self, match_id: &str) {
        let sender = self.match_sender(match_id);
        let _ = sender.send(());
    }

    pub(crate) fn connect_shared_seat(
        &self,
        match_id: &str,
        seat_token: &str,
    ) -> Result<SessionLease, SessionError> {
        self.multiplayer_sessions
            .lock()
            .expect("multiplayer session lock should not be poisoned")
            .connect(match_id, seat_token)
    }

    pub(crate) fn owns_shared_seat_connection(
        &self,
        match_id: &str,
        seat_token: &str,
        lease: SessionLease,
    ) -> bool {
        self.multiplayer_sessions
            .lock()
            .expect("multiplayer session lock should not be poisoned")
            .owns_connection(match_id, seat_token, lease)
    }

    pub(crate) fn disconnect_shared_seat(
        &self,
        match_id: &str,
        seat_token: &str,
        lease: SessionLease,
    ) -> bool {
        self.multiplayer_sessions
            .lock()
            .expect("multiplayer session lock should not be poisoned")
            .disconnect(match_id, seat_token, lease)
    }
}
