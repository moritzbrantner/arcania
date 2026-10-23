use crate::app_state::{AppState, SharedState};
use crate::auth_context::*;
use crate::card_catalog::{CatalogResponse, starter_catalog};
use crate::card_workshop::evidence::CardEvidenceRequest;
use crate::card_workshop::scenarios::CardDraftScenarioRequest;
use crate::card_workshop::{
    CardWorkshop, CreateCardDraftRequest, PublishCardDraftRequest, UpdateCardDraftRequest,
};
use crate::deck_library::{DeckLibrary, SaveDeckRequest, system_deck_response};
use crate::deck_recipe_legality::DeckLegalityPreviewRequest;
use crate::http_errors::*;
use crate::http_types::*;
use crate::identity;
use crate::identity::{GeneratedAvatar, IdentityModule};
use crate::loadout_resolution::*;
use crate::match_access::{Actor, MatchAccess};
use crate::match_commands::MatchCommands;
use crate::match_session::{HeroType, MatchActionRequest, Side};
use crate::match_store::{SharedMatchFormat, SqliteMatchStore};
use crate::preferences::{PreferencesModule, UpdatePreferencesRequest};
use crate::progression::{
    ProgressionError, ProgressionModule, ProgressionResponse, SaveRuneLoadoutRequest,
};
use crate::replay_reads::{MatchSummaryRead, ReplayRead, ReplayReadError, ReplayReads};
use crate::shared_match_lifecycle::{
    SHARED_SEAT_CHECK_INTERVAL, SharedLifecycleError, SharedMatchLifecycle, SharedSeatJoin,
    unix_timestamp,
};
use axum::body::Bytes;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::http::{HeaderMap, Method, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use futures_util::StreamExt;
use std::sync::Arc;
use tokio::time::{self, Instant};
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};

pub fn create_app(store: SqliteMatchStore) -> Router {
    let state = Arc::new(AppState::new(store));
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
        .allow_headers(Any);

    let static_files = ServeDir::new("frontend/dist")
        .not_found_service(ServeFile::new("frontend/dist/index.html"));

    let router = Router::new()
        .route("/api/health", get(health))
        .route("/api/catalog/cards", get(catalog_cards))
        .route(
            "/api/card-drafts",
            get(list_card_drafts).post(create_card_draft),
        )
        .route(
            "/api/card-drafts/{draft_id}",
            get(load_card_draft)
                .patch(update_card_draft)
                .delete(delete_card_draft),
        )
        .route(
            "/api/card-drafts/{draft_id}/publish",
            post(publish_card_draft),
        )
        .route(
            "/api/card-drafts/{draft_id}/scenarios/run",
            post(run_card_draft_scenario),
        )
        .route("/api/card-revisions/{card_id}", get(card_revision_history))
        .route("/api/card-evidence/run", post(run_card_evidence))
        .route(
            "/api/card-transfers/{cardId}/{revision}",
            get(export_card_revision),
        )
        .route("/api/card-transfers/preview", post(preview_card_transfer))
        .route("/api/card-transfers/import", post(import_card_transfer))
        .route(
            "/api/card-pack-transfers/{packId}/{version}",
            get(export_card_pack),
        )
        .route("/api/card-pack-transfers/import", post(import_card_pack))
        .route("/api/card-pack-transfers/preview", post(preview_card_pack))
        .route(
            "/api/card-revisions/{card_id}/{revision}/fork",
            post(fork_card_revision),
        )
        .route("/api/system-decks", get(system_decks))
        .route("/api/users/{handle}/decks/{deck_id}", get(load_public_deck))
        .route("/api/decks", get(list_decks).post(create_deck))
        .route("/api/decks/legality-preview", post(preview_deck_legality))
        .route(
            "/api/decks/{deck_id}",
            get(load_deck).patch(update_deck).delete(delete_deck),
        )
        .route("/api/decks/{deck_id}/duplicate", post(duplicate_deck))
        .route("/api/auth/register", post(register_account))
        .route("/api/auth/login", post(login_account))
        .route("/api/auth/logout", post(logout_account))
        .route("/api/auth/me", get(current_account))
        .route("/api/profile", get(load_profile).patch(update_profile))
        .route("/api/profile/preferred-hero", patch(update_preferred_hero))
        .route("/api/profile/matches", get(list_profile_matches))
        .route(
            "/api/preferences",
            get(load_preferences).patch(update_preferences),
        )
        .route("/api/progression", get(load_progression))
        .route(
            "/api/progression/heroes/{hero_type}/skills/{node_id}",
            post(unlock_progression_skill),
        )
        .route(
            "/api/progression/heroes/{hero_type}/respec",
            post(respec_progression_hero),
        )
        .route(
            "/api/progression/heroes/{hero_type}/loadout",
            patch(save_progression_loadout),
        )
        .route("/api/matches", get(list_matches).post(create_match))
        .route("/api/matches/{match_id}", get(load_match))
        .route("/api/matches/{match_id}/summary", get(load_match_summary))
        .route("/api/matches/{match_id}/replay", get(load_replay))
        .route("/api/matches/{match_id}/actions", post(apply_match_action))
        .route("/api/shared-matches", post(create_shared_match))
        .route(
            "/api/shared-matches/{match_id}/seats/{seat_token}",
            get(load_shared_match),
        )
        .route(
            "/api/shared-matches/{match_id}/seats/{seat_token}/join",
            post(join_shared_match),
        )
        .route(
            "/api/shared-matches/{match_id}/seats/{seat_token}/summary",
            get(load_shared_match_summary),
        )
        .route(
            "/api/shared-matches/{match_id}/seats/{seat_token}/replay",
            get(load_shared_replay),
        )
        .route(
            "/api/shared-matches/{match_id}/seats/{seat_token}/ws",
            get(shared_match_ws),
        );

    #[cfg(debug_assertions)]
    let router = router
        .route("/api/dev/match-scenarios", get(list_match_scenarios))
        .route(
            "/api/dev/match-scenarios/{scenario_id}/matches",
            post(create_match_scenario),
        );

    router
        .layer(cors)
        .fallback_service(static_files)
        .with_state(state)
}

