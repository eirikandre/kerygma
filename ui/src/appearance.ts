// Skrift og størrelse i editoren og lesevisningen. Lagres i app.json (gjelder
// appen, ikke vaulten) og kan endres med Cmd/Ctrl + rullehjul over teksten.
import { api, type Appearance } from "./api.ts";

export interface Font {
  id: string;
  name: string;
  /** CSS font-family, med reserver for Windows. */
  stack: string;
}

export const FONTS: Font[] = [
  { id: "georgia", name: "Georgia", stack: 'Georgia, "Iowan Old Style", serif' },
  { id: "iowan", name: "Iowan Old Style", stack: '"Iowan Old Style", Georgia, serif' },
  { id: "palatino", name: "Palatino", stack: 'Palatino, "Palatino Linotype", "Book Antiqua", serif' },
  { id: "times", name: "Times New Roman", stack: '"Times New Roman", Times, serif' },
  { id: "system", name: "Systemskrift", stack: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif' },
  { id: "helvetica", name: "Helvetica", stack: '"Helvetica Neue", Helvetica, Arial, sans-serif' },
  { id: "avenir", name: "Avenir", stack: '"Avenir Next", Avenir, "Segoe UI", sans-serif' },
  { id: "mono", name: "Monospace", stack: 'ui-monospace, "SF Mono", Menlo, Consolas, monospace' },
];

export const DEFAULT_APPEARANCE: Appearance = { font: "georgia", fontSize: 17 };
export const MIN_SIZE = 10;
export const MAX_SIZE = 40;

const SAVE_DELAY_MS = 400;
/**
 * Minste tid mellom to trinn med rullehjulet. Et hakk på musehjulet gir ett trinn;
 * styreflaten sender mange små hendelser, og de skal ikke gi store hopp.
 */
const WHEEL_INTERVAL_MS = 50;

let current: Appearance = { ...DEFAULT_APPEARANCE };
let saveTimer: number | undefined;
const listeners: Array<(appearance: Appearance) => void> = [];

export function getAppearance(): Appearance {
  return { ...current };
}

/** Kalles hver gang skrift eller størrelse endres (editoren må måle på nytt). */
export function onAppearanceChange(listener: (appearance: Appearance) => void) {
  listeners.push(listener);
}

function clampSize(size: number): number {
  return Math.min(MAX_SIZE, Math.max(MIN_SIZE, Math.round(size)));
}

function apply() {
  const font = FONTS.find((f) => f.id === current.font) ?? FONTS[0];
  const root = document.documentElement.style;
  root.setProperty("--editor-font", font.stack);
  root.setProperty("--editor-size", `${current.fontSize}px`);
  for (const listener of listeners) listener(getAppearance());
}

/** Leser skrift og størrelse fra app.json. */
export async function loadAppearance() {
  try {
    current = await api.appearance();
  } catch (error) {
    console.error(error);
  }
  apply();
}

/** Endrer skrift og/eller størrelse med en gang, og lagrer litt etterpå. */
export function setAppearance(change: Partial<Appearance>) {
  current = { ...current, ...change, fontSize: clampSize(change.fontSize ?? current.fontSize) };
  apply();
  window.clearTimeout(saveTimer);
  saveTimer = window.setTimeout(() => {
    api.setAppearance(current).catch((error) => console.error(error));
  }, SAVE_DELAY_MS);
}

/** Cmd/Ctrl + rullehjul (og knip på styreflaten) over elementene endrer størrelsen. */
export function zoomWithWheel(elements: HTMLElement[]) {
  let last = 0;
  const onWheel = (event: WheelEvent) => {
    if (!(event.ctrlKey || event.metaKey)) return;
    event.preventDefault();
    if (Math.abs(event.deltaY) < 1 || event.timeStamp - last < WHEEL_INTERVAL_MS) return;
    last = event.timeStamp;
    // Rull opp (negativ deltaY) gir større skrift, som i nettlesere.
    setAppearance({ fontSize: current.fontSize + (event.deltaY < 0 ? 1 : -1) });
  };
  for (const el of elements) el.addEventListener("wheel", onWheel, { passive: false });
}
