import { House, Keyboard, RefreshCcw, RotateCcw, Save } from "lucide-react";
import { useEffect, useLayoutEffect, useMemo, useState } from "react";
import {
  defaultHotkeyMap,
  HOTKEY_COMMANDS,
  hotkeyBindingsToMap,
  hotkeyValidationIssuesByCommand,
  keyEventToHotkeyBinding,
  normalizedHotkeySaveBindings,
  validateHotkeyMap,
} from "./hotkeys";
import { accountPreferencesSavePayload } from "./preferences";
import type {
  AccountPreferences,
  AnimationSpeed,
  AuthUser,
  BoardScale,
  BoardVisualMode,
  HotkeyCommandId,
  MotionPreference,
  PreferenceTheme,
} from "./types";
import { AccountActions } from "./components/common";
import type { KeyboardEvent as ReactKeyboardEvent } from "react";

type SettingsState =
  | { status: "loading"; preferences: AccountPreferences }
  | { status: "ready"; preferences: AccountPreferences }
  | { status: "error"; preferences: AccountPreferences; message: string };

type SettingsPageProps = {
  preferencesState: SettingsState;
  currentUser: AuthUser | null;
  onNavigate: (to: string) => void;
  onSave: (preferences: ReturnType<typeof accountPreferencesSavePayload>) => Promise<void>;
  onRefresh: () => Promise<void>;
  onSignOut: () => void;
};

const THEME_OPTIONS: Array<[PreferenceTheme, string]> = [
  ["system", "System"],
  ["dark", "Dark"],
  ["light", "Light"],
  ["highContrast", "High contrast"],
];

const MOTION_OPTIONS: Array<[MotionPreference, string]> = [
  ["system", "System"],
  ["reduced", "Reduced"],
  ["full", "Full"],
];

const SPEED_OPTIONS: Array<[AnimationSpeed, string]> = [
  ["slow", "Slow"],
  ["normal", "Normal"],
  ["fast", "Fast"],
];

const SCALE_OPTIONS: Array<[BoardScale, string]> = [
  ["compact", "Compact"],
  ["normal", "Normal"],
  ["large", "Large"],
];

const BOARD_VISUAL_MODE_OPTIONS: Array<[BoardVisualMode, string]> = [
  ["2d", "2D"],
  ["3d", "3D"],
];