async fn health() -> &'static str {
    "ok"
}

async fn catalog_cards() -> impl IntoResponse {
    Json(CatalogResponse {
        cards: starter_catalog(),
    })
}

async fn list_card_drafts(
    State(state): State<SharedState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let profile = match required_profile_from_headers(&state, &headers) {
        Ok(profile) => profile,
        Err(response) => return response,
    };
    let response = {
        let mut store = state
            .store
            .lock()
            .expect("store lock should not be poisoned");
        let workshop = CardWorkshop::new(store.connection_mut());
        match workshop.list_drafts_for_user(profile.id) {
            Ok(response) => response,
            Err(error) => return card_workshop_error_response(error),
        }
    };
    Json(response).into_response()
}

async fn create_card_draft(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Json(request): Json<CreateCardDraftRequest>,
) -> impl IntoResponse {
    let profile = match required_profile_from_headers(&state, &headers) {
        Ok(profile) => profile,
        Err(response) => return response,
    };
    let draft = {
        let mut store = state
            .store
            .lock()
            .expect("store lock should not be poisoned");
        let mut workshop = CardWorkshop::new(store.connection_mut());
        match workshop.create_draft_for_user(profile.id, request) {
            Ok(draft) => draft,
            Err(error) => return card_workshop_error_response(error),
        }
    };
    Json(draft).into_response()
}

async fn load_card_draft(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(draft_id): Path<i64>,
) -> impl IntoResponse {
    let profile = match required_profile_from_headers(&state, &headers) {
        Ok(profile) => profile,
        Err(response) => return response,
    };
    let draft = {
        let mut store = state
            .store
            .lock()
            .expect("store lock should not be poisoned");
        let workshop = CardWorkshop::new(store.connection_mut());
        match workshop.load_draft_for_user(profile.id, draft_id) {
            Ok(Some(draft)) => draft,
            Ok(None) => {
                return card_workshop_error_response(
                    crate::card_workshop::CardWorkshopError::NotFound,
                );
            }
            Err(error) => return card_workshop_error_response(error),
        }
    };
    Json(draft).into_response()
}

async fn update_card_draft(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(draft_id): Path<i64>,
    Json(request): Json<UpdateCardDraftRequest>,
) -> impl IntoResponse {
    let profile = match required_profile_from_headers(&state, &headers) {
        Ok(profile) => profile,
        Err(response) => return response,
    };
    let draft = {
        let mut store = state
            .store
            .lock()
            .expect("store lock should not be poisoned");
        let mut workshop = CardWorkshop::new(store.connection_mut());
        match workshop.update_draft_for_user(profile.id, draft_id, request) {
            Ok(draft) => draft,
            Err(error) => return card_workshop_error_response(error),
        }
    };
    Json(draft).into_response()
}

async fn delete_card_draft(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(draft_id): Path<i64>,
) -> impl IntoResponse {
    let profile = match required_profile_from_headers(&state, &headers) {
        Ok(profile) => profile,
        Err(response) => return response,
    };
    let deleted = {
        let mut store = state
            .store
            .lock()
            .expect("store lock should not be poisoned");
        let mut workshop = CardWorkshop::new(store.connection_mut());
        match workshop.delete_draft_for_user(profile.id, draft_id) {
            Ok(deleted) => deleted,
            Err(error) => return card_workshop_error_response(error),
        }
    };
    if !deleted {
        return card_workshop_error_response(crate::card_workshop::CardWorkshopError::NotFound);
    }
    Json(AuthMessageResponse {
        message: "Card draft deleted",
    })
    .into_response()
}

async fn publish_card_draft(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(draft_id): Path<i64>,
    Json(request): Json<PublishCardDraftRequest>,
) -> impl IntoResponse {
    let profile = match required_profile_from_headers(&state, &headers) {
        Ok(profile) => profile,
        Err(response) => return response,
    };
    let revision = {
        let mut store = state
            .store
            .lock()
            .expect("store lock should not be poisoned");
        let mut workshop = CardWorkshop::new(store.connection_mut());
        match workshop.publish_draft_for_user(profile.id, draft_id, request) {
            Ok(revision) => revision,
            Err(error) => return card_workshop_error_response(error),
        }
    };
    Json(revision).into_response()
}

async fn run_card_draft_scenario(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(draft_id): Path<i64>,
    Json(request): Json<CardDraftScenarioRequest>,
) -> impl IntoResponse {
    let profile = match required_profile_from_headers(&state, &headers) {
        Ok(profile) => profile,
        Err(response) => return response,
    };
    let prepared = {
        let mut store = state
            .store
            .lock()
            .expect("store lock should not be poisoned");
        let workshop = CardWorkshop::new(store.connection_mut());
        match workshop.prepare_draft_scenario_for_user(profile.id, draft_id, request) {
            Ok(prepared) => prepared,
            Err(error) => return card_workshop_error_response(error),
        }
    };
    match prepared
        .run()
        .and_then(CardDraftScenarioResponse::from_result)
    {
        Ok(result) => Json(result).into_response(),
        Err(error) => card_workshop_error_response(error),
    }
}

async fn run_card_evidence(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Json(request): Json<CardEvidenceRequest>,
) -> impl IntoResponse {
    let profile = match required_profile_from_headers(&state, &headers) {
        Ok(profile) => profile,
        Err(response) => return response,
    };
    let input = {
        let mut store = state
            .store
            .lock()
            .expect("store lock should not be poisoned");
        let workshop = CardWorkshop::new(store.connection_mut());
        match workshop.resolve_evidence_for_user(profile.id, request) {
            Ok(input) => input,
            Err(error) => return card_workshop_error_response(error),
        }
    };
    match tokio::task::spawn_blocking(move || crate::ai_lab::card_evidence::run_comparison(input))
        .await
    {
        Ok(Ok(report)) => Json(report).into_response(),
        Ok(Err(error)) => {
            card_workshop_error_response(crate::card_workshop::CardWorkshopError::Evidence(error))
        }
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                message: format!("Card evidence task failed: {error}"),
            }),
        )
            .into_response(),
    }
}

