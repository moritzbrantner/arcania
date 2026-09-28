// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  attachHotkeyRuntime,
  hotkeyActionRegistry,
  hotkeyCommandForDispatch,
  type HotkeyHandlers,
} from "./hotkeyRuntime";
import type { HotkeyBinding } from "./types";

const detachers: Array<() => void> = [];

afterEach(() => {
  while (detachers.length > 0) {
    detachers.pop()?.();
  }
});

describe("hotkey action registry", () => {
  it("keeps Rune Lanes actions consumer-owned while using shared binding semantics", () => {
    const registry = hotkeyActionRegistry(
      [
        { commandId: "cursorNorthwest", binding: "Z" },
        { commandId: "openSettings", binding: "," },
      ],
      { cursorNorthwest: () => true, openSettings: () => true },
    );

    expect(registry.actions.map((action) => action.id)).toEqual([
      "runeLanes.cursorNorthwest",
      "runeLanes.openSettings",
    ]);
    expect(registry.actions[0]?.repeatPolicy).toBe("allow");
    expect(registry.actions[1]?.repeatPolicy).toBe("never");
    expect(registry.actions[0]?.defaults?.[0]?.sequence[0]).toEqual({
      key: { kind: "logical", value: "z" },
      modifiers: {},
    });
  });

  it("registers only actions the current surface handles", () => {
    const registry = hotkeyActionRegistry([], { openSettings: () => true });

    expect(registry.actions.map((action) => action.id)).toEqual(["runeLanes.openSettings"]);
  });

  it("executes press and repeat dispatches but not releases", () => {
    expect(hotkeyCommandForDispatch({ action: "runeLanes.cursorEast", phase: "press" })).toBe(
      "cursorEast",
    );
    expect(hotkeyCommandForDispatch({ action: "runeLanes.cursorEast", phase: "repeat" })).toBe(
      "cursorEast",
    );
    expect(hotkeyCommandForDispatch({ action: "runeLanes.cursorEast", phase: "release" })).toBeNull();
    expect(hotkeyCommandForDispatch({ action: "other.cursorEast", phase: "press" })).toBeNull();
  });
});

describe("hotkey runtime", () => {
  it("matches saved hotkeys and falls back to defaults for missing commands", () => {
    const openCatalog = vi.fn(() => true);
    const openMatchArchive = vi.fn(() => true);
    attach([{ commandId: "openCatalog", binding: "G" }], { openCatalog, openMatchArchive });

    press("g");
    press("m");
    press("c");

    expect(openCatalog).toHaveBeenCalledOnce();
    expect(openMatchArchive).toHaveBeenCalledOnce();
  });

  it("prevents the browser default only when a handler handled the command", () => {
    attach([], { endTurn: () => false, openCatalog: () => true });

    expect(press("t").defaultPrevented).toBe(false);
    expect(press("c").defaultPrevented).toBe(true);
    expect(press("x").defaultPrevented).toBe(false);
  });

  it("consumes a handled key regardless of the order keydown listeners fire in", () => {
    const keyTarget = new ReverseOrderEventTarget();
    detachers.push(
      attachHotkeyRuntime([], { endTurn: () => false, openCatalog: () => true }, { keyTarget }),
    );

    expect(keyTarget.press("c").defaultPrevented).toBe(true);
    expect(keyTarget.press("x").defaultPrevented).toBe(false);
    expect(keyTarget.press("t").defaultPrevented).toBe(false);
  });

  it("ignores key events from editable controls", () => {
    const openCatalog = vi.fn(() => true);
    attach([], { openCatalog });

    for (const tagName of ["input", "select", "textarea"]) {
      const element = document.createElement(tagName);
      document.body.append(element);
      press("c", {}, element);
      element.remove();
    }

    expect(openCatalog).not.toHaveBeenCalled();
  });

  it("preserves browser modifier shortcuts", () => {
    const openCatalog = vi.fn(() => true);
    attach([], { openCatalog });

    press("c", { ctrlKey: true });
    press("c", { metaKey: true });
    press("c", { altKey: true });

    expect(openCatalog).not.toHaveBeenCalled();
  });

  it("stops dispatching after detach", () => {
    const openCatalog = vi.fn(() => true);
    attach([], { openCatalog });
    detachers.pop()?.();

    press("c");

    expect(openCatalog).not.toHaveBeenCalled();
  });
});

function attach(hotkeys: HotkeyBinding[], handlers: HotkeyHandlers) {
  detachers.push(attachHotkeyRuntime(hotkeys, handlers));
}

function press(key: string, init: KeyboardEventInit = {}, target: EventTarget = window) {
  const event = new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true, ...init });
  target.dispatchEvent(event);
  window.dispatchEvent(new KeyboardEvent("keyup", { key, bubbles: true, cancelable: true, ...init }));
  return event;
}

// Invokes listeners in reverse registration order, as a browser does not promise any
// particular order between listeners registered by different modules or targets.
class ReverseOrderEventTarget {
  private readonly listeners = new Map<string, Array<(event: Event) => void>>();

  addEventListener(type: string, listener: (event: Event) => void) {
    this.listeners.set(type, [...(this.listeners.get(type) ?? []), listener]);
  }

  removeEventListener(type: string, listener: (event: Event) => void) {
    this.listeners.set(
      type,
      (this.listeners.get(type) ?? []).filter((registered) => registered !== listener),
    );
  }

  press(key: string) {
    const event = new KeyboardEvent("keydown", { key, cancelable: true });
    for (const listener of [...(this.listeners.get("keydown") ?? [])].reverse()) {
      listener(event);
    }
    const release = new KeyboardEvent("keyup", { key, cancelable: true });
    for (const listener of [...(this.listeners.get("keyup") ?? [])].reverse()) {
      listener(release);
    }
    return event;
  }
}
