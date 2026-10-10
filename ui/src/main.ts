// Appen: treet til venstre, én fil om gangen i editoren til høyre.
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { api, basename, errorMessage, folderOf, type Tree } from "./api.ts";
import { loadAppearance, onAppearanceChange, zoomWithWheel } from "./appearance.ts";
import { confirmDelete, confirmOverwrite, notice } from "./dialogs.ts";
import { Editor } from "./editor.ts";
import { BOLD, ITALIC, STRIKETHROUGH, UNDERLINE, headingLevel, setHeading, toggleMark, type Mark } from "./format.ts";
import { type IconName, initTooltips, setIcon } from "./icons.ts";
import { importWord } from "./importReport.ts";
import { present } from "./present.ts";
import { openSettings } from "./settings.ts";
import { TreeView } from "./tree.ts";
import { openVerseModal } from "./verseModal.ts";
import "./styles.css";

const SAVE_DELAY_MS = 1000;

const $ = <T extends HTMLElement = HTMLElement>(id: string) => document.getElementById(id) as T;

// Knappene i index.html har bare ikonnavn og hover-tekst; ikonene settes inn her.
for (const button of document.querySelectorAll<HTMLElement>("[data-icon]")) {
  setIcon(button, button.dataset.icon as IconName, button.dataset.tooltip ?? "");
}
initTooltips();

const state = {
  vaultPath: undefined as string | undefined,
  tree: undefined as Tree | undefined,
  /** Fila som er åpen, relativt til vault-roten. */
  path: undefined as string | undefined,
  /** Det som sist ble lest fra eller skrevet til disk. */
  saved: "",
  dirty: false,
  mode: "edit" as "edit" | "preview",
  saveTimer: undefined as number | undefined,
};

const editor = new Editor(
  $("editor"),
  () => {
    state.dirty = true;
    window.clearTimeout(state.saveTimer);
    state.saveTimer = window.setTimeout(() => void save(), SAVE_DELAY_MS);
  },
  () => {
    // Overskriftsvalget viser nivået på linja markøren står på.
    $<HTMLSelectElement>("heading").value = String(Math.min(headingLevel(editor.view.state), 3));
  },
);

// Skrift og størrelse fra innstillingene; Cmd/Ctrl + rullehjul over teksten endrer størrelsen.
onAppearanceChange(() => editor.view.requestMeasure());
zoomWithWheel([$("editor"), $("preview")]);
void loadAppearance();

const tree = new TreeView($("tree"), {
  open: (path) => void run(() => openNote(path)),
  newNote: (folder) => void run(() => newNote(folder)),
  newFolder: (parent) => void run(() => newFolder(parent)),
  rename: (path, name) => run(() => renamePath(path, name)),
  delete: (path, name) => void run(() => deletePath(path, name)),
  createPresentation: (path) => void run(() => createPresentation(path)),
});

/** Kjører en handling og viser feilen som en melding. */
async function run(action: () => Promise<void>) {
  try {
    await action();
  } catch (error) {
    console.error(error);
    notice(errorMessage(error), { error: true });
  }
}

// --- Vault ---------------------------------------------------------------

async function openVault(path?: string) {
  const chosen = path ?? (await openDialog({ directory: true, title: "Åpne mappe med manus" }));
  if (typeof chosen !== "string") return;
  await save();
  const result = await api.openVault(chosen);
  state.vaultPath = chosen;
  tree.setVault(chosen);
  closeNote();
  showTree(result);
  $("vault-name").textContent = result.name;
  $("empty-text").textContent = "Velg et notat i treet, eller lag et nytt med Cmd/Ctrl + N.";
  $("empty-open").hidden = true;
}

function showTree(result: Tree) {
  state.tree = result;
  tree.render(result);
  tree.setActive(state.path);
}

async function refreshTree() {
  if (state.vaultPath) showTree(await api.tree());
}

// --- Notater -------------------------------------------------------------

async function save() {
  window.clearTimeout(state.saveTimer);
  if (!state.path || !state.dirty) return;
  const path = state.path;
  const content = editor.getValue();
  state.dirty = false;
  try {
    await api.writeNote(path, content);
    if (state.path === path) state.saved = content;
  } catch (error) {
    if (state.path === path) state.dirty = true;
    notice(errorMessage(error), { error: true });
  }
}

async function openNote(path: string) {
  if (path === state.path) return;
  await save();
  const content = await api.readNote(path);
  state.path = path;
  state.saved = content;
  state.dirty = false;
  editor.load(content);
  $("title").textContent = basename(path);
  $("toolbar").hidden = false;
  $("empty").hidden = true;
  tree.setActive(path);
  await showMode();
  if (state.mode === "edit") editor.focus();
}

