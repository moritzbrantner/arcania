use crate::card_workshop::CardWorkshopError;
use crate::deck_library::DeckLibraryError;
use crate::identity::IdentityError;
use crate::loadout_resolution::LoadoutResolutionError;
use crate::match_commands::MatchCommandError;
use crate::match_store::MatchStoreError;
use crate::preferences::PreferencesError;
use crate::progression::ProgressionError;
use crate::*;
use axum::Json;
use axum::http::StatusCode;
use axum::response::IntoResponse;

pub(crate) fn store_error_response(error: MatchStoreError) -> axum::response::Response {
    let status = match error {
        MatchStoreError::VersionConflict { .. } => StatusCode::CONFLICT,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (
        status,
        Json(ApiError {
            message: error.to_string(),
        }),
    )
        .into_response()
}

pub(crate) fn match_command_error_response(
    error: MatchCommandError,
    match_id: &str,
) -> axum::response::Response {
    match error {
        MatchCommandError::NotFound | MatchCommandError::Forbidden => {
            match_not_found_response(match_id)
        }
        MatchCommandError::Rule(error) => (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                message: error.to_string(),
            }),
        )
            .into_response(),
        MatchCommandError::Store(error) => store_error_response(error),
        error => (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                message: error.to_string(),
            }),
        )
            .into_response(),
    }
}

pub(crate) fn card_workshop_error_response(error: CardWorkshopError) -> axum::response::Response {
    let status = match error {
        CardWorkshopError::NotFound => StatusCode::NOT_FOUND,
        CardWorkshopError::VersionConflict { .. }
        | CardWorkshopError::CardIdConflict(_)
        | CardWorkshopError::PublishedCardIdImmutable { .. } => StatusCode::CONFLICT,
        CardWorkshopError::InvalidDefinition { .. }
        | CardWorkshopError::InvalidPublishedRevision(_) => StatusCode::BAD_REQUEST,
        CardWorkshopError::Sqlite(_) | CardWorkshopError::Snapshot(_) => {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    };
    (
        status,
        Json(ApiError {
            message: error.to_string(),
        }),
    )
        .into_response()
}

pub(crate) fn deck_error_response(error: DeckLibraryError) -> axum::response::Response {
    let status = match error {
        DeckLibraryError::UnknownTemplate(_)
        | DeckLibraryError::NegativeCount(_)
        | DeckLibraryError::EmptyName
        | DeckLibraryError::NameTooLong
        | DeckLibraryError::TooManyDecks
        | DeckLibraryError::IllegalRecipe(_)
        | DeckLibraryError::UnknownSystemDeck(_) => StatusCode::BAD_REQUEST,
        DeckLibraryError::NotFound => StatusCode::NOT_FOUND,
        DeckLibraryError::Sqlite(_) | DeckLibraryError::Snapshot(_) => {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    };
    (
        status,
        Json(ApiError {
            message: error.to_string(),
        }),
    )
        .into_response()
}

pub(crate) fn preferences_error_response(error: PreferencesError) -> axum::response::Response {
    match error {
        PreferencesError::Validation(message) => {
            (StatusCode::BAD_REQUEST, Json(ApiError { message })).into_response()
        }
        other => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                message: other.to_string(),
            }),
        )
            .into_response(),
    }
}

pub(crate) fn progression_error_response(error: ProgressionError) -> axum::response::Response {
    let status = match error {
        ProgressionError::Sqlite(_) | ProgressionError::Snapshot(_) => {
            StatusCode::INTERNAL_SERVER_ERROR
        }
        ProgressionError::UnknownRune(_)
        | ProgressionError::LockedRune(_)
        | ProgressionError::DuplicateRune(_)
        | ProgressionError::TooManyRunes { .. }
        | ProgressionError::UnknownSkill(_)
        | ProgressionError::SkillAlreadyUnlocked(_)
        | ProgressionError::SkillPrerequisiteMissing(_)
        | ProgressionError::NotEnoughSkillPoints
        | ProgressionError::UnknownHeroAppearance(_)
        | ProgressionError::LockedHeroAppearance(_)
        | ProgressionError::MismatchedHeroAppearance { .. } => StatusCode::BAD_REQUEST,
    };
    (
        status,
        Json(ApiError {
            message: error.to_string(),
        }),
    )
        .into_response()
}

pub(crate) fn loadout_resolution_error_response(
    error: LoadoutResolutionError,
) -> axum::response::Response {
    match error {
        LoadoutResolutionError::UnauthorizedAccountDeck => unauthorized_response(),
        LoadoutResolutionError::DeckNotFound => deck_not_found_response(),
        LoadoutResolutionError::SharedSeatNotFound => shared_not_found_response(),
        LoadoutResolutionError::Deck(error) => deck_error_response(error),
        LoadoutResolutionError::Progression(error) => progression_error_response(error),
        LoadoutResolutionError::Store(error) => store_error_response(error),
    }
}

pub(crate) fn identity_error_response(error: IdentityError) -> axum::response::Response {
    let status = match error {
        IdentityError::Validation(_) => StatusCode::BAD_REQUEST,
        IdentityError::Sqlite(_) | IdentityError::PasswordHash(_) => {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    };
    (
        status,
        Json(ApiError {
            message: error.to_string(),
        }),
    )
        .into_response()
}

pub(crate) fn invalid_credentials_response() -> axum::response::Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(ApiError {
            message: "Email or password is incorrect.".to_string(),
        }),
    )
        .into_response()
}

pub(crate) fn unauthorized_response() -> axum::response::Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(ApiError {
            message: "Sign in to continue.".to_string(),
        }),
    )
        .into_response()
}

pub(crate) fn match_not_found_response(match_id: &str) -> axum::response::Response {
    (
        StatusCode::NOT_FOUND,
        Json(ApiError {
            message: format!("Match {match_id} was not found"),
        }),
    )
        .into_response()
}

pub(crate) fn replay_not_found_response(match_id: &str) -> axum::response::Response {
    (
        StatusCode::NOT_FOUND,
        Json(ApiError {
            message: format!("Replay for match {match_id} was not found"),
        }),
    )
        .into_response()
}

pub(crate) fn match_summary_not_found_response(match_id: &str) -> axum::response::Response {
    (
        StatusCode::NOT_FOUND,
        Json(ApiError {
            message: format!("Match summary for match {match_id} was not found"),
        }),
    )
        .into_response()
}

pub(crate) fn deck_not_found_response() -> axum::response::Response {
    (
        StatusCode::NOT_FOUND,
        Json(ApiError {
            message: "Deck recipe was not found".to_string(),
        }),
    )
        .into_response()
}

pub(crate) fn shared_not_found_response() -> axum::response::Response {
    (
        StatusCode::NOT_FOUND,
        Json(ApiError {
            message: "Shared match seat was not found".to_string(),
        }),
    )
        .into_response()
}
