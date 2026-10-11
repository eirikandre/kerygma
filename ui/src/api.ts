// Kommandoene og typene fra Rust. Alt her kommer fra `bindings.ts`, som lages av
// src-tauri/src/lib.rs; endres en kommando i Rust, gir det feil her i `tsc`.
// Feil fra kommandoene kastes som en norsk melding (string).
import { commands, PRESENTATION_MARKER_PATTERN, type Suggestion_Serialize } from "./bindings.ts";

export {
  events,
  type Appearance,
  type BibleStatus,
  type CreateResult,
  type ImportedFile,
  type ImportTarget,
  type Kind,
  type Presenter,
  type SearchResponse,
  type SearchResult,
  type Settings,
  type Theme,
  type Tree,
} from "./bindings.ts";

export const api = commands;

/** Et forslag i oppslagsvinduet for bibelvers: tekst som kan settes inn, eller en melding. */
export type Suggestion = Suggestion_Serialize;

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

/** Markøren appen setter i presentasjoner, med samme mønster som `presentation_source` i kerygma-core. */
const PRESENTATION_MARKER = new RegExp(PRESENTATION_MARKER_PATTERN, "m");

/** Manuset (filnavnet, i samme mappe) presentasjonen er laget fra, eller undefined for andre filer. */
export function presentationSource(content: string): string | undefined {
  return PRESENTATION_MARKER.exec(content)?.[1];
}
