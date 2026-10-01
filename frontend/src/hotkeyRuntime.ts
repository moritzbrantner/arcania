import type { ActionRegistry } from "@moritzbrantner/input-bindings";
import {
  InputRuntimeController,
  type RuntimeDispatch,
} from "@moritzbrantner/input-bindings-runtime";
import {
  attachKeyboardRuntime,
  normalizeLogicalKey,
  type BrowserRuntimeAdapterOptions,
  type RuntimeEventTargetLike,
} from "@moritzbrantner/input-bindings-web";
import { HOTKEY_COMMANDS, normalizeHotkeysWithDefaults } from "./hotkeys";
import type { HotkeyBinding, HotkeyCommandId } from "./types";

/**
 * Rune Lanes owns its semantic hotkey commands and the handlers that execute them.
 * A handler returns `true` when it actually handled the command; only then is the
 * browser default for that key prevented.
 */
export type HotkeyHandlers = Partial<Record<HotkeyCommandId, () => boolean>>;

export type HotkeyRuntimeTargets = Pick<
  BrowserRuntimeAdapterOptions,
  "keyTarget" | "focusTarget" | "visibilityTarget"
>;

const ACTION_PREFIX = "runeLanes.";
const RUNE_LANES_CONTEXT = "runeLanes";
const ACTIVE_CONTEXTS: ReadonlySet<string> = new Set([RUNE_LANES_CONTEXT]);

/**
 * Describes the current Rune Lanes hotkeys as an `input-bindings` action registry.
 * Only commands with a handler on the current surface are registered.
 */
export function hotkeyActionRegistry(
  hotkeys: HotkeyBinding[],
  handlers: HotkeyHandlers,
): ActionRegistry {
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
          source: "arcania/rune-lanes",
          version: "1",
        },
      };
    }),
  };
}

/** Maps a shared runtime dispatch back to the Rune Lanes command it should execute. */
export function hotkeyCommandForDispatch(
  dispatch: Pick<RuntimeDispatch, "action" | "phase">,
): HotkeyCommandId | null {
  return dispatch.phase === "release" ? null : commandIdForAction(dispatch.action);
}

/**
 * Attaches the shared `input-bindings` keyboard runtime for the given hotkeys and returns
 * a function that detaches it.
 */
export function attachHotkeyRuntime(
  hotkeys: HotkeyBinding[],
  handlers: HotkeyHandlers,
  targets: HotkeyRuntimeTargets = {},
): () => void {
  let handledCurrentKeydown = false;
  const controller = new InputRuntimeController({
    registry: hotkeyActionRegistry(hotkeys, handlers),
    getActiveContexts: () => ACTIVE_CONTEXTS,
    // The shared runtime decides consumption before a handler can report whether it
    // handled the command, so Rune Lanes consumes handled keys itself (see below).
    consumePolicy: "never",
    onDispatch: (dispatch) => {
      const commandId = hotkeyCommandForDispatch(dispatch);
      if (commandId && handlers[commandId]?.()) {
        handledCurrentKeydown = true;
      }
    },
  });

  return attachKeyboardRuntime(controller, {
    ...targets,
    keyTarget: consumingHandledKeydowns(targets.keyTarget ?? window, () => {
      const handled = handledCurrentKeydown;
      handledCurrentKeydown = false;
      return handled;
    }),
    ignoreTextEntry: true,
    mode: "logical",
    resetOnBlur: true,
    resetOnHidden: true,
    resetOnDetach: true,
  });
}

/**
 * Wraps the runtime's own `keydown` listener so that the key is consumed in the same
 * listener call that dispatched a handled command. A separate consuming listener would
 * depend on the order in which the two listeners fire.
 */
function consumingHandledKeydowns(
  target: RuntimeEventTargetLike,
  takeHandled: () => boolean,
): RuntimeEventTargetLike {
  type Listener = (event: KeyboardEvent) => void;
  const wrappers = new Map<Listener, Listener>();

  return {
    addEventListener(type, listener, options) {
      if (type !== "keydown") {
        target.addEventListener(type, listener, options);
        return;
      }
      const wrapped: Listener = (event) => {
        takeHandled();
        listener(event);
        if (takeHandled()) {
          event.preventDefault();
        }
      };
      wrappers.set(listener, wrapped);
      target.addEventListener(type, wrapped, options);
    },
    removeEventListener(type, listener, options) {
      const wrapped = type === "keydown" ? wrappers.get(listener) : undefined;
      if (wrapped) {
        wrappers.delete(listener);
      }
      target.removeEventListener(type, wrapped ?? listener, options);
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
