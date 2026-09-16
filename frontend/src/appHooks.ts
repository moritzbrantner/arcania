import { useCallback, useEffect, useState } from "react";
import { loadPreferences, updatePreferences } from "./api";
import { effectiveBoardVisualMode, saveLocalBoardVisualMode } from "./boardVisualMode";
import type { HotkeyHandlers } from "./hotkeyRuntime";
import {
  DEFAULT_ACCOUNT_PREFERENCES,
  normalizeAccountPreferences,
  resolveMotionPreference,
  visualPreferencesCssAttributes,
  visualPreferencesLiveAiDelayMs,
} from "./preferences";
import { attachSharedHotkeyRuntime } from "./sharedInputBindings";
import type { AccountPreferencesState, AppliedVisualPreferences } from "./appTypes";
import type { AccountPreferences, AuthUser } from "./types";

const MATCH_CHROME_STORAGE_KEY = "rune-lanes-match-chrome-minimized";

function readStoredMatchChromeMinimized() {
  if (typeof window === "undefined") {
    return false;
  }

  return window.localStorage.getItem(MATCH_CHROME_STORAGE_KEY) === "true";
}

export function useMatchChromeMinimized() {
  const [matchChromeMinimized, setMatchChromeMinimized] = useState(readStoredMatchChromeMinimized);

  useEffect(() => {
    window.localStorage.setItem(MATCH_CHROME_STORAGE_KEY, matchChromeMinimized ? "true" : "false");
  }, [matchChromeMinimized]);

  return [matchChromeMinimized, setMatchChromeMinimized] as const;
}

export function usePrefersReducedMotion() {
  const [prefersReducedMotion, setPrefersReducedMotion] = useState(() =>
    window.matchMedia("(prefers-reduced-motion: reduce)").matches,
  );

  useEffect(() => {
    const query = window.matchMedia("(prefers-reduced-motion: reduce)");
    const handleChange = () => setPrefersReducedMotion(query.matches);
    handleChange();
    query.addEventListener("change", handleChange);
    return () => query.removeEventListener("change", handleChange);
  }, []);

  return prefersReducedMotion;
}

export function usePrefersDarkTheme() {
  const [prefersDarkTheme, setPrefersDarkTheme] = useState(() =>
    window.matchMedia("(prefers-color-scheme: dark)").matches,
  );

  useEffect(() => {
    const query = window.matchMedia("(prefers-color-scheme: dark)");
    const handleChange = () => setPrefersDarkTheme(query.matches);
    handleChange();
    query.addEventListener("change", handleChange);
    return () => query.removeEventListener("change", handleChange);
  }, []);

  return prefersDarkTheme;
}

export function useAccountPreferences(currentUser: AuthUser | null) {
  const [accountPreferences, setAccountPreferences] = useState<AccountPreferencesState>({
    state: {
      status: "ready",
      preferences: localAccountPreferences(),
    },
    loadedUserId: null,
  });
  const currentUserId = currentUser?.id ?? null;

  const refresh = useCallback(async () => {
    if (!currentUser) {
      setAccountPreferences({
        state: { status: "ready", preferences: localAccountPreferences() },
        loadedUserId: null,
      });
      return;
    }

    setAccountPreferences((current) => ({
      ...current,
      state: { status: "loading", preferences: current.state.preferences },
    }));
    try {
      const loaded = normalizeAccountPreferences(await loadPreferences());
      setAccountPreferences({
        state: { status: "ready", preferences: loaded },
        loadedUserId: currentUser.id,
      });
    } catch (error) {
      setAccountPreferences((current) => ({
        ...current,
        state: {
          status: "error",
          preferences: current.state.preferences,
          message: error instanceof Error ? error.message : "Could not load settings",
        },
      }));
    }
  }, [currentUser?.id]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const save = useCallback(
    async (preferences: Parameters<typeof updatePreferences>[0]) => {
      if (currentUserId === null) {
        saveLocalBoardVisualMode(preferences.boardVisualMode);
        setAccountPreferences({
          state: {
            status: "ready",
            preferences: normalizeAccountPreferences({
              ...localAccountPreferences(),
              boardVisualMode: preferences.boardVisualMode,
            }),
          },
          loadedUserId: null,
        });
        return;
      }

      const updated = normalizeAccountPreferences(await updatePreferences(preferences));
      setAccountPreferences({
        state: { status: "ready", preferences: updated },
        loadedUserId: currentUserId,
      });
    },
    [currentUserId],
  );

  const state =
    currentUserId !== null && accountPreferences.loadedUserId !== currentUserId
      ? { status: "loading" as const, preferences: DEFAULT_ACCOUNT_PREFERENCES }
      : accountPreferences.state;

  return { state, refresh, save };
}

function localAccountPreferences(): AccountPreferences {
  return {
    ...DEFAULT_ACCOUNT_PREFERENCES,
    boardVisualMode: effectiveBoardVisualMode(null),
  };
}

export function useHotkeyHandlers(hotkeys: AccountPreferences["hotkeys"], handlers: HotkeyHandlers) {
  useEffect(() => {
    const runtime = attachSharedHotkeyRuntime(hotkeys, handlers);
    return () => runtime.destroy();
  }, [hotkeys, handlers]);
}

export function useAppliedVisualPreferences(preferences: AccountPreferences): AppliedVisualPreferences {
  const prefersDarkTheme = usePrefersDarkTheme();
  const prefersReducedMotion = usePrefersReducedMotion();
  const effectiveMotion = resolveMotionPreference(preferences.motion, prefersReducedMotion);

  useEffect(() => {
    const attributes = visualPreferencesCssAttributes(preferences, {
      prefersDarkTheme,
      prefersReducedMotion,
    });
    for (const [name, value] of Object.entries(attributes)) {
      document.documentElement.setAttribute(name, value);
    }
  }, [preferences, prefersDarkTheme, prefersReducedMotion]);

  return {
    preferences,
    effectiveMotion,
    liveAiDelayMs: visualPreferencesLiveAiDelayMs(
      preferences.animationSpeed,
      effectiveMotion,
    ),
  };
}
