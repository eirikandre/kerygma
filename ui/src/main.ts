// Appen: treet til venstre, én fil om gangen i editoren til høyre.
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import { api, basename, errorMessage, events, folderOf, presentationSource, type Tree } from "./api.ts";
import { loadAppearance, onAppearanceChange, zoomWithWheel } from "./appearance.ts";
import { connectCommands } from "./commands.ts";
import { confirmDelete, confirmOverwrite, notice } from "./dialogs.ts";
import { OpenDocument } from "./document.ts";
import { Editor } from "./editor.ts";
import type { ExportKind } from "./export.ts";
import {
  BOLD,
  ITALIC,
  STRIKETHROUGH,
  UNDERLINE,
  headingLevel,
  setHeading,
  toggleMark,
  toggleQuote,
  type Mark,
} from "./format.ts";
import { type IconName, icon, initTooltips, setIcon } from "./icons.ts";
import { importWord } from "./importReport.ts";
import { present } from "./present.ts";
import { monitorCount, otherMonitor, presenterRunning, startPresenter, stopPresenter } from "./presenter.ts";
import { setSafeHtml } from "./sanitize.ts";
import { SearchView } from "./search.ts";
import { openSettings } from "./settings.ts";
import { TreeView } from "./tree.ts";
import { openVerseModal } from "./verseModal.ts";
import "./styles.css";

const $ = <T extends HTMLElement = HTMLElement>(id: string) => document.getElementById(id) as T;

// Knappene i index.html har bare ikonnavn og hover-tekst; ikonene settes inn her.
for (const button of document.querySelectorAll<HTMLElement>("[data-icon]")) {
  setIcon(button, button.dataset.icon as IconName, button.dataset.tooltip ?? "");
}
initTooltips();

const state = {
  vaultPath: undefined as string | undefined,
  tree: undefined as Tree | undefined,
  mode: "edit" as Mode,
  /** Visningen sist valgt i et manus; presentasjoner åpnes alltid i lesevisning. */
  manuscriptMode: loadManuscriptMode(),
  /** Med to skjermer kan talervisningen startes fra manuset også. */
  screens: 1,
};

type Mode = "edit" | "preview";
const MODE_KEY = "kerygma.manuscriptMode";

function loadManuscriptMode(): Mode {
  try {
    return localStorage.getItem(MODE_KEY) === "preview" ? "preview" : "edit";
  } catch {
    return "edit";
  }
}

// --- Navigasjonstreet kan skjules ------------------------------------------

const SIDEBAR_KEY = "kerygma.sidebarHidden";

function setSidebarHidden(hidden: boolean) {
  $("app").classList.toggle("sidebar-hidden", hidden);
  try {
    localStorage.setItem(SIDEBAR_KEY, String(hidden));
  } catch {
    // Uten lagring huskes valget bare til appen lukkes.
  }
}

try {
  if (localStorage.getItem(SIDEBAR_KEY) === "true") $("app").classList.add("sidebar-hidden");
} catch {
  // Treet vises.
}

const editor = new Editor(
  $("editor"),
  () => {
    doc.changed();
    updatePresentationUi();
  },
  () => {
    // Overskriftsvalget viser nivået på linja markøren står på.
    $<HTMLSelectElement>("heading").value = String(Math.min(headingLevel(editor.view.state), 3));
  },
);

// Fila som er åpen, med autolagring og endringer utenfra.
const doc = new OpenDocument(editor, { reloaded: () => showMode(), closed: () => showClosed() });

// Skrift og størrelse fra innstillingene; Cmd/Ctrl + rullehjul over teksten endrer størrelsen.
onAppearanceChange(() => editor.view.requestMeasure());
zoomWithWheel([$("editor"), $("preview")]);
void loadAppearance();

// Søk i alle manusene. Et treff åpner fila og går til linja.
$("search-icon").innerHTML = icon("search");
const search = new SearchView($<HTMLInputElement>("search"), $("search-results"), $("tree"), (path, line) => {
  void run(() => openNoteAt(path, line));
});