function closeNote() {
  state.path = undefined;
  state.saved = "";
  state.dirty = false;
  editor.load("");
  $("toolbar").hidden = true;
  $("editor").hidden = true;
  $("preview").hidden = true;
  $("empty").hidden = false;
  tree.setActive(undefined);
}

async function showMode() {
  const preview = state.mode === "preview";
  $("editor").hidden = preview;
  $("preview").hidden = !preview;
  $("format").hidden = preview;
  setIcon($("toggle-preview"), preview ? "pencil" : "eye", preview ? "Rediger (Cmd/Ctrl + E)" : "Lesevisning (Cmd/Ctrl + E)");
  if (preview && state.path) {
    $("preview").innerHTML = await api.renderNote(state.path, editor.getValue());
  }
}

async function togglePreview() {
  if (!state.path) return;
  state.mode = state.mode === "edit" ? "preview" : "edit";
  await showMode();
  if (state.mode === "edit") editor.focus();
}

/** Lenker i lesevisningen: [[Notat]] åpnes i appen, andre lenker i nettleseren. */
$("preview").addEventListener("click", (event) => {
  const link = (event.target as HTMLElement).closest("a");
  if (!link) return;
  event.preventDefault();
  const target = link.dataset.href;
  if (target !== undefined && state.path) {
    const from = state.path;
    void run(async () => {
      const path = await api.resolveLink(target, from);
      if (path?.endsWith(".md")) await openNote(path);
      else notice(`Fant ikke "${target}".`);
    });
  }
});

async function newNote(folder?: string) {
  if (!state.vaultPath) return;
  const path = await api.createNote(folder ?? (state.path ? folderOf(state.path) : ""));
  await refreshTree();
  state.mode = "edit";
  await openNote(path);
  startTitleRename();
}

async function newFolder(parent: string) {
  const path = await api.createFolder(parent, "");
  await refreshTree();
  tree.setActive(state.path);
  tree.startRename(path);
}

async function renamePath(path: string, newName: string) {
  const newPath = await api.rename(path, newName);
  if (state.path === path) {
    state.path = newPath;
    $("title").textContent = basename(newPath);
  } else if (state.path?.startsWith(`${path}/`)) {
    state.path = newPath + state.path.slice(path.length);
  }
  await refreshTree();
}

async function deletePath(path: string, name: string) {
  if (!(await confirmDelete(name))) return;
  if (state.path === path || state.path?.startsWith(`${path}/`)) {
    window.clearTimeout(state.saveTimer);
    closeNote();
  }
  await api.delete(path);
  await refreshTree();
}

/** Klikk på tittelen for å gi fila nytt navn. */
function startTitleRename() {
  const title = $("title");
  if (!state.path || title.querySelector("input")) return;
  const path = state.path;
  const input = document.createElement("input");
  input.className = "title-input";
  input.value = basename(path);
  title.replaceChildren(input);
  input.focus();
  input.select();
  let done = false;
  const finish = async (saveName: boolean) => {
    if (done) return;
    done = true;
    const value = input.value.trim();
    title.textContent = basename(state.path ?? path);
    if (saveName && value && value !== basename(path)) await run(() => renamePath(path, value));
    editor.focus();
  };
  input.addEventListener("keydown", (event) => {
    event.stopPropagation();
    if (event.key === "Enter") void finish(true);
    if (event.key === "Escape") void finish(false);
  });
  input.addEventListener("blur", () => void finish(true));
}

// --- Presentasjon ----------------------------------------------------------

/** «Lag presentasjon», fra fila som er åpen eller fra høyreklikk i treet. */
async function createPresentation(path = state.path) {
  if (!path || !path.endsWith(".md")) {
    notice("Åpne et manus først.");
    return;
  }
  // Ulagrede endringer skal med, så innholdet tas fra editoren når fila er åpen.
  const content = path === state.path ? editor.getValue() : null;
  let result = await api.createPresentation(path, content, false);
  if (result.kind === "exists") {
    if (!(await confirmOverwrite(result.path))) return;
    result = await api.createPresentation(path, content, true);
  }
  switch (result.kind) {
    case "missingManuscript":
      notice(`Fant ikke manuset "${result.path}".`);
      return;
    case "isFolder":
      notice(`"${result.path}" er en mappe.`);
      return;
    case "exists":
      return;
    case "created": {
      for (const warning of result.warnings) notice(warning, { error: true });
      notice(`Presentasjon laget: ${basename(result.path)}`);
      await refreshTree();
      if (result.open) {
        await openNote(result.path);
      } else if (state.path === result.path) {
        // Presentasjonen er åpen, men laget på nytt: vis det nye innholdet.
        const fresh = await api.readNote(result.path);
        state.saved = fresh;
        state.dirty = false;
        editor.replace(fresh);
        await showMode();
      }
    }
  }
}

async function showPresentation() {
  if (!state.path) {
    notice("Åpne en presentasjon først.");
    return;
  }
  const slides = await api.slides(state.path, editor.getValue());
  if (slides.length === 0) {
    notice("Det er ingenting å vise.");
    return;
  }
  await present(slides);
}

