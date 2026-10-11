// Kommandoene i appen, og hvordan de startes: menyen (fra Rust), hurtigtastene og knappene.
// Hver kommando har en id, som er den samme som id-en til menyvalget i src-tauri/src/lib.rs.
import { events } from "./api.ts";

export type Commands = Record<string, () => void | Promise<void>>;

export interface Shortcut {
  /** `event.key` med små bokstaver. */
  key: string;
  shift: boolean;
  id: string;
}

/** Cmd/Ctrl + tast. Står også i menyen på macOS (`lib.rs::menu`). */
export const SHORTCUTS: Shortcut[] = [
  { key: "n", shift: false, id: "new-note" },
  { key: "e", shift: false, id: "toggle-preview" },
  { key: "m", shift: true, id: "create-presentation" },
  { key: "p", shift: true, id: "show-presentation" },
  { key: "b", shift: true, id: "insert-verse" },
  { key: ",", shift: false, id: "settings" },
  { key: "f", shift: true, id: "search" },
  { key: "t", shift: true, id: "toggle-sidebar" },
  // Cmd/Ctrl + Alt + Shift + P (talervisning i eget vindu) håndteres for seg, se `shortcutFor`.
];

/** Kommandoen tastetrykket starter, eller undefined. */
export function shortcutFor(event: Pick<KeyboardEvent, "key" | "code" | "metaKey" | "ctrlKey" | "altKey" | "shiftKey">): string | undefined {
  if (!(event.metaKey || event.ctrlKey)) return undefined;
  // Cmd/Ctrl + Alt + Shift + P: Alt endrer tegnet (∏ på Mac), så tasten leses fra `code`.
  if (event.altKey && event.shiftKey && event.code === "KeyP") return "presenter-window";
  if (event.altKey) return undefined;
  return SHORTCUTS.find((s) => s.key === event.key.toLowerCase() && s.shift === event.shiftKey)?.id;
}

/**
 * Kobler kommandoene til menyen og hurtigtastene, og gir funksjonen som starter en kommando.
 * `blocked`: en presentasjon eller et modalvindu er åpent, og kommandoer skal ikke åpne noe oppå.
 * `run`: kjører handlingen og viser feil som en melding.
 */
export function connectCommands(
  commands: Commands,
  blocked: () => boolean,
  run: (action: () => Promise<void>) => Promise<void>,
): (id: string) => void {
  // På macOS kan både menyen og grensesnittet få samme hurtigtast; det andre kallet ignoreres.
  let last = { id: "", at: 0 };
  const command = (id: string) => {
    const now = Date.now();
    if (last.id === id && now - last.at < 300) return;
    last = { id, at: now };
    if (blocked()) return;
    const action = commands[id];
    if (action) void run(async () => action());
  };

  document.addEventListener("keydown", (event) => {
    if (document.querySelector(".modal-backdrop")) return;
    const id = shortcutFor(event);
    if (!id) return;
    event.preventDefault();
    event.stopPropagation();
    command(id);
  });
  void events.menu.listen((event) => command(event.payload));
  return command;
}