const tree = new TreeView($("tree"), {
  open: (path) => void run(() => openNote(path)),
  newNote: (folder) => void run(() => newNote(folder)),
  newFolder: (parent) => void run(() => newFolder(parent)),
  rename: (path, name) => run(() => renamePath(path, name)),
  move: (path, folder) => void run(() => movePath(path, folder)),
  delete: (path, name) => void run(() => deletePath(path, name)),
  createPresentation: (path) => void run(() => createPresentation(path)),
  exportPresentation: (path, kind) => void run(() => exportPresentation(kind, path)),
  importWord: (folder) => void run(() => importWordTo(folder, true)),
});

// Tauri tar ikke imot filer som slippes i vinduet (da virker dra og slipp i treet på
// Windows), så webvisningen må ikke åpne dem i stedet.
for (const type of ["dragover", "drop"]) {
  document.addEventListener(type, (event) => {
    if ((event as DragEvent).dataTransfer?.types.includes("Files")) event.preventDefault();
  });
}

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
  await doc.save();
  const result = await api.openVault(chosen);
  state.vaultPath = chosen;
  tree.setVault(chosen);
  closeNote();
  showTree(result);
  $("search-box").hidden = false;
  $("new-note").hidden = false;
  $("new-folder").hidden = false;
  search.clear();
  $("empty-text").textContent = "Velg et manus i treet, eller lag et nytt med Cmd/Ctrl + N.";
  $("empty-open").hidden = true;
}

function showTree(result: Tree) {
  state.tree = result;
  tree.render(result);
  tree.setActive(doc.path);
  updatePresentationUi();
}

/** Finnes fila i treet? */
function inTree(path: string, node: Tree | undefined = state.tree): boolean {
  return !!node && (node.path === path || node.children.some((child) => inTree(path, child)));
}

/**
 * Knappene og varselet etter om fila er en presentasjon laget av appen
 * (markøren `<!-- kerygma-presentasjon: … -->`): bare presentasjoner kan vises,
 * «Lag presentasjon» lager den på nytt fra manuset, og i redigering varsles det
 * om at endringer blir overskrevet.
 */
function updatePresentationUi() {
  const source = doc.path ? presentationSource(doc.content) : undefined;
  const manuscript = source && doc.path ? `${folderOf(doc.path) ? `${folderOf(doc.path)}/` : ""}${source}` : undefined;
  const create = $<HTMLButtonElement>("create-presentation");
  // I manuset lager knappen presentasjonen; i presentasjonen kommer ▷ etter den.
  // Med to skjermer starter ▷ talervisningen, som lager slidene fra manuset, så da vises den i manuset også.
  $("show-presentation").hidden = !source && !(doc.path && state.screens > 1);
  setIcon(
    $("show-presentation"),
    "play",
    (state.screens > 1 ? "Talervisning på to skjermer (Cmd/Ctrl + Shift + P)" : "Vis presentasjon (Cmd/Ctrl + Shift + P)") +
      ". Alt-klikk: talervisning i eget vindu",
  );
  if (source) {
    const exists = !!manuscript && inTree(manuscript);
    create.disabled = !exists;
    setIcon(
      create,
      "presentation",
      exists
        ? `Lag presentasjonen på nytt fra «${basename(source)}» (Cmd/Ctrl + Shift + M)`
        : `Fant ikke manuset «${basename(source)}»`,
    );
  } else {
    create.disabled = false;
    setIcon(create, "presentation", "Lag presentasjon (Cmd/Ctrl + Shift + M)");
  }
  const banner = $("presentation-banner");
  banner.hidden = !source || state.mode !== "edit";
  if (source) {
    const exists = !!manuscript && inTree(manuscript);
    $("presentation-banner-text").textContent = exists
      ? `Dette er en presentasjon laget fra «${basename(source)}». ` +
        "Endringer du gjør her, blir overskrevet neste gang presentasjonen lages."
      : `Dette er en presentasjon laget fra «${basename(source)}», men manuset finnes ikke lenger. ` +
        "Presentasjonen kan ikke lages på nytt.";
    $("open-manuscript").hidden = !exists;
    $("open-manuscript").onclick = () => manuscript && void run(() => openNote(manuscript));
  }
}

async function refreshTree() {
  if (state.vaultPath) showTree(await api.tree());
}

// --- Manus ---------------------------------------------------------------

/**
 * Åpner fila. Presentasjoner åpnes i lesevisning, manus i visningen
 * som sist ble valgt i et manus, med mindre `mode` er oppgitt.
 */
