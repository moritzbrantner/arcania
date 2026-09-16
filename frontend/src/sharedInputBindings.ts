import { HOTKEY_COMMANDS, normalizeHotkeysWithDefaults } from "./hotkeys";
import type { HotkeyHandlers } from "./hotkeyRuntime";
import type { HotkeyBinding, HotkeyCommandId } from "./types";

export const INPUT_BINDINGS_BUNDLE_URL =
  "https://moritzbrantner.github.io/input-bindings/input-bindings-browser.js";

const ACTION_PREFIX = "runeLanes.";

type SharedDispatch = {
  action: string;
  phase: string;
};

type RuntimeControllerOptions = {
  registry: ReturnType<typeof sharedHotkeyRegistry>;
  getActiveContexts: () => Set<string>;
  consumePolicy: "dispatched";
  onDispatch: (dispatch: SharedDispatch) => void;
};

type SharedInputBindingsModule = {
  InputRuntimeController: new (options: RuntimeControllerOptions) => object;
  attachKeyboardRuntime: (
    controller: object,
    options: {
      ignoreTextEntry: boolean;
      mode: "logical";
      resetOnBlur: boolean;
      resetOnHidden: boolean;
      resetOnDetach: boolean;
    },
  ) => () => void;
};

export function sharedHotkeyRegistry(hotkeys: HotkeyBinding[], handlers: HotkeyHandlers) {
  const effectiveBindings = new Map(
    normalizeHotkeysWithDefaults(hotkeys).map((binding) => [binding.commandId, binding.binding]),
  );

  return {
    actions: HOTKEY_COMMANDS.filter((command) => Boolean(handlers[command.id])).map((command) => {
      const action = actionId(command.id);
      return {
        id: action,
        title: command.label,
        categoryPath: ["Rune Lanes"],
        repeatPolicy: command.id.startsWith("cursor") ? "allow" : "never",
        allowedDevices: ["keyboard"],
        defaults: [
          {
            id: `${action}.effective`,
            action,
            sequence: [
              {
                key: {
                  kind: "logical",
                  value: normalizeLogicalKey(effectiveBindings.get(command.id) ?? command.defaultBinding),
                },
                modifiers: {},
              },
            ],
          },
        ],
        provenance: {
          source: "card-board-hybrid/rune-lanes",
          version: "1",
        },
      };
    }),
  };
}

export function attachSharedHotkeyRuntime(hotkeys: HotkeyBinding[], handlers: HotkeyHandlers) {
  let disposed = false;
  let detachRuntime = () => {};

  const ready = import(/* @vite-ignore */ INPUT_BINDINGS_BUNDLE_URL).then(
    (module) => {
      if (disposed) {
        return;
      }

      const shared = module as SharedInputBindingsModule;
      const controller = new shared.InputRuntimeController({
        registry: sharedHotkeyRegistry(hotkeys, handlers),
        getActiveContexts: () => new Set(["runeLanes"]),
        consumePolicy: "dispatched",
        onDispatch: (dispatch) => {
          if (dispatch.phase !== "press") {
            return;
          }

          const commandId = commandIdForAction(dispatch.action);
          if (commandId) {
            handlers[commandId]?.();
          }
        },
      });

      detachRuntime = shared.attachKeyboardRuntime(controller, {
        ignoreTextEntry: true,
        mode: "logical",
        resetOnBlur: true,
        resetOnHidden: true,
        resetOnDetach: true,
      });
    },
    (error) => {
      console.error("Failed to load shared input-bindings runtime", error);
    },
  );

  return {
    ready,
    destroy() {
      disposed = true;
      detachRuntime();
    },
  };
}

function actionId(commandId: HotkeyCommandId) {
  return `${ACTION_PREFIX}${commandId}`;
}

function commandIdForAction(action: string): HotkeyCommandId | null {
  if (!action.startsWith(ACTION_PREFIX)) {
    return null;
  }

  const commandId = action.slice(ACTION_PREFIX.length);
  return HOTKEY_COMMANDS.some((command) => command.id === commandId)
    ? (commandId as HotkeyCommandId)
    : null;
}

function normalizeLogicalKey(key: string) {
  if (key === " ") {
    return "Space";
  }
  if (key === "Esc") {
    return "Escape";
  }
  return key.length === 1 ? key.toLocaleLowerCase() : key;
}
