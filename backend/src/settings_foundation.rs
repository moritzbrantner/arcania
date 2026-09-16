use settings_core::{
    ApplyMode, SettingDefinition, SettingId, SettingKind, SettingScope, SettingValue,
    SettingsRegistry,
};

use crate::identity::BoardVisualMode;
use crate::preferences::{
    AnimationSpeed, BoardScale, MotionPreference, PreferenceTheme, PreferencesError,
    UpdatePreferencesRequest,
};

const THEME_ID: &str = "appearance.theme";
const MOTION_ID: &str = "accessibility.motion";
const ANIMATION_SPEED_ID: &str = "appearance.animationSpeed";
const BOARD_SCALE_ID: &str = "appearance.boardScale";
const BOARD_VISUAL_MODE_ID: &str = "appearance.boardVisualMode";

pub(crate) fn validate_preferences(
    request: &UpdatePreferencesRequest,
) -> Result<(), PreferencesError> {
    let registry = settings_registry();
    validate_choice(&registry, THEME_ID, theme_value(&request.theme))?;
    validate_choice(&registry, MOTION_ID, motion_value(&request.motion))?;
    validate_choice(
        &registry,
        ANIMATION_SPEED_ID,
        animation_speed_value(&request.animation_speed),
    )?;
    validate_choice(&registry, BOARD_SCALE_ID, board_scale_value(&request.board_scale))?;
    validate_choice(
        &registry,
        BOARD_VISUAL_MODE_ID,
        board_visual_mode_value(request.board_visual_mode),
    )?;
    Ok(())
}

pub(crate) fn settings_registry() -> SettingsRegistry {
    let mut registry = SettingsRegistry::new();
    for definition in [
        choice_setting(
            THEME_ID,
            "system",
            &["system", "dark", "light", "highContrast"],
        ),
        choice_setting(MOTION_ID, "system", &["system", "reduced", "full"]),
        choice_setting(
            ANIMATION_SPEED_ID,
            "normal",
            &["slow", "normal", "fast"],
        ),
        choice_setting(
            BOARD_SCALE_ID,
            "normal",
            &["compact", "normal", "large"],
        ),
        choice_setting(BOARD_VISUAL_MODE_ID, "3d", &["2d", "3d"]),
    ] {
        registry
            .register(definition)
            .expect("Rune Lanes settings definitions must remain valid and unique");
    }
    registry
}

fn choice_setting(id: &str, default: &str, options: &[&str]) -> SettingDefinition {
    SettingDefinition {
        id: SettingId::new(id).expect("static Rune Lanes setting ids must be valid"),
        kind: SettingKind::Choice {
            options: options.iter().map(|value| (*value).to_owned()).collect(),
        },
        default: SettingValue::Choice(default.to_owned()),
        scope: SettingScope::User,
        apply_mode: ApplyMode::Apply,
        availability: None,
    }
}

fn validate_choice(
    registry: &SettingsRegistry,
    id: &str,
    value: &str,
) -> Result<(), PreferencesError> {
    let id = SettingId::new(id).map_err(|error| {
        PreferencesError::Validation(format!("Invalid Rune Lanes setting id: {error}"))
    })?;
    let definition = registry.get(&id).ok_or_else(|| {
        PreferencesError::Validation(format!("Unknown Rune Lanes setting: {id}"))
    })?;
    definition
        .validate_value(&SettingValue::Choice(value.to_owned()))
        .map_err(|error| PreferencesError::Validation(format!("Invalid setting {id}: {error}")))
}

fn theme_value(theme: &PreferenceTheme) -> &'static str {
    match theme {
        PreferenceTheme::System => "system",
        PreferenceTheme::Dark => "dark",
        PreferenceTheme::Light => "light",
        PreferenceTheme::HighContrast => "highContrast",
    }
}

fn motion_value(motion: &MotionPreference) -> &'static str {
    match motion {
        MotionPreference::System => "system",
        MotionPreference::Reduced => "reduced",
        MotionPreference::Full => "full",
    }
}

fn animation_speed_value(speed: &AnimationSpeed) -> &'static str {
    match speed {
        AnimationSpeed::Slow => "slow",
        AnimationSpeed::Normal => "normal",
        AnimationSpeed::Fast => "fast",
    }
}

fn board_scale_value(scale: &BoardScale) -> &'static str {
    match scale {
        BoardScale::Compact => "compact",
        BoardScale::Normal => "normal",
        BoardScale::Large => "large",
    }
}

fn board_visual_mode_value(mode: BoardVisualMode) -> &'static str {
    match mode {
        BoardVisualMode::TwoD => "2d",
        BoardVisualMode::ThreeD => "3d",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_keeps_generic_settings_separate_from_input_bindings() {
        let registry = settings_registry();
        let definitions = registry
            .iter()
            .map(|(id, definition)| {
                (
                    id.as_str().to_owned(),
                    definition.scope,
                    definition.apply_mode,
                )
            })
            .collect::<Vec<_>>();

        assert_eq!(definitions.len(), 5);
        assert!(definitions.iter().all(|(id, scope, apply_mode)| {
            !id.contains("hotkey")
                && *scope == SettingScope::User
                && *apply_mode == ApplyMode::Apply
        }));
    }

    #[test]
    fn existing_preference_values_conform_to_shared_settings_definitions() {
        let request = UpdatePreferencesRequest {
            theme: PreferenceTheme::HighContrast,
            motion: MotionPreference::Reduced,
            animation_speed: AnimationSpeed::Fast,
            board_scale: BoardScale::Large,
            board_visual_mode: BoardVisualMode::TwoD,
            hotkeys: Vec::new(),
        };

        validate_preferences(&request).expect("existing preference values should be valid");
    }
}