async function openNote(path: string, mode?: Mode) {
  if (path === doc.path) return;
  const content = await doc.open(path);
  state.mode = mode ?? (presentationSource(content) ? "preview" : state.manuscriptMode);
  $("title").textContent = basename(path);
  $("toolbar").hidden = false;
  $("empty").hidden = true;
  tree.setActive(path);
  tree.setLocation(folderOf(path));
  await showMode();
  if (state.mode === "edit") editor.focus();
}

/** Åpner fila fra et søketreff, i redigering, og går til linja med treffet. */
async function openNoteAt(path: string, line?: number) {
  await openNote(path);
  if (line === undefined || doc.path !== path) return;
  if (state.mode === "preview") {
    state.mode = "edit";
    await showMode();
  }
  editor.revealLine(line, search.terms);
}

function closeNote() {
  doc.close();
  showClosed();
}

/** Viser at ingen fil er åpen. */
function showClosed() {
  $("toolbar").hidden = true;
  $("editor").hidden = true;
  $("preview").hidden = true;
  $("empty").hidden = false;
  tree.setActive(undefined);
  updatePresentationUi();
}

async function showMode() {
  const preview = state.mode === "preview";
  $("editor").hidden = preview;
  $("preview").hidden = !preview;
  $("format").hidden = preview;
  updatePresentationUi();
  setIcon($("toggle-preview"), preview ? "pencil" : "eye", preview ? "Rediger (Cmd/Ctrl + E)" : "Lesevisning (Cmd/Ctrl + E)");
  if (preview && doc.path) {
    setSafeHtml($("preview"), await api.renderNote(doc.path, doc.content));
  }
}

async function togglePreview() {
  if (!doc.path) return;
  state.mode = state.mode === "edit" ? "preview" : "edit";
  // Valget huskes for manus; i en presentasjon gjelder det bare der og da.
  if (!presentationSource(doc.content)) {
    state.manuscriptMode = state.mode;
    try {
      localStorage.setItem(MODE_KEY, state.mode);
    } catch {
      // Uten lagring huskes valget til appen lukkes.
    }
  }
  await showMode();
  if (state.mode === "edit") editor.focus();
}

/**
 * Lenker i lesevisningen: [[Manus]] og [tekst](Manus.md) åpnes i appen, #anker (fotnoter)
 * rulles til, og nettadresser åpnes i nettleseren. Vinduet skal aldri navigere bort fra appen.
 */
$("preview").addEventListener("click", (event) => {
  const link = (event.target as HTMLElement).closest("a");
  if (!link) return;
  event.preventDefault();
  const href = link.getAttribute("href") ?? "";
  if (link.dataset.href === undefined && href.startsWith("#")) {
    const id = decodeLink(href.slice(1));
    $("preview").querySelector(`#${CSS.escape(id)}`)?.scrollIntoView({ block: "center" });
    return;
  }
  if (link.dataset.href === undefined && /^(https?|mailto|tel):/i.test(href)) {
    void run(() => openUrl(href));
    return;
  }
  const target = link.dataset.href ?? decodeLink(href);
  if (target && doc.path) {
    const from = doc.path;
    void run(async () => {
      const path = await api.resolveLink(target, from);
      if (path?.endsWith(".md")) await openNote(path);
      else notice(`Fant ikke "${target}".`);
    });
  }
});

/** `%20` og lignende i lenker til filer, som `[tekst](Mitt%20manus.md)`. */
function decodeLink(href: string): string {
  try {
    return decodeURIComponent(href);
  } catch {
    return href;
  }
}

async function newNote(folder?: string) {
  if (!state.vaultPath) return;
  const path = await api.createNote(folder ?? tree.folder);
  await refreshTree();
  await openNote(path, "edit");
  startTitleRename();
}

async function newFolder(parent: string) {
  const path = await api.createFolder(parent, "");
  setSidebarHidden(false); // Navnet skrives i treet.
  await refreshTree();
  tree.setActive(doc.path);
  tree.startRename(path);
}

async function renamePath(path: string, newName: string) {
  await moved(path, await api.rename(path, newName));
}

async function movePath(path: string, folder: string) {
  await moved(path, await api.moveTo(path, folder));
}

/** Fila eller mappa har fått ny sti: følg etter med fila som er åpen. */
async function moved(path: string, newPath: string) {
  tree.moved(path, newPath);
  doc.moved(path, newPath);
  if (doc.path === newPath) $("title").textContent = basename(newPath);
  await refreshTree();
}