async fn card_revision_history(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(card_id): Path<String>,
) -> impl IntoResponse {
    let profile = match required_profile_from_headers(&state, &headers) {
        Ok(profile) => profile,
        Err(response) => return response,
    };
    let history = {
        let mut store = state
            .store
            .lock()
            .expect("store lock should not be poisoned");
        let workshop = CardWorkshop::new(store.connection_mut());
        match workshop.revision_history_for_user(profile.id, &card_id) {
            Ok(history) => history,
            Err(error) => return card_workshop_error_response(error),
        }
    };
    Json(history).into_response()
}

async fn export_card_revision(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(id): Path<rune_lanes_core::CardRevisionId>,
) -> impl IntoResponse {
    let profile = match required_profile_from_headers(&state, &headers) {
        Ok(profile) => profile,
        Err(response) => return response,
    };
    let response = {
        let mut store = state
            .store
            .lock()
            .expect("store lock should not be poisoned");
        let workshop = CardWorkshop::new(store.connection_mut());
        match workshop.export_revision_for_user(profile.id, &id) {
            Ok(response) => response,
            Err(error) => return card_workshop_error_response(error),
        }
    };
    Json(response).into_response()
}

async fn preview_card_transfer(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Json(transfer): Json<rune_lanes_core::CardRevisionTransfer>,
) -> impl IntoResponse {
    let profile = match required_profile_from_headers(&state, &headers) {
        Ok(profile) => profile,
        Err(response) => return response,
    };
    let response = {
        let mut store = state
            .store
            .lock()
            .expect("store lock should not be poisoned");
        let workshop = CardWorkshop::new(store.connection_mut());
        match workshop.preview_transfer_for_user(profile.id, &transfer) {
            Ok(response) => response,
            Err(error) => return card_workshop_error_response(error),
        }
    };
    Json(response).into_response()
}

async fn import_card_transfer(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Json(transfer): Json<rune_lanes_core::CardRevisionTransfer>,
) -> impl IntoResponse {
    let profile = match required_profile_from_headers(&state, &headers) {
        Ok(profile) => profile,
        Err(response) => return response,
    };
    let response = {
        let mut store = state
            .store
            .lock()
            .expect("store lock should not be poisoned");
        let mut workshop = CardWorkshop::new(store.connection_mut());
        match workshop.import_transfer_for_user(profile.id, transfer) {
            Ok(response) => response,
            Err(error) => return card_workshop_error_response(error),
        }
    };
    Json(response).into_response()
}

async fn export_card_pack(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(id): Path<rune_lanes_core::CardPackRevisionId>,
) -> impl IntoResponse {
    let profile = match required_profile_from_headers(&state, &headers) {
        Ok(profile) => profile,
        Err(response) => return response,
    };
    let response = {
        let mut store = state
            .store
            .lock()
            .expect("store lock should not be poisoned");
        let workshop = CardWorkshop::new(store.connection_mut());
        match workshop.export_pack_for_user(profile.id, &id) {
            Ok(response) => response,
            Err(error) => return card_workshop_error_response(error),
        }
    };
    Json(response).into_response()
}

async fn preview_card_pack(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Json(transfer): Json<rune_lanes_core::CardPackTransfer>,
) -> impl IntoResponse {
    let profile = match required_profile_from_headers(&state, &headers) {
        Ok(profile) => profile,
        Err(response) => return response,
    };
    let response = {
        let mut store = state
            .store
            .lock()
            .expect("store lock should not be poisoned");
        let workshop = CardWorkshop::new(store.connection_mut());
        match workshop.preview_pack_for_user(profile.id, &transfer) {
            Ok(response) => response,
            Err(error) => return card_workshop_error_response(error),
        }
    };
    Json(response).into_response()
}

async fn import_card_pack(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Json(transfer): Json<rune_lanes_core::CardPackTransfer>,
) -> impl IntoResponse {
    let profile = match required_profile_from_headers(&state, &headers) {
        Ok(profile) => profile,
        Err(response) => return response,
    };
    let response = {
        let mut store = state
            .store
            .lock()
            .expect("store lock should not be poisoned");
        let mut workshop = CardWorkshop::new(store.connection_mut());
        match workshop.import_pack_for_user(profile.id, transfer) {
            Ok(response) => response,
            Err(error) => return card_workshop_error_response(error),
        }
    };
    Json(response).into_response()
}

async fn fork_card_revision(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path((card_id, revision)): Path<(String, u32)>,
) -> impl IntoResponse {
    let profile = match required_profile_from_headers(&state, &headers) {
        Ok(profile) => profile,
        Err(response) => return response,
    };
    let draft = {
        let mut store = state
            .store
            .lock()
            .expect("store lock should not be poisoned");
        let mut workshop = CardWorkshop::new(store.connection_mut());
        match workshop.fork_revision_for_user(profile.id, &card_id, revision) {
            Ok(draft) => draft,
            Err(error) => return card_workshop_error_response(error),
        }
    };
    Json(draft).into_response()
}

async fn system_decks() -> impl IntoResponse {
    Json(system_deck_response())
}

async fn list_decks(State(state): State<SharedState>, headers: HeaderMap) -> impl IntoResponse {
    let profile = match required_profile_from_headers(&state, &headers) {
        Ok(profile) => profile,
        Err(response) => return response,
    };
    let response = {
        let mut store = state
            .store
            .lock()
            .expect("store lock should not be poisoned");
        let mut decks = DeckLibrary::new(store.connection_mut());
        match decks.list_for_user(profile.id) {
            Ok(response) => response,
            Err(error) => return deck_error_response(error),
        }
    };
    Json(response).into_response()
}

