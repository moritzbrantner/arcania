use crate::identity::AccountProfile;
use crate::match_store::{MatchStoreError, SqliteMatchStore, StoredMatchSummary};

#[derive(Clone, Debug)]
pub enum Actor {
    Anonymous,
    Account {
        user_id: i64,
    },
    Seat {
        match_id: String,
        seat_token: String,
    },
}

pub struct MatchAccess<'a> {
    store: &'a SqliteMatchStore,
}

impl<'a> MatchAccess<'a> {
    pub fn new(store: &'a SqliteMatchStore) -> Self {
        Self { store }
    }

    pub fn can_load_match(&self, actor: &Actor, match_id: &str) -> bool {
        self.account_can_access_owned_match(actor, match_id)
    }

    pub fn can_apply_solo_action(&self, actor: &Actor, match_id: &str) -> bool {
        self.account_can_access_owned_match(actor, match_id)
    }

    pub fn list_profile_matches(
        &self,
        user_id: i64,
    ) -> Result<Vec<StoredMatchSummary>, MatchStoreError> {
        self.store.list_profile_matches(user_id)
    }

    fn account_can_access_owned_match(&self, actor: &Actor, match_id: &str) -> bool {
        match self.store.match_owner_user_id(match_id) {
            Ok(Some(Some(owner_user_id))) => {
                matches!(actor, Actor::Account { user_id } if *user_id == owner_user_id)
            }
            Ok(Some(None)) => true,
            Ok(None) | Err(_) => false,
        }
    }
}

impl From<Option<&AccountProfile>> for Actor {
    fn from(profile: Option<&AccountProfile>) -> Self {
        match profile {
            Some(profile) => Self::Account {
                user_id: profile.id,
            },
            None => Self::Anonymous,
        }
    }
}