async function deletePath(path: string, name: string) {
  if (!(await confirmDelete(name))) return;
  if (doc.path === path || doc.path?.startsWith(`${path}/`)) closeNote();
  await api.delete(path);
  await refreshTree();
}

/** Klikk på tittelen for å gi fila nytt navn. */
function startTitleRename() {
  const title = $("title");
  if (!doc.path || title.querySelector("input")) return;
  const path = doc.path;
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
    title.textContent = basename(doc.path ?? path);
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
async function createPresentation(path = doc.path) {
  if (!path || !path.endsWith(".md")) {
    notice("Åpne et manus først.");
    return;
  }
  // Ulagrede endringer skal med, så innholdet tas fra editoren når fila er åpen.
  const content = path === doc.path ? doc.content : null;
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
      if (doc.path === result.path) {
        // Presentasjonen er åpen, men laget på nytt: vis det nye innholdet.
        await doc.reload();
        await showMode();
      } else {
        // Presentasjoner vises ikke i treet, så den åpnes alltid; der startes den med ▷.
        await openNote(result.path);
      }
    }
  }
}

/**
 * «Vis presentasjon». Med to skjermer starter talervisningen (slidene på den andre skjermen,
 * manuset her). `inWindow`: talervisning med slidene i et vanlig vindu,
 * uansett hvor mange skjermer det er.
 */
async function showPresentation(inWindow = false) {
  if (!doc.path) {
    notice(inWindow ? "Åpne et manus eller en presentasjon først." : "Åpne en presentasjon først.");
    return;
  }
  const isPresentation = !!presentationSource(doc.content);
  const target = inWindow ? "window" : await otherMonitor().catch(() => undefined);
  if (target) {
    await doc.save();
    try {
      await startPresenter(doc.path, isPresentation ? null : doc.content, target);
      return;
    } catch (error) {
      // Uten manuset (slettet eller flyttet) vises presentasjonen som før på to skjermer.
      if (!isPresentation || inWindow) throw error;
      notice(errorMessage(error), { error: true });
    }
  }
  // Presentasjonen startes fra presentasjonen, ikke fra manuset: først lages den, så vises den.
  if (!isPresentation) {
    notice("Lag presentasjonen først (Cmd/Ctrl + Shift + M). Den åpnes, og der starter du den med ▷.");
    return;
  }
  const slides = await api.slides(doc.path, doc.content);
  if (slides.length === 0) {
    notice("Det er ingenting å vise.");
    return;
  }
  await present(slides);
}

/** Ser etter om en skjerm er koblet til eller fra, så ▷ vises i manus når talervisningen kan brukes. */
async function updateScreens() {
  const screens = await monitorCount();
  if (screens === state.screens) return;
  state.screens = screens;
  updatePresentationUi();
}

/**
 * Eksporterer presentasjonen til PDF, PowerPoint eller HTML: presentasjonen selv, eller den som
 * er laget fra manuset. Fila lagres der brukeren velger, foreslått ved siden av presentasjonen.
 */
async function exportPresentation(kind: ExportKind, path = doc.path) {
  if (!path || !state.vaultPath) {
    notice("Åpne et manus eller en presentasjon først.");
    return;
  }
  await doc.save();
  const content = path === doc.path ? doc.content : await api.readNote(path);
  const presentation = await api.presentationPath(path, content);
  if (!presentation) {
    notice("Lag presentasjonen først (Cmd/Ctrl + Shift + M), så eksporteres den.");
    return;
  }
  const slides = await api.slides(presentation, presentation === doc.path ? doc.content : null);
  if (slides.length === 0) {
    notice("Det er ingenting å eksportere.");
    return;
  }
  // pptxgenjs er stor, så eksporten lastes først når den brukes.
  const { exportFileName, exportHtml, exportPdf, exportPptx } = await import("./export.ts");
  const folder = folderOf(presentation);
  const name = exportFileName(presentation, kind);
  const format = { pdf: "PDF", pptx: "PowerPoint", html: "HTML" }[kind];
  const target = await saveDialog({
    title: `Eksporter som ${format}`,
    defaultPath: [state.vaultPath, folder, name].filter(Boolean).join("/"),
    filters: [{ name: format, extensions: [kind] }],
  });
  if (!target) return;
  if (kind === "pdf") await exportPdf(slides, target);
  else if (kind === "pptx") await exportPptx(slides, target, basename(presentation));
  else await exportHtml(slides, target, basename(presentation));
  notice(`Eksportert: ${target.split(/[\\/]/).pop()}`);
}