export function SettingsPage({
  preferencesState,
  currentUser,
  onNavigate,
  onSave,
  onRefresh,
  onSignOut,
}: SettingsPageProps) {
  const isSignedIn = Boolean(currentUser);
  const preferences = preferencesState.preferences;
  const [theme, setTheme] = useState<PreferenceTheme>(preferences.theme);
  const [motion, setMotion] = useState<MotionPreference>(preferences.motion);
  const [animationSpeed, setAnimationSpeed] = useState<AnimationSpeed>(
    preferences.animationSpeed,
  );
  const [boardScale, setBoardScale] = useState<BoardScale>(preferences.boardScale);
  const [boardVisualMode, setBoardVisualMode] = useState<BoardVisualMode>(
    preferences.boardVisualMode,
  );
  const [hotkeys, setHotkeys] = useState(() => hotkeyBindingsToMap(preferences.hotkeys));
  const [recordingCommandId, setRecordingCommandId] = useState<HotkeyCommandId | null>(null);
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const hotkeyIssues = useMemo(() => validateHotkeyMap(hotkeys), [hotkeys]);
  const hotkeyErrors = useMemo(
    () => hotkeyValidationIssuesByCommand(hotkeyIssues),
    [hotkeyIssues],
  );
  const controlsDisabled = busy || preferencesState.status === "loading";

  useLayoutEffect(() => {
    setTheme(preferences.theme);
    setMotion(preferences.motion);
    setAnimationSpeed(preferences.animationSpeed);
    setBoardScale(preferences.boardScale);
    setBoardVisualMode(preferences.boardVisualMode);
    setHotkeys(hotkeyBindingsToMap(preferences.hotkeys));
    setRecordingCommandId(null);
  }, [preferences]);

  async function handleSave() {
    if (hotkeyIssues.length > 0) {
      setNotice("Resolve hotkey conflicts before saving.");
      return;
    }

    setBusy(true);
    setNotice(null);
    try {
      await onSave(
        accountPreferencesSavePayload(preferences, {
          theme,
          motion,
          animationSpeed,
          boardScale,
          boardVisualMode,
        },
        normalizedHotkeySaveBindings(hotkeys),
      ),
      );
      setNotice("Settings saved.");
    } catch (error) {
      setNotice(error instanceof Error ? error.message : "Could not save settings");
    } finally {
      setBusy(false);
    }
  }

  function handleHotkeyKeyDown(commandId: HotkeyCommandId, event: ReactKeyboardEvent) {
    if (recordingCommandId !== commandId) {
      return;
    }

    event.preventDefault();
    event.stopPropagation();

    const binding = keyEventToHotkeyBinding(event.nativeEvent);
    if (!binding) {
      setHotkeys((current) => ({ ...current, [commandId]: "" }));
      setRecordingCommandId(null);
      return;
    }

    setHotkeys((current) => ({ ...current, [commandId]: binding }));
    setRecordingCommandId(null);
    setNotice(null);
  }

  function resetHotkeysToDefaults() {
    setHotkeys(defaultHotkeyMap());
    setRecordingCommandId(null);
    setNotice(null);
  }

  async function handleRefresh() {
    setBusy(true);
    setNotice(null);
    try {
      await onRefresh();
      setNotice("Settings refreshed.");
    } catch (error) {
      setNotice(error instanceof Error ? error.message : "Could not refresh settings");
    } finally {
      setBusy(false);
    }
  }

  return (
    <main className="app-shell archive-shell">
      <section className="settings-layout" aria-label="Settings">
        <header className="top-bar">
          <div>
            <p className="eyebrow">{isSignedIn ? "Account preferences" : "Local preferences"}</p>
            <h1>Settings</h1>
            <button className="secondary-link" type="button" onClick={() => onNavigate("/workshop?tab=rules")}>Edit match rules</button>
            <button className="secondary-link" type="button" onClick={() => onNavigate("/workshop?tab=cards")}>Open card editor</button>
          </div>
          <div className="actions">
            <button className="icon-button" type="button" onClick={() => onNavigate("/")} title="Dashboard">
              <House size={18} />
            </button>
            {currentUser ? (
              <AccountActions
                currentUser={currentUser}
                onNavigate={onNavigate}
                onSignOut={onSignOut}
                activeAccountRoute="settings"
              />
            ) : null}
          </div>
        </header>

        <section className="settings-panel" aria-label="Visual preferences">
          <div className="section-heading">
            <h2>Visual Preferences</h2>
          </div>
          {preferencesState.status === "loading" ? (
            <p className="empty-state">Loading settings.</p>
          ) : null}
          {preferencesState.status === "error" ? (
            <p className="notice">{preferencesState.message}</p>
          ) : null}
          {isSignedIn ? (
            <>
              <PreferenceSelect
                id="settings-theme"
                label="Theme"
                value={theme}
                options={THEME_OPTIONS}
                onChange={(value) => setTheme(value as PreferenceTheme)}
                disabled={controlsDisabled}
              />
              <PreferenceSelect
                id="settings-motion"
                label="Motion"
                value={motion}
                options={MOTION_OPTIONS}
                onChange={(value) => setMotion(value as MotionPreference)}
                disabled={controlsDisabled}
              />
              <PreferenceSelect
                id="settings-animation-speed"
                label="Animation speed"
                value={animationSpeed}
                options={SPEED_OPTIONS}
                onChange={(value) => setAnimationSpeed(value as AnimationSpeed)}
                disabled={controlsDisabled}
              />
              <PreferenceSelect
                id="settings-board-scale"
                label="Board scale"
                value={boardScale}
                options={SCALE_OPTIONS}
                onChange={(value) => setBoardScale(value as BoardScale)}
                disabled={controlsDisabled}
              />
            </>
          ) : null}
          <PreferenceSelect
            id="settings-board-visual-mode"
            label="Board visual mode"
            value={boardVisualMode}
            options={BOARD_VISUAL_MODE_OPTIONS}
            onChange={(value) => setBoardVisualMode(value as BoardVisualMode)}
            disabled={controlsDisabled}
          />
        </section>

        {isSignedIn ? (
          <section className="settings-panel" aria-label="Hotkey preferences">
          <div className="section-heading">
            <h2>Hotkeys</h2>
            <button
              className="secondary-link"
              type="button"
              onClick={resetHotkeysToDefaults}
              disabled={controlsDisabled}
            >
              <RotateCcw size={18} />
              Reset Defaults
            </button>
          </div>
          <div className="hotkey-list">
            {HOTKEY_COMMANDS.map((command) => {
              const error = hotkeyErrors[command.id];
              const isRecording = recordingCommandId === command.id;

              return (
                <div
                  className={`hotkey-row${error ? " hotkey-row-error" : ""}`}
                  key={command.id}
                >
                  <div className="hotkey-label">
                    <span>{command.label}</span>
                    <small>Default {formatBinding(command.defaultBinding)}</small>
                  </div>
                  <button
                    className="hotkey-recorder"
                    type="button"
                    aria-label={`${command.label} hotkey`}
                    aria-invalid={error ? "true" : undefined}
                    aria-describedby={error ? `${command.id}-hotkey-error` : undefined}
                    onClick={() => {
                      setRecordingCommandId(command.id);
                      setNotice(null);
                    }}
                    onKeyDown={(event) => handleHotkeyKeyDown(command.id, event)}
                    disabled={controlsDisabled}
                  >
                    <Keyboard size={18} />
                    {isRecording ? "Press key" : formatBinding(hotkeys[command.id])}
                  </button>
                  {error ? (
                    <p className="hotkey-error" id={`${command.id}-hotkey-error`}>
                      {error}
                    </p>
                  ) : null}
                </div>
              );
            })}
          </div>
          <div className="settings-actions">
            <button
              className="primary-button"
              type="button"
              onClick={() => void handleSave()}
              disabled={controlsDisabled || hotkeyIssues.length > 0}
            >
              <Save size={18} />
              Save Settings
            </button>
            <button className="secondary-link" type="button" onClick={() => void handleRefresh()} disabled={busy}>
              <RefreshCcw size={18} />
              Refresh
            </button>
          </div>
          {notice ? <p className="notice">{notice}</p> : null}
          </section>
        ) : (
          <section className="settings-panel" aria-label="Local preference actions">
            <div className="settings-actions">
              <button
                className="primary-button"
                type="button"
                onClick={() => void handleSave()}
                disabled={controlsDisabled}
              >
                <Save size={18} />
                Save Settings
              </button>
            </div>
            {notice ? <p className="notice">{notice}</p> : null}
          </section>
        )}
      </section>
    </main>
  );
}

function formatBinding(binding: string) {
  if (binding === " ") {
    return "Space";
  }
  if (!binding) {
    return "Unassigned";
  }
  return binding;
}

function PreferenceSelect({
  id,
  label,
  value,
  options,
  onChange,
  disabled,
}: {
  id: string;
  label: string;
  value: string;
  options: Array<[string, string]>;
  onChange: (value: string) => void;
  disabled: boolean;
}) {
  return (
    <label className="settings-control" htmlFor={id}>
      <span>{label}</span>
      <select
        id={id}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        disabled={disabled}
      >
        {options.map(([optionValue, optionLabel]) => (
          <option key={optionValue} value={optionValue}>
            {optionLabel}
          </option>
        ))}
      </select>
    </label>
  );
}

export type { SettingsState };