async fn create_deck(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Json(request): Json<SaveDeckRequest>,
) -> impl IntoResponse {
    let profile = match required_profile_from_headers(&state, &headers) {
        Ok(profile) => profile,
        Err(response) => return response,
    };
    let request = default_deck_configuration(request, profile.preferred_hero_type);
    let deck = {
        let mut store = state
            .store
            .lock()
            .expect("store lock should not be poisoned");
        if let Err(error) = validate_deck_configuration(&mut store, profile.id, &request) {
            return progression_error_response(error);
        }
        let mut decks = DeckLibrary::new(store.connection_mut());
        match decks.create_for_user(profile.id, request) {
            Ok(deck) => deck,
            Err(error) => return deck_error_response(error),
        }
    };
    Json(deck).into_response()
}

async fn preview_deck_legality(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Json(request): Json<DeckLegalityPreviewRequest>,
) -> impl IntoResponse {
    if let Err(response) = required_profile_from_headers(&state, &headers) {
        return response;
    }

    Json(crate::deck_recipe_legality::preview_requested_cards(
        request.cards,
    ))
    .into_response()
}

async fn load_deck(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(deck_id): Path<i64>,
) -> impl IntoResponse {
    let profile = match required_profile_from_headers(&state, &headers) {
        Ok(profile) => profile,
        Err(response) => return response,
    };
    let deck = {
        let mut store = state
            .store
            .lock()
            .expect("store lock should not be poisoned");
        let mut decks = DeckLibrary::new(store.connection_mut());
        match decks.load_for_user(profile.id, deck_id) {
            Ok(Some(deck)) => deck,
            Ok(None) => return deck_not_found_response(),
            Err(error) => return deck_error_response(error),
        }
    };
    Json(deck).into_response()
}

async fn load_public_deck(
    State(state): State<SharedState>,
    Path((handle, deck_id)): Path<(String, i64)>,
) -> impl IntoResponse {
    let response = {
        let mut store = state
            .store
            .lock()
            .expect("store lock should not be poisoned");
        let owner = {
            let identity = IdentityModule::new(store.connection_mut());
            match identity.public_profile_by_handle(&handle) {
                Ok(Some(owner)) => owner,
                Ok(None) => return deck_not_found_response(),
                Err(error) => return identity_error_response(error),
            }
        };
        let decks = DeckLibrary::new(store.connection_mut());
        let deck = match decks.load_public_for_user(owner.id, deck_id) {
            Ok(Some(deck)) => deck,
            Ok(None) => return deck_not_found_response(),
            Err(error) => return deck_error_response(error),
        };
        PublicDeckRecipeResponse {
            owner: PublicDeckOwnerResponse::from(owner),
            deck,
        }
    };

    Json(response).into_response()
}

async fn update_deck(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(deck_id): Path<i64>,
    Json(request): Json<SaveDeckRequest>,
) -> impl IntoResponse {
    let profile = match required_profile_from_headers(&state, &headers) {
        Ok(profile) => profile,
        Err(response) => return response,
    };
    let request = default_deck_configuration(request, profile.preferred_hero_type);
    let deck = {
        let mut store = state
            .store
            .lock()
            .expect("store lock should not be poisoned");
        if let Err(error) = validate_deck_configuration(&mut store, profile.id, &request) {
            return progression_error_response(error);
        }
        let mut decks = DeckLibrary::new(store.connection_mut());
        match decks.update_for_user(profile.id, deck_id, request) {
            Ok(Some(deck)) => deck,
            Ok(None) => return deck_not_found_response(),
            Err(error) => return deck_error_response(error),
        }
    };
    Json(deck).into_response()
}

async fn duplicate_deck(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(deck_id): Path<i64>,
) -> impl IntoResponse {
    let profile = match required_profile_from_headers(&state, &headers) {
        Ok(profile) => profile,
        Err(response) => return response,
    };
    let deck = {
        let mut store = state
            .store
            .lock()
            .expect("store lock should not be poisoned");
        let mut decks = DeckLibrary::new(store.connection_mut());
        match decks.duplicate_for_user(profile.id, deck_id) {
            Ok(Some(deck)) => deck,
            Ok(None) => return deck_not_found_response(),
            Err(error) => return deck_error_response(error),
        }
    };
    Json(deck).into_response()
}

async fn delete_deck(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(deck_id): Path<i64>,
) -> impl IntoResponse {
    let profile = match required_profile_from_headers(&state, &headers) {
        Ok(profile) => profile,
        Err(response) => return response,
    };
    let deleted = {
        let mut store = state
            .store
            .lock()
            .expect("store lock should not be poisoned");
        let mut decks = DeckLibrary::new(store.connection_mut());
        match decks.delete_for_user(profile.id, deck_id) {
            Ok(deleted) => deleted,
            Err(error) => return deck_error_response(error),
        }
    };
    if !deleted {
        return deck_not_found_response();
    }
    Json(AuthMessageResponse {
        message: "Deck deleted",
    })
    .into_response()
}

async fn register_account(
    State(state): State<SharedState>,
    Json(request): Json<AuthRequest>,
) -> impl IntoResponse {
    let Some((email, normalized_email)) = identity::normalized_email(&request.email) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                message: "Enter a valid email address.".to_string(),
            }),
        )
            .into_response();
    };
    if request.password.len() < 8 && !normalized_email.ends_with("@local.dev") {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                message: "Password must be at least 8 characters.".to_string(),
            }),
        )
            .into_response();
    }

    let session = {
        let mut store = state
            .store
            .lock()
            .expect("store lock should not be poisoned");
        let mut identity = IdentityModule::new(store.connection_mut());
        match identity.register(&email, &normalized_email, &request.password) {
            Ok(Some(session)) => session,
            Ok(None) => {
                return (
                    StatusCode::CONFLICT,
                    Json(ApiError {
                        message: "That email already has an account.".to_string(),
                    }),
                )
                    .into_response();
            }
            Err(error) => return identity_error_response(error),
        }
    };

    Json(AuthSessionResponse::from(session)).into_response()
}