function insertVerse() {
  if (!state.path) {
    notice("Åpne et notat og sett markøren der verset skal inn.");
    return;
  }
  if (state.mode === "preview") {
    state.mode = "edit";
    void showMode();
  }
  openVerseModal(editor);
}

// --- Kommandoer, knapper, meny og hurtigtaster -------------------------------

const commands: Record<string, () => void | Promise<void>> = {
  "open-vault": () => openVault(),
  "new-note": () => newNote(),
  "import-word": async () => {
    if (!state.vaultPath) {
      notice("Åpne en mappe først.");
      return;
    }
    await importWord(state.path ? folderOf(state.path) : "", async (path) => {
      await refreshTree();
      await openNote(path);
    });
    await refreshTree();
  },
  settings: () => openSettings(),
  "toggle-preview": () => togglePreview(),
  "create-presentation": () => createPresentation(),
  "show-presentation": () => showPresentation(),
  "insert-verse": () => insertVerse(),
};

// På macOS kan både menyen og grensesnittet få samme hurtigtast; det andre kallet ignoreres.
let lastCommand = { id: "", at: 0 };
function command(id: string) {
  const now = Date.now();
  if (lastCommand.id === id && now - lastCommand.at < 300) return;
  lastCommand = { id, at: now };
  // Menyen sender kommandoer også når et modalvindu er åpent; de skal ikke åpne et nytt oppå.
  if (document.querySelector(".presentation-overlay, .modal-backdrop")) return;
  const action = commands[id];
  if (action) void run(async () => action());
}

const SHORTCUTS: Array<{ key: string; shift: boolean; id: string }> = [
  { key: "n", shift: false, id: "new-note" },
  { key: "e", shift: false, id: "toggle-preview" },
  { key: "m", shift: true, id: "create-presentation" },
  { key: "p", shift: true, id: "show-presentation" },
  { key: "b", shift: true, id: "insert-verse" },
  { key: ",", shift: false, id: "settings" },
];

document.addEventListener("keydown", (event) => {
  if (!(event.metaKey || event.ctrlKey) || event.altKey) return;
  if (document.querySelector(".modal-backdrop")) return;
  const shortcut = SHORTCUTS.find((s) => s.key === event.key.toLowerCase() && s.shift === event.shiftKey);
  if (!shortcut) return;
  event.preventDefault();
  event.stopPropagation();
  command(shortcut.id);
});

for (const id of ["toggle-preview", "insert-verse", "create-presentation", "show-presentation"]) {
  $(id).addEventListener("click", () => command(id));
}
// Formatering. mousedown hindres, så editoren beholder markeringen når knappen trykkes.
const FORMAT_BUTTONS: Array<[string, Mark]> = [
  ["format-bold", BOLD],
  ["format-italic", ITALIC],
  ["format-underline", UNDERLINE],
  ["format-strikethrough", STRIKETHROUGH],
];
for (const [id, mark] of FORMAT_BUTTONS) {
  $(id).addEventListener("mousedown", (event) => event.preventDefault());
  $(id).addEventListener("click", () => toggleMark(editor.view, mark));
}
$<HTMLSelectElement>("heading").addEventListener("change", (event) => {
  setHeading(editor.view, Number((event.target as HTMLSelectElement).value));
});
$("open-vault").addEventListener("click", () => command("open-vault"));
$("empty-open").addEventListener("click", () => command("open-vault"));
$("settings-button").addEventListener("click", () => command("settings"));
$("title").addEventListener("click", () => startTitleRename());

void listen<string>("menu", (event) => command(event.payload));

/** Filer endret utenfra (Obsidian, Finder, Word-import). */
void listen<string[]>("vault-changed", (event) => {
  void run(async () => {
    await refreshTree();
    const path = state.path;
    if (!path || !event.payload.includes(path) || state.dirty) return;
    let content: string;
    try {
      content = await api.readNote(path);
    } catch {
      closeNote(); // Fila er slettet eller flyttet.
      return;
    }
    if (state.path === path && !state.dirty && content !== state.saved && content !== editor.getValue()) {
      state.saved = content;
      editor.replace(content);
      await showMode();
    }
  });
});

void listen("bible-ready", () => {
  void api.bibleStatus().then((status) => {
    if (status.kind === "failed") notice(`Kunne ikke laste biblene: ${status.message}`, { error: true });
  });
});
void listen<string>("bible-warning", (event) => notice(event.payload, { error: true }));

// Lagre før vinduet lukkes.
void getCurrentWindow().onCloseRequested(async () => {
  await save();
});
window.addEventListener("blur", () => void save());

// Åpne mappa fra sist.
void run(async () => {
  const last = await api.lastVault();
  if (last) await openVault(last);
});
