import { describe, expect, it } from "vitest";
import type { HotkeyHandlers } from "./hotkeyRuntime";
import { sharedHotkeyRegistry } from "./sharedInputBindings";
import type { HotkeyBinding } from "./types";

const hotkeys: HotkeyBinding[] = [
  { commandId: "cursorNorthwest", binding: "Z" },
  { commandId: "openSettings", binding: "," },
];

describe("sharedHotkeyRegistry", () => {
  it("keeps Rune Lanes actions consumer-owned while using shared binding semantics", () => {
    const handlers: HotkeyHandlers = {
      cursorNorthwest: () => true,
      openSettings: () => true,
    };

    const registry = sharedHotkeyRegistry(hotkeys, handlers);

    expect(registry.actions.map((action) => action.id)).toEqual([
      "runeLanes.cursorNorthwest",
      "runeLanes.openSettings",
    ]);
    expect(registry.actions[0]?.repeatPolicy).toBe("allow");
    expect(registry.actions[1]?.repeatPolicy).toBe("never");
    expect(registry.actions[0]?.defaults[0]?.sequence[0]?.key).toEqual({
      kind: "logical",
      value: "z",
    });
  });

  it("registers only actions owned by the active consumer surface", () => {
    const registry = sharedHotkeyRegistry(hotkeys, {
      openSettings: () => true,
    });

    expect(registry.actions).toHaveLength(1);
    expect(registry.actions[0]?.id).toBe("runeLanes.openSettings");
  });
});