async fn login_account(
    State(state): State<SharedState>,
    Json(request): Json<AuthRequest>,
) -> impl IntoResponse {
    let Some((_, normalized_email)) = identity::normalized_email(&request.email) else {
        return invalid_credentials_response();
    };

    let session = {
        let mut store = state
            .store
            .lock()
            .expect("store lock should not be poisoned");
        let mut identity = IdentityModule::new(store.connection_mut());
        match identity.login(&normalized_email, &request.password) {
            Ok(Some(session)) => session,
            Ok(None) => return invalid_credentials_response(),
            Err(error) => return identity_error_response(error),
        }
    };

    Json(AuthSessionResponse::from(session)).into_response()
}

async fn current_account(
    State(state): State<SharedState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    match required_profile_from_headers(&state, &headers) {
        Ok(user) => Json(AuthUserResponse::from(user)).into_response(),
        Err(response) => response,
    }
}

async fn load_profile(State(state): State<SharedState>, headers: HeaderMap) -> impl IntoResponse {
    match required_profile_from_headers(&state, &headers) {
        Ok(profile) => Json(AuthUserResponse::from(profile)).into_response(),
        Err(response) => response,
    }
}

async fn update_profile(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Json(request): Json<UpdateProfileRequest>,
) -> impl IntoResponse {
    let profile = match required_profile_from_headers(&state, &headers) {
        Ok(profile) => profile,
        Err(response) => return response,
    };
    let display_name = request.display_name.trim();
    if display_name.is_empty() || display_name.len() > 32 {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                message: "Display name must be between 1 and 32 characters.".to_string(),
            }),
        )
            .into_response();
    }
    if !generated_avatar_is_valid(&request.avatar) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                message: "Choose a valid generated avatar.".to_string(),
            }),
        )
            .into_response();
    }

    let updated = {
        let mut store = state
            .store
            .lock()
            .expect("store lock should not be poisoned");
        let mut identity = IdentityModule::new(store.connection_mut());
        match identity.update_profile(
            profile.id,
            &request.handle,
            display_name,
            GeneratedAvatar {
                symbol: request.avatar.symbol,
                color: request.avatar.color,
            },
            request
                .preferred_hero_type
                .unwrap_or(profile.preferred_hero_type),
            request
                .board_visual_mode
                .unwrap_or(profile.board_visual_mode),
        ) {
            Ok(Some(profile)) => profile,
            Ok(None) => return unauthorized_response(),
            Err(error) => return identity_error_response(error),
        }
    };

    Json(AuthUserResponse::from(updated)).into_response()
}

async fn update_preferred_hero(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Json(request): Json<UpdatePreferredHeroRequest>,
) -> impl IntoResponse {
    let profile = match required_profile_from_headers(&state, &headers) {
        Ok(profile) => profile,
        Err(response) => return response,
    };
    let updated = {
        let mut store = state
            .store
            .lock()
            .expect("store lock should not be poisoned");
        let mut identity = IdentityModule::new(store.connection_mut());
        match identity.update_preferred_hero(profile.id, request.hero_type) {
            Ok(Some(profile)) => profile,
            Ok(None) => return unauthorized_response(),
            Err(error) => return identity_error_response(error),
        }
    };

    Json(AuthUserResponse::from(updated)).into_response()
}

async fn list_profile_matches(
    State(state): State<SharedState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let profile = match required_profile_from_headers(&state, &headers) {
        Ok(profile) => profile,
        Err(response) => return response,
    };
    let matches = {
        let store = state
            .store
            .lock()
            .expect("store lock should not be poisoned");
        match MatchAccess::new(&store).list_profile_matches(profile.id) {
            Ok(matches) => {
                let mut summaries = Vec::with_capacity(matches.len());
                for summary in matches {
                    let context = match store.viewer_context_for_user(&summary.id, profile.id) {
                        Ok(context) => context,
                        Err(error) => return store_error_response(error),
                    };
                    let (team, deck_name) = context.unwrap_or((
                        crate::match_session::Team::Player,
                        summary.player_deck_name.clone(),
                    ));
                    summaries.push(MatchSummary::for_viewer(summary, team, deck_name));
                }
                summaries
            }
            Err(error) => return store_error_response(error),
        }
    };

    Json(MatchArchiveResponse { matches }).into_response()
}

async fn load_preferences(
    State(state): State<SharedState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let profile = match required_profile_from_headers(&state, &headers) {
        Ok(profile) => profile,
        Err(response) => return response,
    };
    let preferences = {
        let mut store = state
            .store
            .lock()
            .expect("store lock should not be poisoned");
        let preferences = PreferencesModule::new(store.connection_mut());
        match preferences.load_for_user(profile.id) {
            Ok(preferences) => preferences,
            Err(error) => return preferences_error_response(error),
        }
    };
    Json(preferences).into_response()
}

async fn update_preferences(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Json(request): Json<UpdatePreferencesRequest>,
) -> impl IntoResponse {
    let profile = match required_profile_from_headers(&state, &headers) {
        Ok(profile) => profile,
        Err(response) => return response,
    };
    let preferences = {
        let mut store = state
            .store
            .lock()
            .expect("store lock should not be poisoned");
        let mut preferences = PreferencesModule::new(store.connection_mut());
        match preferences.update_for_user(profile.id, request) {
            Ok(preferences) => preferences,
            Err(error) => return preferences_error_response(error),
        }
    };
    Json(preferences).into_response()
}

async fn load_progression(
    State(state): State<SharedState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let profile = match required_profile_from_headers(&state, &headers) {
        Ok(profile) => profile,
        Err(response) => return response,
    };
    let response = {
        let mut store = state
            .store
            .lock()
            .expect("store lock should not be poisoned");
        let mut progression = ProgressionModule::new(store.connection_mut());
        match progression.load_for_user(profile.id) {
            Ok(response) => response,
            Err(error) => return progression_error_response(error),
        }
    };
    Json(response).into_response()
}