function insertVerse() {
  if (!doc.path) {
    notice("Åpne et manus og sett markøren der verset skal inn.");
    return;
  }
  if (state.mode === "preview") {
    state.mode = "edit";
    void showMode();
  }
  openVerseModal(editor);
}

/** Importerer Word-manus til mappa. `chosen`: mappa er valgt i treet og går foran innstillingen. */
async function importWordTo(folder: string, chosen: boolean) {
  if (!state.vaultPath) {
    notice("Åpne en mappe først.");
    return;
  }
  await importWord(
    folder,
    async (path) => {
      await refreshTree();
      await openNote(path);
    },
    chosen,
  );
  await refreshTree();
}

// --- Kommandoer, knapper, meny og hurtigtaster -------------------------------

const commands: Record<string, () => void | Promise<void>> = {
  "open-vault": () => openVault(),
  "new-note": () => newNote(),
  "new-folder": () => newFolder(tree.folder),
  "toggle-sidebar": () => setSidebarHidden(!$("app").classList.contains("sidebar-hidden")),
  "import-word": () => importWordTo(doc.path ? folderOf(doc.path) : "", false),
  settings: () => openSettings(),
  "toggle-preview": () => togglePreview(),
  "create-presentation": () => createPresentation(),
  "show-presentation": () => showPresentation(),
  "presenter-window": () => showPresentation(true),
  "export-pdf": () => exportPresentation("pdf"),
  "export-pptx": () => exportPresentation("pptx"),
  "export-html": () => exportPresentation("html"),
  "insert-verse": () => insertVerse(),
  search: () => {
    if (state.vaultPath) {
      setSidebarHidden(false);
      search.focus();
    } else notice("Åpne en mappe først.");
  },
};

// Menyen sender kommandoer også når et modalvindu er åpent; de skal ikke åpne et nytt oppå.
const command = connectCommands(
  commands,
  () => !!document.querySelector(".presentation-overlay, .modal-backdrop") || presenterRunning(),
  run,
);

for (const id of ["toggle-preview", "insert-verse", "create-presentation"]) {
  $(id).addEventListener("click", () => command(id));
}
// Alt-klikk på ▷: talervisning med slidene i et vanlig vindu.
$("show-presentation").addEventListener("click", (event) =>
  command(event.altKey ? "presenter-window" : "show-presentation"),
);
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
$("format-quote").addEventListener("mousedown", (event) => event.preventDefault());
$("format-quote").addEventListener("click", () => toggleQuote(editor.view));
$<HTMLSelectElement>("heading").addEventListener("change", (event) => {
  setHeading(editor.view, Number((event.target as HTMLSelectElement).value));
});
$("new-note").addEventListener("click", () => command("new-note"));
$("new-folder").addEventListener("click", () => command("new-folder"));
$("hide-sidebar").addEventListener("click", () => command("toggle-sidebar"));
$("show-sidebar").addEventListener("click", () => command("toggle-sidebar"));
$("empty-open").addEventListener("click", () => command("open-vault"));
$("title").addEventListener("click", () => startTitleRename());

/** Filer endret utenfra (Obsidian, Finder, Word-import). */
void events.vaultChanged.listen((event) => {
  search.refresh();
  void run(async () => {
    await refreshTree();
    await doc.externalChange(event.payload);
  });
});

void events.bibleReady.listen(() => {
  void api.bibleStatus().then((status) => {
    if (status.kind === "failed") notice(`Kunne ikke laste biblene: ${status.message}`, { error: true });
  });
});
void events.bibleWarning.listen((event) => notice(event.payload, { error: true }));

// Lagre før vinduet lukkes.
void getCurrentWindow().onCloseRequested(async () => {
  await doc.save();
  await stopPresenter();
});
window.addEventListener("blur", () => void doc.save());
window.addEventListener("focus", () => void updateScreens());
void updateScreens();

// Åpne mappa fra sist.
void run(async () => {
  const last = await api.lastVault();
  if (last) await openVault(last);
});
