use std::collections::HashMap;

use game_server::{
    RECONNECT_TOKEN_BYTES, ReconnectToken, SessionError, SessionLease, SessionRegistry,
};
use rand_core::{OsRng, RngCore};

const MAX_SHARED_SEATS: usize = 4;
const DURABLE_SEAT_RECONNECT_GRACE_TICKS: u64 = u64::MAX;

pub(crate) struct MultiplayerSessions {
    matches: HashMap<String, MatchSessions>,
    tick: u64,
}

struct MatchSessions {
    registry: SessionRegistry,
    seats: HashMap<String, SessionLease>,
}

impl MultiplayerSessions {
    pub(crate) fn new() -> Self {
        Self {
            matches: HashMap::new(),
            tick: 0,
        }
    }

    pub(crate) fn connect(
        &mut self,
        match_id: &str,
        seat_token: &str,
    ) -> Result<SessionLease, SessionError> {
        self.tick = self.tick.saturating_add(1);
        let current_tick = self.tick;
        let sessions = self
            .matches
            .entry(match_id.to_string())
            .or_insert_with(MatchSessions::new);

        let lease = if let Some(previous) = sessions.seats.get(seat_token).copied() {
            let _ = sessions.registry.disconnect(
                previous.player_id,
                previous.connection_epoch,
                current_tick,
            );
            reconnect_with_fresh_token(
                &mut sessions.registry,
                previous.reconnect_token,
                current_tick,
            )?
        } else {
            admit_with_fresh_token(&mut sessions.registry)?
        };

        sessions.seats.insert(seat_token.to_string(), lease);
        Ok(lease)
    }

    pub(crate) fn owns_connection(
        &self,
        match_id: &str,
        seat_token: &str,
        lease: SessionLease,
    ) -> bool {
        self.matches.get(match_id).is_some_and(|sessions| {
            sessions.seats.get(seat_token).copied() == Some(lease)
                && sessions
                    .registry
                    .owns_connection(lease.player_id, lease.connection_epoch)
        })
    }

    pub(crate) fn disconnect(
        &mut self,
        match_id: &str,
        seat_token: &str,
        lease: SessionLease,
    ) -> bool {
        self.tick = self.tick.saturating_add(1);
        let current_tick = self.tick;
        let Some(sessions) = self.matches.get_mut(match_id) else {
            return false;
        };
        if sessions.seats.get(seat_token).copied() != Some(lease) {
            return false;
        }

        sessions
            .registry
            .disconnect(lease.player_id, lease.connection_epoch, current_tick)
    }
}

impl MatchSessions {
    fn new() -> Self {
        Self {
            registry: SessionRegistry::new(MAX_SHARED_SEATS, DURABLE_SEAT_RECONNECT_GRACE_TICKS),
            seats: HashMap::new(),
        }
    }
}

fn admit_with_fresh_token(registry: &mut SessionRegistry) -> Result<SessionLease, SessionError> {
    loop {
        match registry.admit(fresh_reconnect_token()) {
            Err(SessionError::TokenCollision) => continue,
            result => return result,
        }
    }
}

fn reconnect_with_fresh_token(
    registry: &mut SessionRegistry,
    previous_token: ReconnectToken,
    current_tick: u64,
) -> Result<SessionLease, SessionError> {
    loop {
        match registry.reconnect(previous_token, fresh_reconnect_token(), current_tick) {
            Err(SessionError::TokenCollision) => continue,
            result => return result,
        }
    }
}

fn fresh_reconnect_token() -> ReconnectToken {
    let mut bytes = [0_u8; RECONNECT_TOKEN_BYTES];
    let mut rng = OsRng;
    rng.fill_bytes(&mut bytes);
    ReconnectToken(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replacement_connection_preserves_player_identity_and_fences_old_epoch() {
        let mut sessions = MultiplayerSessions::new();
        let first = sessions.connect("rl-test", "seat-a").unwrap();
        let second = sessions.connect("rl-test", "seat-a").unwrap();

        assert_eq!(second.player_id, first.player_id);
        assert_eq!(second.connection_epoch, first.connection_epoch + 1);
        assert!(!sessions.owns_connection("rl-test", "seat-a", first));
        assert!(sessions.owns_connection("rl-test", "seat-a", second));

        assert!(!sessions.disconnect("rl-test", "seat-a", first));
        assert!(sessions.owns_connection("rl-test", "seat-a", second));
        assert!(sessions.disconnect("rl-test", "seat-a", second));
        assert!(!sessions.owns_connection("rl-test", "seat-a", second));
    }

    #[test]
    fn different_seats_receive_distinct_player_identities() {
        let mut sessions = MultiplayerSessions::new();
        let first = sessions.connect("rl-test", "seat-a").unwrap();
        let second = sessions.connect("rl-test", "seat-b").unwrap();

        assert_ne!(first.player_id, second.player_id);
        assert!(sessions.owns_connection("rl-test", "seat-a", first));
        assert!(sessions.owns_connection("rl-test", "seat-b", second));
    }
}