async fn unlock_progression_skill(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path((hero_type, node_id)): Path<(HeroType, String)>,
) -> impl IntoResponse {
    mutate_progression(&state, &headers, |progression, user_id| {
        progression.unlock_skill(user_id, hero_type, &node_id)
    })
}

async fn respec_progression_hero(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(hero_type): Path<HeroType>,
) -> impl IntoResponse {
    mutate_progression(&state, &headers, |progression, user_id| {
        progression.respec_hero(user_id, hero_type)
    })
}

async fn save_progression_loadout(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(hero_type): Path<HeroType>,
    Json(request): Json<SaveRuneLoadoutRequest>,
) -> impl IntoResponse {
    mutate_progression(&state, &headers, |progression, user_id| {
        progression.save_rune_loadout(user_id, hero_type, request)
    })
}

fn mutate_progression(
    state: &SharedState,
    headers: &HeaderMap,
    mutate: impl FnOnce(
        &mut ProgressionModule<'_>,
        i64,
    ) -> Result<ProgressionResponse, ProgressionError>,
) -> axum::response::Response {
    let profile = match required_profile_from_headers(state, headers) {
        Ok(profile) => profile,
        Err(response) => return response,
    };
    let response = {
        let mut store = state
            .store
            .lock()
            .expect("store lock should not be poisoned");
        let mut progression = ProgressionModule::new(store.connection_mut());
        match mutate(&mut progression, profile.id) {
            Ok(response) => response,
            Err(error) => return progression_error_response(error),
        }
    };
    Json(response).into_response()
}

async fn logout_account(State(state): State<SharedState>, headers: HeaderMap) -> impl IntoResponse {
    let Some(token) = bearer_token_from_headers(&headers) else {
        return Json(AuthMessageResponse {
            message: "Signed out",
        })
        .into_response();
    };

    {
        let mut store = state
            .store
            .lock()
            .expect("store lock should not be poisoned");
        let mut identity = IdentityModule::new(store.connection_mut());
        if let Err(error) = identity.logout(&token) {
            return identity_error_response(error);
        }
    }

    Json(AuthMessageResponse {
        message: "Signed out",
    })
    .into_response()
}

async fn list_matches(State(state): State<SharedState>, headers: HeaderMap) -> impl IntoResponse {
    let profile = match required_profile_from_headers(&state, &headers) {
        Ok(profile) => profile,
        Err(response) => return response,
    };
    let matches = {
        let store = state
            .store
            .lock()
            .expect("store lock should not be poisoned");
        match store.list_replayable_matches_for_user(profile.id) {
            Ok(matches) => {
                let mut summaries = Vec::with_capacity(matches.len());
                for summary in matches {
                    let context = match store.viewer_context_for_user(&summary.id, profile.id) {
                        Ok(context) => context,
                        Err(error) => return store_error_response(error),
                    };
                    let (team, deck_name) = context.unwrap_or((
                        crate::match_session::Team::Player,
                        summary.player_deck_name.clone(),
                    ));
                    summaries.push(MatchSummary::for_viewer(summary, team, deck_name));
                }
                summaries
            }
            Err(error) => return store_error_response(error),
        }
    };

    Json(MatchArchiveResponse { matches }).into_response()
}

async fn create_match(
    State(state): State<SharedState>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let profile = match optional_profile_from_headers(&state, &headers) {
        Ok(profile) => profile,
        Err(response) => return response,
    };
    let request = if body.is_empty() || body.iter().all(|byte| byte.is_ascii_whitespace()) {
        CreateMatchRequest {
            hero_type: None,
            player_deck: None,
            player_deck_id: None,
            ai_opponent: None,
            rune_ids: None,
            format: None,
        }
    } else {
        match serde_json::from_slice::<CreateMatchRequest>(&body) {
            Ok(request) => request,
            Err(error) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiError {
                        message: format!("Invalid match request: {error}"),
                    }),
                )
                    .into_response();
            }
        }
    };

    let created = {
        let mut store = state
            .store
            .lock()
            .expect("store lock should not be poisoned");
        let loadouts = match resolve_solo_match_loadouts(
            &mut store,
            SoloMatchLoadoutRequest {
                user_id: profile.as_ref().map(|profile| profile.id),
                preferred_hero_type: profile
                    .as_ref()
                    .map(|profile| profile.preferred_hero_type)
                    .unwrap_or_default(),
                requested_hero_type: request.hero_type,
                player_deck: request.player_deck,
                legacy_player_deck_id: request.player_deck_id,
                ai_opponent: request.ai_opponent,
                requested_rune_ids: request.rune_ids,
            },
        ) {
            Ok(loadouts) => loadouts,
            Err(error) => return loadout_resolution_error_response(error),
        };
        let result = store.create_match_for_user_with_decks(
            loadouts.player.hero_type,
            loadouts.opponent.hero_type,
            loadouts.player.cards,
            loadouts.opponent.cards,
            loadouts.player.progression,
            loadouts.opponent.progression,
            loadouts.player.deck_recipe.name,
            profile.as_ref().map(|profile| profile.id),
        );
        match result {
            Ok(created) => created,
            Err(error) => return store_error_response(error),
        }
    };

    Json(MatchResponse::from(created)).into_response()
}

#[cfg(debug_assertions)]
async fn list_match_scenarios() -> impl IntoResponse {
    let scenarios = crate::match_session::scenarios::match_scenarios()
        .into_iter()
        .map(MatchScenarioSummary::from)
        .collect();

    Json(MatchScenarioListResponse { scenarios }).into_response()
}

