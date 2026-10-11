import { describe, expect, it } from "vitest";
import { shortcutFor } from "../src/commands.ts";
import { presenterKey } from "../src/presenterEvents.ts";

const key = (key: string, extra: Partial<KeyboardEvent> = {}) =>
  ({ key, code: "", metaKey: false, ctrlKey: false, altKey: false, shiftKey: false, ...extra }) as KeyboardEvent;

describe("hurtigtaster", () => {
  it("Cmd eller Ctrl med tasten gir kommandoen", () => {
    expect(shortcutFor(key("n", { metaKey: true }))).toBe("new-note");
    expect(shortcutFor(key("M", { ctrlKey: true, shiftKey: true }))).toBe("create-presentation");
    expect(shortcutFor(key("m", { ctrlKey: true }))).toBeUndefined();
    expect(shortcutFor(key("n"))).toBeUndefined();
  });

  it("Cmd + Alt + Shift + P leses fra code, siden Alt endrer tegnet", () => {
    expect(shortcutFor(key("∏", { metaKey: true, altKey: true, shiftKey: true, code: "KeyP" }))).toBe("presenter-window");
    expect(shortcutFor(key("n", { metaKey: true, altKey: true }))).toBeUndefined();
  });
});

describe("tastene i talervisningen", () => {
  it("fjernkontrollen og piltastene blar, Esc avslutter", () => {
    expect(presenterKey(key("PageDown"))).toBe("next");
    expect(presenterKey(key("ArrowLeft"))).toBe("previous");
    expect(presenterKey(key("ArrowDown"))).toBe("down");
    expect(presenterKey(key("Escape"))).toBe("stop");
    expect(presenterKey(key("ArrowRight", { metaKey: true }))).toBeUndefined();
  });
});
