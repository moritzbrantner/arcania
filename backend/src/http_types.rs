use crate::deck_library;
use crate::identity::{AccountProfile, BoardVisualMode, CreatedAuthSession, PublicAccountProfile};
use crate::match_session::{
    self, HeroType, MatchActionRequest, MatchMode, MatchState, RecordedReplayFrame, ReplayEvent,
    ReplayVisibility, Side,
};
use crate::match_store::{
    CreatedSharedMatch, SharedMatchStatus, StoredMatch, StoredMatchSummary, StoredReplayFrame,
    StoredSharedMatch,
};
use crate::progression::{self, MatchRewardSummary, ProgressionSummary};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub(crate) struct ApiError {
    pub(crate) message: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AuthRequest {
    pub(crate) email: String,
    pub(crate) password: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AuthUserResponse {
    pub(crate) id: i64,
    pub(crate) handle: String,
    pub(crate) email: String,
    pub(crate) display_name: String,
    pub(crate) avatar: GeneratedAvatarResponse,
    pub(crate) preferred_hero_type: HeroType,
    pub(crate) board_visual_mode: BoardVisualMode,
    pub(crate) progression_summary: ProgressionSummary,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AuthSessionResponse {
    pub(crate) token: String,
    pub(crate) user: AuthUserResponse,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AuthMessageResponse {
    pub(crate) message: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GeneratedAvatarResponse {
    pub(crate) symbol: String,
    pub(crate) color: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PublicDeckOwnerResponse {
    pub(crate) id: i64,
    pub(crate) handle: String,
    pub(crate) display_name: String,
    pub(crate) avatar: GeneratedAvatarResponse,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PublicDeckRecipeResponse {
    pub(crate) owner: PublicDeckOwnerResponse,
    pub(crate) deck: deck_library::DeckRecipeSummary,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateProfileRequest {
    pub(crate) display_name: String,
    pub(crate) handle: String,
    pub(crate) avatar: GeneratedAvatarRequest,
    #[serde(default, alias = "preferredWizardType")]
    pub(crate) preferred_hero_type: Option<HeroType>,
    #[serde(default)]
    pub(crate) board_visual_mode: Option<BoardVisualMode>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdatePreferredHeroRequest {
    pub(crate) hero_type: HeroType,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GeneratedAvatarRequest {
    pub(crate) symbol: String,
    pub(crate) color: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateMatchRequest {
    #[serde(default, alias = "wizardType")]
    pub(crate) hero_type: Option<HeroType>,
    #[serde(default)]
    pub(crate) player_deck: Option<DeckChoiceRequest>,
    #[serde(default)]
    pub(crate) player_deck_id: Option<i64>,
    #[serde(default)]
    pub(crate) ai_opponent: Option<AiOpponentRequest>,
    #[serde(default)]
    pub(crate) rune_ids: Option<Vec<String>>,
    #[serde(default)]
    pub(crate) format: Option<SharedMatchFormatRequest>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum SharedMatchFormatRequest {
    Duel,
    TwoVTwo,
}

#[derive(Deserialize)]
#[serde(
    tag = "source",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(crate) enum AiOpponentRequest {
    System {
        system_deck_id: String,
    },
    Account {
        deck_id: i64,
        #[serde(alias = "wizardType")]
        hero_type: HeroType,
    },
}

#[derive(Deserialize)]
#[serde(
    tag = "source",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(crate) enum DeckChoiceRequest {
    Starter,
    System { system_deck_id: String },
    Account { deck_id: i64 },
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct JoinSharedMatchRequest {
    #[serde(alias = "wizardType")]
    pub(crate) hero_type: HeroType,
    #[serde(default)]
    pub(crate) deck_choice: Option<DeckChoiceRequest>,
    #[serde(default)]
    pub(crate) deck_recipe_id: Option<i64>,
    #[serde(default)]
    pub(crate) rune_ids: Option<Vec<String>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MatchResponse {
    pub(crate) match_id: String,
    pub(crate) match_state: MatchState,
    pub(crate) hero_appearances: Vec<HeroAppearanceAssignment>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) replay_frames: Vec<ReplayFrameResponse>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MatchArchiveResponse {
    pub(crate) matches: Vec<MatchSummary>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MatchSummaryResponse {
    pub(crate) match_id: String,
    pub(crate) summary: MatchSummary,
    pub(crate) viewer: MatchSummaryViewer,
    pub(crate) reward: Option<MatchRewardSummary>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MatchSummaryViewer {
    pub(crate) side: Option<Side>,
    pub(crate) result: ViewerResult,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ViewerResult {
    Victory,
    Defeat,
    Spectator,
}

#[cfg(debug_assertions)]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MatchScenarioListResponse {
    pub(crate) scenarios: Vec<MatchScenarioSummary>,
}

#[cfg(debug_assertions)]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MatchScenarioSummary {
    pub(crate) id: &'static str,
    pub(crate) name: &'static str,
    pub(crate) description: &'static str,
    pub(crate) primary_actions: &'static [&'static str],
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MatchSummary {
    pub(crate) match_id: String,
    pub(crate) mode: MatchMode,
    pub(crate) created_at: i64,
    pub(crate) updated_at: i64,
    pub(crate) round: u32,
    pub(crate) phase: match_session::Phase,
    pub(crate) winner: Option<match_session::Side>,
    pub(crate) frame_count: usize,
    pub(crate) viewer_team: match_session::Team,
    pub(crate) viewer_hero_types: Vec<HeroType>,
    pub(crate) opposing_hero_types: Vec<HeroType>,
    pub(crate) viewer_deck_name: Option<String>,
    pub(crate) viewer_result: Option<ViewerResult>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MatchReplayResponse {
    pub(crate) match_id: String,
    pub(crate) visibility: ReplayVisibility,
    pub(crate) summary: MatchSummary,
    pub(crate) frames: Vec<ReplayFrameResponse>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReplayFrameResponse {
    pub(crate) frame_index: u32,
    pub(crate) action_index: Option<u32>,
    pub(crate) event: ReplayEvent,
    pub(crate) match_state: serde_json::Value,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateSharedMatchResponse {
    pub(crate) match_id: String,
    pub(crate) mode: &'static str,
    pub(crate) format: &'static str,
    pub(crate) status: &'static str,
    pub(crate) viewer_side: Side,
    pub(crate) player_seat_url: String,
    pub(crate) invite_seat_url: String,
    pub(crate) seat_urls: Vec<SharedSeatUrlResponse>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SharedSeatUrlResponse {
    pub(crate) side: Side,
    pub(crate) team: crate::match_session::Team,
    pub(crate) label: &'static str,
    pub(crate) url: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SharedMatchResponse {
    pub(crate) match_id: String,
    pub(crate) mode: &'static str,
    pub(crate) format: &'static str,
    pub(crate) status: &'static str,
    pub(crate) viewer_side: Side,
    pub(crate) viewer_team: crate::match_session::Team,
    pub(crate) viewer_hero_type: Option<HeroType>,
    pub(crate) opponent_hero_type: Option<HeroType>,
    pub(crate) viewer_ready: bool,
    pub(crate) opponent_ready: bool,
    pub(crate) seats: Vec<SharedSeatResponse>,
    pub(crate) active_side: Option<Side>,
    pub(crate) opponent_connected: bool,
    pub(crate) can_claim_forfeit_at: Option<i64>,
    pub(crate) hero_appearances: Vec<HeroAppearanceAssignment>,
    pub(crate) match_state: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HeroAppearanceAssignment {
    pub(crate) side: Side,
    pub(crate) hero_type: HeroType,
    pub(crate) appearance_id: String,
    pub(crate) source: HeroAppearanceAssignmentSource,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code, reason = "viewer-local fallback is emitted by the frontend")]
pub(crate) enum HeroAppearanceAssignmentSource {
    OwnerSelection,
    ViewerSelection,
    Base,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SharedSeatResponse {
    pub(crate) side: Side,
    pub(crate) team: crate::match_session::Team,
    pub(crate) label: &'static str,
    pub(crate) ready: bool,
    pub(crate) connected: bool,
    pub(crate) hero_type: Option<HeroType>,
    pub(crate) knocked_out: bool,
}

#[derive(Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(crate) enum SharedClientMessage {
    Action {
        request_id: String,
        action: MatchActionRequest,
    },
    ClaimForfeit {
        request_id: String,
    },
    Heartbeat,
}

#[derive(Serialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(crate) enum SharedServerMessage {
    Snapshot {
        payload: SharedMatchResponse,
    },
    ActionAccepted {
        request_id: String,
        payload: SharedMatchResponse,
    },
    ActionRejected {
        request_id: String,
        message: String,
    },
    PresenceChanged {
        payload: SharedMatchResponse,
    },
    Error {
        message: String,
    },
}

#[allow(
    clippy::result_large_err,
    reason = "route helpers return Axum responses directly"
)]
impl From<AccountProfile> for AuthUserResponse {
    fn from(profile: AccountProfile) -> Self {
        Self {
            id: profile.id,
            handle: profile.public_handle,
            email: profile.email,
            display_name: profile.display_name,
            avatar: GeneratedAvatarResponse {
                symbol: profile.avatar.symbol,
                color: profile.avatar.color,
            },
            preferred_hero_type: profile.preferred_hero_type,
            board_visual_mode: profile.board_visual_mode,
            progression_summary: progression::summary_for_xp(profile.total_xp),
        }
    }
}

impl From<PublicAccountProfile> for PublicDeckOwnerResponse {
    fn from(profile: PublicAccountProfile) -> Self {
        Self {
            id: profile.id,
            handle: profile.public_handle,
            display_name: profile.display_name,
            avatar: GeneratedAvatarResponse {
                symbol: profile.avatar.symbol,
                color: profile.avatar.color,
            },
        }
    }
}

impl From<CreatedAuthSession> for AuthSessionResponse {
    fn from(session: CreatedAuthSession) -> Self {
        Self {
            token: session.token,
            user: AuthUserResponse::from(session.profile),
        }
    }
}

impl From<StoredMatch> for MatchResponse {
    fn from(stored_match: StoredMatch) -> Self {
        Self {
            match_id: stored_match.id,
            match_state: stored_match.state,
            hero_appearances: Vec::new(),
            replay_frames: Vec::new(),
        }
    }
}

#[cfg(debug_assertions)]
impl From<match_session::scenarios::MatchScenarioDefinition> for MatchScenarioSummary {
    fn from(scenario: match_session::scenarios::MatchScenarioDefinition) -> Self {
        Self {
            id: scenario.id,
            name: scenario.name,
            description: scenario.description,
            primary_actions: scenario.primary_actions,
        }
    }
}

impl MatchResponse {
    pub(crate) fn from_stored_with_replay_frames(
        stored_match: StoredMatch,
        replay_frames: Vec<RecordedReplayFrame>,
    ) -> Self {
        Self {
            match_id: stored_match.id,
            match_state: stored_match.state,
            hero_appearances: Vec::new(),
            replay_frames: replay_frames
                .iter()
                .enumerate()
                .map(|(index, frame)| ReplayFrameResponse::from_recorded(index as u32, frame))
                .collect(),
        }
    }
}

impl From<CreatedSharedMatch> for CreateSharedMatchResponse {
    fn from(created: CreatedSharedMatch) -> Self {
        let mut seat_urls = vec![
            SharedSeatUrlResponse {
                side: Side::Player,
                team: Side::Player.team(),
                label: "Player 1",
                url: format!("/match/{}/{}", created.match_id, created.player_token),
            },
            SharedSeatUrlResponse {
                side: Side::Opponent,
                team: Side::Opponent.team(),
                label: "Opponent 1",
                url: format!("/match/{}/{}", created.match_id, created.opponent_token),
            },
        ];
        if let Some(token) = &created.player_two_token {
            seat_urls.push(SharedSeatUrlResponse {
                side: Side::PlayerTwo,
                team: Side::PlayerTwo.team(),
                label: "Player 2",
                url: format!("/match/{}/{}", created.match_id, token),
            });
        }
        if let Some(token) = &created.opponent_two_token {
            seat_urls.push(SharedSeatUrlResponse {
                side: Side::OpponentTwo,
                team: Side::OpponentTwo.team(),
                label: "Opponent 2",
                url: format!("/match/{}/{}", created.match_id, token),
            });
        }
        Self {
            player_seat_url: format!("/match/{}/{}", created.match_id, created.player_token),
            invite_seat_url: format!("/match/{}/{}", created.match_id, created.opponent_token),
            match_id: created.match_id,
            mode: "shared",
            format: created.format.as_str(),
            status: "setup",
            viewer_side: Side::Player,
            seat_urls,
        }
    }
}

impl From<StoredSharedMatch> for SharedMatchResponse {
    fn from(shared: StoredSharedMatch) -> Self {
        let active_side = shared.state.as_ref().and_then(|state| {
            if state.phase == match_session::Phase::MatchOver {
                None
            } else {
                Some(state.active_side)
            }
        });
        let match_state = shared
            .state
            .as_ref()
            .map(|state| state.queries().public_match(shared.viewer_seat.side));
        let opponent_connected = shared
            .seats
            .iter()
            .filter(|seat| seat.side.team() != shared.viewer_seat.side.team())
            .any(|seat| seat.joined_at.is_some() && seat.disconnected_at.is_none());
        let can_claim_forfeit_at = if shared.status == SharedMatchStatus::Active {
            let opposing_disconnect_times: Vec<_> = shared
                .seats
                .iter()
                .filter(|seat| seat.side.team() != shared.viewer_seat.side.team())
                .map(|seat| seat.disconnected_at)
                .collect();
            if opposing_disconnect_times.iter().all(Option::is_some) {
                opposing_disconnect_times
                    .into_iter()
                    .flatten()
                    .map(|disconnected_at| disconnected_at + 120)
                    .max()
            } else {
                None
            }
        } else {
            None
        };

        Self {
            match_id: shared.match_id,
            mode: "shared",
            format: shared.format.as_str(),
            status: shared.status.as_str(),
            viewer_side: shared.viewer_seat.side,
            viewer_team: shared.viewer_seat.side.team(),
            viewer_hero_type: shared.viewer_seat.hero_type,
            opponent_hero_type: shared.opposing_seat.hero_type,
            viewer_ready: shared.viewer_seat.joined_at.is_some(),
            opponent_ready: shared.opposing_seat.joined_at.is_some(),
            seats: shared
                .seats
                .iter()
                .map(|seat| SharedSeatResponse {
                    side: seat.side,
                    team: seat.side.team(),
                    label: seat.side.label_for_response(),
                    ready: seat.joined_at.is_some(),
                    connected: seat.joined_at.is_some() && seat.disconnected_at.is_none(),
                    hero_type: seat.hero_type,
                    knocked_out: shared
                        .state
                        .as_ref()
                        .and_then(|state| state.participant_for_public(seat.side))
                        .is_some_and(|participant| participant.knocked_out),
                })
                .collect(),
            active_side,
            opponent_connected,
            can_claim_forfeit_at,
            hero_appearances: Vec::new(),
            match_state,
        }
    }
}

impl MatchSummary {
    pub(crate) fn for_viewer(
        summary: StoredMatchSummary,
        viewer_team: match_session::Team,
        viewer_deck_name: Option<String>,
    ) -> Self {
        let (viewer_hero_types, opposing_hero_types) =
            hero_types_for_teams(&summary.state, viewer_team);
        let viewer_result = summary.state.winner.map(|winner| {
            if winner.team() == viewer_team {
                ViewerResult::Victory
            } else {
                ViewerResult::Defeat
            }
        });
        Self {
            match_id: summary.id,
            mode: summary.state.mode,
            created_at: summary.created_at,
            updated_at: summary.updated_at,
            round: summary.state.round,
            phase: summary.state.phase,
            winner: summary.state.winner,
            frame_count: summary.frame_count,
            viewer_team,
            viewer_hero_types,
            opposing_hero_types,
            viewer_deck_name,
            viewer_result,
        }
    }
}

fn hero_types_for_teams(
    state: &MatchState,
    viewer_team: match_session::Team,
) -> (Vec<HeroType>, Vec<HeroType>) {
    let mut viewer = Vec::new();
    let mut opposing = Vec::new();
    for player in [
        Some(&state.player),
        Some(&state.opponent),
        state.player_two.as_ref(),
        state.opponent_two.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        if player.side.team() == viewer_team {
            viewer.push(player.hero.hero_type);
        } else {
            opposing.push(player.hero.hero_type);
        }
    }
    (viewer, opposing)
}

impl ReplayFrameResponse {
    pub(crate) fn from_recorded(frame_index: u32, frame: &RecordedReplayFrame) -> Self {
        Self {
            frame_index,
            action_index: frame.action_index,
            event: frame.event.for_visibility(ReplayVisibility::Public),
            match_state: MatchState::from_snapshot_json(&frame.snapshot_json)
                .expect("recorded replay frame snapshot should deserialize")
                .replay_value(ReplayVisibility::Public),
        }
    }

    pub(crate) fn from_stored(frame: StoredReplayFrame, visibility: ReplayVisibility) -> Self {
        Self {
            frame_index: frame.frame_index,
            action_index: frame.action_index,
            event: frame.event.for_visibility(visibility),
            match_state: frame.state.replay_value(visibility),
        }
    }
}
