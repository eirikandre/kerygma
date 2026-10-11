// Fila som er åpen i editoren: stien, det som sist ble lest fra eller skrevet til disk,
// om det finnes ulagrede endringer, og autolagringen. Lagrer 1 sekund etter siste
// tastetrykk, og før en annen fil åpnes. Endringer utenfra lastes inn hvis editoren
// ikke har ulagrede endringer.
import { api, basename, errorMessage } from "./api.ts";
import { notice } from "./dialogs.ts";
import type { Editor } from "./editor.ts";

const SAVE_DELAY_MS = 1000;

export interface DocumentHooks {
  /** Innholdet i editoren er byttet ut med det som er på disk (endret utenfra). */
  reloaded: () => Promise<void>;
  /** Fila er slettet eller flyttet utenfra, og er lukket. */
  closed: () => void;
}

export class OpenDocument {
  /** Fila som er åpen, relativt til vault-roten. */
  path: string | undefined;
  /** Det som sist ble lest fra eller skrevet til disk. */
  private saved = "";
  private dirty = false;
  private timer: number | undefined;

  constructor(
    private editor: Editor,
    private hooks: DocumentHooks,
  ) {}

  /** Innholdet i editoren, med ulagrede endringer. */
  get content(): string {
    return this.editor.getValue();
  }

  /** Editoren er endret: lagre om litt. */
  changed() {
    this.dirty = true;
    window.clearTimeout(this.timer);
    this.timer = window.setTimeout(() => void this.save(), SAVE_DELAY_MS);
  }

  /** Lagrer ulagrede endringer nå. Feil vises som en melding, og endringene prøves igjen neste gang. */
  async save() {
    window.clearTimeout(this.timer);
    if (!this.path || !this.dirty) return;
    const path = this.path;
    const content = this.editor.getValue();
    this.dirty = false;
    try {
      await api.writeNote(path, content);
      if (this.path === path) this.saved = content;
    } catch (error) {
      if (this.path === path) this.dirty = true;
      notice(errorMessage(error), { error: true });
    }
  }

  /** Lagrer fila som er åpen, og leser `path` inn i editoren. Gir innholdet. */
  async open(path: string): Promise<string> {
    await this.save();
    const content = await api.readNote(path);
    this.path = path;
    this.saved = content;
    this.dirty = false;
    this.editor.load(content);
    return content;
  }

  /** Lukker fila uten å lagre (det er gjort, eller fila er borte). */
  close() {
    window.clearTimeout(this.timer);
    this.path = undefined;
    this.saved = "";
    this.dirty = false;
    this.editor.load("");
  }

  /** Fila eller en mappe over den har fått ny sti. */
  moved(from: string, to: string) {
    if (this.path === from) this.path = to;
    else if (this.path?.startsWith(`${from}/`)) this.path = to + this.path.slice(from.length);
  }

  /** Leser fila på nytt etter at appen selv har skrevet den (som «Lag presentasjon»). */
  async reload() {
    if (!this.path) return;
    const content = await api.readNote(this.path);
    this.saved = content;
    this.dirty = false;
    this.editor.replace(content);
  }

  /**
   * Filer endret utenfra (Obsidian, Finder, Word-import). Er fila som er åpen endret, og
   * editoren ikke har ulagrede endringer, lastes den inn. Har editoren ulagrede endringer,
   * går de foran og lagres om litt, og brukeren får beskjed.
   */
  async externalChange(paths: string[]) {
    const path = this.path;
    if (!path || !paths.includes(path)) return;
    if (this.dirty) {
      const content = await api.readNote(path).catch(() => undefined);
      if (content !== undefined && this.path === path && this.dirty && content !== this.saved) {
        this.saved = content;
        notice(`«${basename(path)}» ble endret utenfra mens du skrev. Din versjon lagres over den.`, { error: true });
      }
      return;
    }
    let content: string;
    try {
      content = await api.readNote(path);
    } catch {
      this.close();
      this.hooks.closed();
      return;
    }
    if (this.path === path && !this.dirty && content !== this.saved && content !== this.editor.getValue()) {
      this.saved = content;
      this.editor.replace(content);
      await this.hooks.reloaded();
    }
  }
}
