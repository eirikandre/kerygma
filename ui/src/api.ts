// Kommandoene i src-tauri/src/commands.rs. Feil kommer som en norsk melding (string).
import { invoke } from "@tauri-apps/api/core";

export type Kind = "Folder" | "Note" | "File";

export interface Tree {
  name: string;
  /** Relativt til vault-roten, med `/`. Tom for roten. */
  path: string;
  kind: Kind;
  children: Tree[];
}

export interface Settings {
  suffix: string;
  openCopy: boolean;
  maxChars: number;
  addTranslation: boolean;
  translation: string;
  defaultTranslation: string;
  aliases: string;
  importFolder: string;
  /** Temaet presentasjonen vises med (slideThemes.ts). */
  slideTheme: string;
  /** Skrift i overskrifter og i sitater/bibelvers på slidene. Tom: temaets. */
  headingFont: string;
  quoteFont: string;
}

export type Theme = "system" | "light" | "dark";

/** Skrift og størrelse i editoren og lesevisningen, og tema (app.json, ikke vaulten). */
export interface Appearance {
  font: string;
  fontSize: number;
  theme: Theme;
}

/** Et notat med treff i søket. */
export interface SearchResult {
  path: string;
  name: string;
  /** Alle søkeordene står i navnet. */
  nameMatch: boolean;
  /** De første linjene med treff (linjenummer fra 1). */
  lines: Array<{ line: number; text: string }>;
  /** Antall linjer med treff i hele notatet. */
  total: number;
}

export type CreateResult =
  | { kind: "created"; path: string; warnings: string[] }
  | { kind: "exists"; path: string }
  | { kind: "missingManuscript"; path: string }
  | { kind: "isFolder"; path: string };

/** Talervisningen: slidene laget fra manuset, og manuset med merker for hver slide. */
export interface Presenter {
  /** Manuset slidene er laget fra. */
  path: string;
  slides: string[];
  /** Manuset som HTML, med merkene fra `kerygma_core::presenter`. */
  manuscript: string;
  warnings: string[];
}

export interface Suggestion {
  /** Teksten som settes inn. Mangler når forslaget bare er en melding. */
  text?: string;
  message?: string;
}

export type BibleStatus = { kind: "loading" } | { kind: "ready"; ids: string[] } | { kind: "failed"; message: string };

export interface ImportTarget {
  file: string;
  name: string;
  path: string;
  exists: boolean;
  isFolder: boolean;
}

export interface ImportedFile {
  name: string;
  path?: string | null;
  quotes: number;
  uncertain: number;
  unmatched: string[];
  error?: string | null;
}

export const api = {
  lastVault: () => invoke<string | null>("last_vault"),
  appearance: () => invoke<Appearance>("appearance"),
  setAppearance: (appearance: Appearance) => invoke<Appearance>("set_appearance", { appearance }),
  openVault: (path: string) => invoke<Tree>("open_vault", { path }),
  tree: () => invoke<Tree>("tree"),
  readNote: (path: string) => invoke<string>("read_note", { path }),
  writeNote: (path: string, content: string) => invoke<void>("write_note", { path, content }),
  createNote: (folder: string) => invoke<string>("create_note", { folder }),
  createFolder: (parent: string, name: string) => invoke<string>("create_folder", { parent, name }),
  rename: (path: string, newName: string) => invoke<string>("rename", { path, newName }),
  delete: (path: string) => invoke<void>("delete", { path }),
  search: (query: string) => invoke<SearchResult[]>("search", { query }),
  renderNote: (path: string, content: string) => invoke<string>("render_note", { path, content }),
  resolveLink: (target: string, from: string) => invoke<string | null>("resolve_link", { target, from }),
  createPresentation: (path: string, content: string | null, overwrite: boolean) =>
    invoke<CreateResult>("create_presentation", { path, content, overwrite }),
  slides: (path: string, content: string | null) => invoke<string[]>("slides", { path, content }),
  presenter: (path: string, content: string | null) => invoke<Presenter>("presenter", { path, content }),
  /** Presentasjonen som eksporteres fra notatet, eller null når den ikke er laget. */
  presentationPath: (path: string, content: string) => invoke<string | null>("presentation_path", { path, content }),
  lookupVerse: (input: string) => invoke<Suggestion[]>("lookup_verse", { input }),
  quoteVerse: (verse: string, before: string, after: string) => invoke<string>("quote_verse", { verse, before, after }),
  getSettings: () => invoke<Settings>("get_settings"),
  setSettings: (settings: Settings) => invoke<string[]>("set_settings", { settings }),
  bibleStatus: () => invoke<BibleStatus>("bible_status"),
  /** `folderChosen`: mappa er valgt i treet og går foran «Mappe for importerte Word-manus». */
  importTargets: (files: string[], currentFolder: string, folderChosen: boolean) =>
    invoke<ImportTarget[]>("import_targets", { files, currentFolder, folderChosen }),
  importWord: (files: Array<{ file: string; path: string }>) => invoke<ImportedFile[]>("import_word", { files }),
};

/** Feilmeldingen fra en kommando eller et unntak. */
export function errorMessage(error: unknown): string {
  return typeof error === "string" ? error : error instanceof Error ? error.message : String(error);
}

/** Filnavnet uten mappe og uten `.md`. */
export function basename(path: string): string {
  return (path.split("/").pop() ?? path).replace(/\.md$/, "");
}

/** Mappa stien ligger i, uten `/` på slutten (tom for roten). */
export function folderOf(path: string): string {
  const i = path.lastIndexOf("/");
  return i < 0 ? "" : path.slice(0, i);
}

/** Markøren appen setter i presentasjoner, som `presentation_source` i kerygma-core. */
const PRESENTATION_MARKER = /^<!-- kerygma-presentasjon: (.+?) -->[ \t]*\r?$/m;

/** Manuset (filnavnet, i samme mappe) presentasjonen er laget fra, eller undefined for andre notater. */
export function presentationSource(content: string): string | undefined {
  return PRESENTATION_MARKER.exec(content)?.[1];
}
