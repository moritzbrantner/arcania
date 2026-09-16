use settings_core::{
    ApplyMode, SettingDefinition, SettingId, SettingKind, SettingScope, SettingValue,
    SettingsRegistry,
};

const THEME_ID: &str = "appearance.theme";
const MOTION_ID: &str = "accessibility.motion";
const ANIMATION_SPEED_ID: &str = "appearance.animationSpeed";
const BOARD_SCALE_ID: &str = "appearance.boardScale";
const BOARD_VISUAL_MODE_ID: &str = "appearance.boardVisualMode";

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
    fn registry_preserves_current_preference_defaults() {
        let registry = settings_registry();
        let defaults = registry
            .iter()
            .map(|(id, definition)| (id.as_str(), &definition.default))
            .collect::<Vec<_>>();

        assert!(defaults.contains(&(THEME_ID, &SettingValue::Choice("system".to_owned()))));
        assert!(defaults.contains(&(MOTION_ID, &SettingValue::Choice("system".to_owned()))));
        assert!(defaults.contains(&(
            ANIMATION_SPEED_ID,
            &SettingValue::Choice("normal".to_owned()),
        )));
        assert!(defaults.contains(&(
            BOARD_SCALE_ID,
            &SettingValue::Choice("normal".to_owned()),
        )));
        assert!(defaults.contains(&(
            BOARD_VISUAL_MODE_ID,
            &SettingValue::Choice("3d".to_owned()),
        )));
    }
}