#[cfg(debug_assertions)]
async fn create_match_scenario(
    State(state): State<SharedState>,
    Path(scenario_id): Path<String>,
) -> impl IntoResponse {
    let Some(match_state) = crate::match_session::scenarios::build_match_scenario(&scenario_id)
    else {
        return (
            StatusCode::NOT_FOUND,
            Json(ApiError {
                message: format!("Match scenario {scenario_id} was not found"),
            }),
        )
            .into_response();
    };

    let created = {
        let mut store = state
            .store
            .lock()
            .expect("store lock should not be poisoned");
        match store.create_scenario_match(&scenario_id, match_state) {
            Ok(created) => created,
            Err(error) => return store_error_response(error),
        }
    };

    Json(MatchResponse::from(created)).into_response()
}

async fn create_shared_match(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Json(request): Json<CreateMatchRequest>,
) -> impl IntoResponse {
    let profile = match optional_profile_from_headers(&state, &headers) {
        Ok(profile) => profile,
        Err(response) => return response,
    };
    let format = match request.format {
        Some(SharedMatchFormatRequest::TwoVTwo) => SharedMatchFormat::TwoVTwo,
        Some(SharedMatchFormatRequest::Duel) | None => SharedMatchFormat::Duel,
    };
    match SharedMatchLifecycle::new(state)
        .create(profile.as_ref().map(|profile| profile.id), format)
    {
        Ok(created) => Json(CreateSharedMatchResponse::from(created)).into_response(),
        Err(error) => store_error_response(error),
    }
}

async fn load_shared_match(
    State(state): State<SharedState>,
    Path((match_id, seat_token)): Path<(String, String)>,
) -> impl IntoResponse {
    match SharedMatchLifecycle::new(state).open_seat(&match_id, &seat_token) {
        Ok(shared) => Json(SharedMatchResponse::from(shared)).into_response(),
        Err(error) => shared_lifecycle_error_response(error),
    }
}

async fn join_shared_match(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path((match_id, seat_token)): Path<(String, String)>,
    Json(request): Json<JoinSharedMatchRequest>,
) -> impl IntoResponse {
    let profile = match optional_profile_from_headers(&state, &headers) {
        Ok(profile) => profile,
        Err(response) => return response,
    };
    match SharedMatchLifecycle::new(state).join(SharedSeatJoin {
        match_id: &match_id,
        seat_token: &seat_token,
        user_id: profile.as_ref().map(|profile| profile.id),
        hero_type: request.hero_type,
        deck_choice: request.deck_choice,
        legacy_deck_id: request.deck_recipe_id,
        requested_rune_ids: request.rune_ids,
    }) {
        Ok(shared) => Json(SharedMatchResponse::from(shared)).into_response(),
        Err(error) => shared_lifecycle_error_response(error),
    }
}

fn shared_lifecycle_error_response(error: SharedLifecycleError) -> axum::response::Response {
    match error {
        SharedLifecycleError::NotFound => shared_not_found_response(),
        SharedLifecycleError::Store(error) => store_error_response(error),
        SharedLifecycleError::Loadout(error) => loadout_resolution_error_response(error),
    }
}

async fn shared_match_ws(
    ws: WebSocketUpgrade,
    State(state): State<SharedState>,
    Path((match_id, seat_token)): Path<(String, String)>,
) -> impl IntoResponse {
    match SharedMatchLifecycle::new(state.clone()).seat_exists(&match_id, &seat_token) {
        Ok(true) => {}
        Ok(false) => return shared_not_found_response(),
        Err(error) => return store_error_response(error),
    }

    ws.on_upgrade(move |socket| handle_shared_socket(socket, state, match_id, seat_token))
        .into_response()
}

/// Thin WebSocket adapter over `SharedMatchLifecycle`: it moves messages between the socket and
/// the seat connection, which decides replies, notifications and when to close.
async fn handle_shared_socket(
    mut socket: WebSocket,
    state: SharedState,
    match_id: String,
    seat_token: String,
) {
    let connection = match SharedMatchLifecycle::new(state.clone()).connect(&match_id, &seat_token)
    {
        Ok(connection) => connection,
        Err(error) => {
            let _ = send_shared_message(
                &mut socket,
                SharedServerMessage::Error {
                    message: format!("Could not establish shared match session: {error}"),
                },
            )
            .await;
            return;
        }
    };

    let mut receiver = state.match_sender(connection.match_id()).subscribe();
    let mut checks = time::interval(SHARED_SEAT_CHECK_INTERVAL);
    let mut last_client_message = Instant::now();
    if let Some(message) = connection.snapshot()
        && send_shared_message(&mut socket, message).await.is_err()
    {
        connection.finish();
        return;
    }
    connection.announce();

    loop {
        tokio::select! {
            received = socket.next() => {
                let Some(Ok(message)) = received else {
                    break;
                };
                if !connection.is_current() {
                    break;
                }
                last_client_message = Instant::now();
                match message {
                    Message::Text(text) => {
                        let outcome = connection.handle_text(&text, unix_timestamp());
                        if !outcome.keep_open {
                            break;
                        }
                        if let Some(reply) = outcome.reply {
                            let _ = send_shared_message(&mut socket, reply).await;
                        }
                        if outcome.notify_others {
                            connection.announce();
                        }
                    }
                    Message::Close(_) => break,
                    _ => {}
                }
            }
            _ = checks.tick() => {
                if connection.should_close(last_client_message.elapsed()) {
                    break;
                }
            }
            broadcast = receiver.recv() => {
                if broadcast.is_err() || !connection.is_current() {
                    break;
                }
                if let Some(message) = connection.snapshot()
                    && send_shared_message(&mut socket, message).await.is_err()
                {
                    break;
                }
            }
        }
    }

    connection.finish();
}

async fn send_shared_message(
    socket: &mut WebSocket,
    message: SharedServerMessage,
) -> Result<(), axum::Error> {
    let text = serde_json::to_string(&message).expect("server message should serialize");
    socket.send(Message::Text(text.into())).await
}

