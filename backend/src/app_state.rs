use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use settings_core::SettingsRegistry;
use tokio::sync::broadcast;

use crate::match_store::SqliteMatchStore;
use crate::settings_foundation::settings_registry;

pub(crate) type SharedState = Arc<AppState>;

pub(crate) struct AppState {
    pub(crate) store: Mutex<SqliteMatchStore>,
    _settings_registry: SettingsRegistry,
    live_matches: Mutex<HashMap<String, broadcast::Sender<()>>>,
}

impl AppState {
    pub(crate) fn new(store: SqliteMatchStore) -> Self {
        Self {
            store: Mutex::new(store),
            _settings_registry: settings_registry(),
            live_matches: Mutex::new(HashMap::new()),
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
}