async fn load_match(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(match_id): Path<String>,
) -> impl IntoResponse {
    let profile = match optional_profile_from_headers(&state, &headers) {
        Ok(profile) => profile,
        Err(response) => return response,
    };
    let loaded = {
        let store = state
            .store
            .lock()
            .expect("store lock should not be poisoned");
        let loaded = match store.load_match(&match_id) {
            Ok(Some(loaded)) => loaded,
            Ok(None) => {
                return match_not_found_response(&match_id);
            }
            Err(error) => return store_error_response(error),
        };
        let actor = Actor::from(profile.as_ref());
        if !MatchAccess::new(&store).can_load_match(&actor, &match_id) {
            return match_not_found_response(&match_id);
        }
        loaded
    };

    Json(MatchResponse::from(loaded)).into_response()
}

async fn load_replay(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(match_id): Path<String>,
) -> impl IntoResponse {
    let profile = match optional_profile_from_headers(&state, &headers) {
        Ok(profile) => profile,
        Err(response) => return response,
    };
    let actor = Actor::from(profile.as_ref());
    replay_read_response(&state, &actor, &match_id)
}

async fn load_match_summary(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(match_id): Path<String>,
) -> impl IntoResponse {
    let profile = match optional_profile_from_headers(&state, &headers) {
        Ok(profile) => profile,
        Err(response) => return response,
    };
    let actor = Actor::from(profile.as_ref());
    match_summary_read_response(&state, &actor, &match_id)
}

async fn load_shared_match_summary(
    State(state): State<SharedState>,
    Path((match_id, seat_token)): Path<(String, String)>,
) -> impl IntoResponse {
    let actor = Actor::Seat {
        match_id: match_id.clone(),
        seat_token,
    };
    match_summary_read_response(&state, &actor, &match_id)
}

async fn load_shared_replay(
    State(state): State<SharedState>,
    Path((match_id, seat_token)): Path<(String, String)>,
) -> impl IntoResponse {
    let actor = Actor::Seat {
        match_id: match_id.clone(),
        seat_token,
    };
    replay_read_response(&state, &actor, &match_id)
}

fn replay_read_response(
    state: &SharedState,
    actor: &Actor,
    match_id: &str,
) -> axum::response::Response {
    let read = {
        let mut store = state
            .store
            .lock()
            .expect("store lock should not be poisoned");
        ReplayReads::new(&mut store).replay(actor, match_id)
    };
    let ReplayRead { replay, visibility } = match read {
        Ok(read) => read,
        Err(ReplayReadError::NotFound) => return replay_not_found_response(match_id),
        Err(ReplayReadError::Store(error)) => return store_error_response(error),
        Err(ReplayReadError::Progression(error)) => return progression_error_response(error),
    };
    let match_id = replay.summary.id.clone();
    let summary = MatchSummary::for_viewer(
        replay.summary.clone(),
        crate::match_session::Team::Player,
        replay.summary.player_deck_name.clone(),
    );
    let frames = replay
        .frames
        .into_iter()
        .map(|frame| ReplayFrameResponse::from_stored(frame, visibility))
        .collect();

    Json(MatchReplayResponse {
        match_id,
        visibility,
        summary,
        frames,
    })
    .into_response()
}

fn match_summary_read_response(
    state: &SharedState,
    actor: &Actor,
    match_id: &str,
) -> axum::response::Response {
    let read = {
        let mut store = state
            .store
            .lock()
            .expect("store lock should not be poisoned");
        ReplayReads::new(&mut store).summary(actor, match_id)
    };
    let MatchSummaryRead {
        summary,
        viewer_side,
        viewer_deck_name,
        reward,
    } = match read {
        Ok(read) => read,
        Err(ReplayReadError::NotFound) => return match_summary_not_found_response(match_id),
        Err(ReplayReadError::Store(error)) => return store_error_response(error),
        Err(ReplayReadError::Progression(error)) => return progression_error_response(error),
    };
    let viewer = MatchSummaryViewer {
        side: viewer_side,
        result: viewer_result(viewer_side, summary.state.winner),
    };
    let match_id = summary.id.clone();
    Json(MatchSummaryResponse {
        match_id,
        summary: MatchSummary::for_viewer(
            summary,
            viewer_side
                .map(|side| side.team())
                .unwrap_or(crate::match_session::Team::Player),
            viewer_deck_name,
        ),
        viewer,
        reward,
    })
    .into_response()
}

fn viewer_result(viewer_side: Option<Side>, winner: Option<Side>) -> ViewerResult {
    match (viewer_side, winner) {
        (Some(viewer_side), Some(winner)) if viewer_side.team() == winner.team() => {
            ViewerResult::Victory
        }
        (Some(_), Some(_)) => ViewerResult::Defeat,
        _ => ViewerResult::Spectator,
    }
}

async fn apply_match_action(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(match_id): Path<String>,
    Json(request): Json<MatchActionRequest>,
) -> impl IntoResponse {
    let profile = match optional_profile_from_headers(&state, &headers) {
        Ok(profile) => profile,
        Err(response) => return response,
    };
    let applied = {
        let mut store = state
            .store
            .lock()
            .expect("store lock should not be poisoned");
        let actor = Actor::from(profile.as_ref());
        match MatchCommands::new(&mut store).apply_solo_action(actor, &match_id, request) {
            Ok(applied) => applied,
            Err(error) => return match_command_error_response(error, &match_id),
        }
    };

    Json(MatchResponse::from_stored_with_replay_frames(
        applied.stored_match,
        applied.replay_frames,
    ))
    .into_response()
}

fn generated_avatar_is_valid(avatar: &GeneratedAvatarRequest) -> bool {
    const SYMBOLS: [&str; 6] = ["sparkles", "shield", "sword", "wand", "rune", "flame"];
    const COLORS: [&str; 6] = ["emerald", "indigo", "rose", "amber", "sky", "slate"];
    SYMBOLS.contains(&avatar.symbol.as_str()) && COLORS.contains(&avatar.color.as_str())
}
