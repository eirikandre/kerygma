// Søk i alle notatene i vaulten (navn og innhold). Feltet ligger øverst i
// sidefeltet; når det står noe i det, vises treffene i stedet for treet.
import { api, errorMessage, folderOf, type SearchResult } from "./api.ts";

const SEARCH_DELAY_MS = 150;

/** Ordene i søket, som i `kerygma_core::search::terms`. */
export function searchTerms(query: string): string[] {
  return query.normalize("NFC").toLowerCase().split(/\s+/).filter(Boolean);
}

export class SearchView {
  private timer: number | undefined;
  private request = 0;

  constructor(
    private input: HTMLInputElement,
    private results: HTMLElement,
    private tree: HTMLElement,
    /** Åpner notatet, og går til linja hvis den er oppgitt. */
    private open: (path: string, line?: number) => void,
  ) {
    input.addEventListener("input", () => this.schedule());
    input.addEventListener("keydown", (event) => {
      event.stopPropagation();
      if (event.key === "Escape") {
        event.preventDefault();
        this.clear();
      } else if (event.key === "Enter") {
        event.preventDefault();
        this.results.querySelector<HTMLElement>(".search-file")?.click();
      } else if (event.key === "ArrowDown") {
        event.preventDefault();
        this.results.querySelector<HTMLElement>("[tabindex]")?.focus();
      }
    });
    // Piltaster mellom treffene, Enter åpner, Esc går tilbake til feltet.
    results.addEventListener("keydown", (event) => {
      const items = [...results.querySelectorAll<HTMLElement>("[tabindex]")];
      const i = items.indexOf(document.activeElement as HTMLElement);
      if (event.key === "ArrowDown" || event.key === "ArrowUp") {
        event.preventDefault();
        const next = i + (event.key === "ArrowDown" ? 1 : -1);
        if (next < 0) input.focus();
        else items[Math.min(next, items.length - 1)]?.focus();
      } else if (event.key === "Enter" && i >= 0) {
        event.preventDefault();
        items[i].click();
      } else if (event.key === "Escape") {
        event.preventDefault();
        input.focus();
      }
    });
  }

  get active(): boolean {
    return this.input.value.trim() !== "";
  }

  focus() {
    this.input.focus();
    this.input.select();
  }

  clear() {
    this.input.value = "";
    this.schedule(0);
  }

  /** Søker på nytt med samme tekst, når filer er endret. */
  refresh() {
    if (this.active) this.schedule(0);
  }

  private schedule(delay = SEARCH_DELAY_MS) {
    window.clearTimeout(this.timer);
    this.timer = window.setTimeout(() => void this.run(), delay);
  }

  private async run() {
    const query = this.input.value;
    const id = ++this.request;
    if (!this.active) {
      this.results.hidden = true;
      this.tree.hidden = false;
      this.results.replaceChildren();
      return;
    }
    let found: SearchResult[];
    try {
      found = await api.search(query);
    } catch (error) {
      if (id !== this.request) return;
      this.show(message(errorMessage(error)));
      return;
    }
    if (id !== this.request) return; // Et nyere søk har allerede svart.
    this.show(this.render(found, searchTerms(query)));
  }

  private show(content: HTMLElement) {
    this.results.replaceChildren(content);
    this.results.hidden = false;
    this.tree.hidden = true;
  }

  private render(found: SearchResult[], terms: string[]): HTMLElement {
    const list = document.createElement("div");
    list.className = "search-list";
    const count = document.createElement("div");
    count.className = "search-count";
    count.textContent =
      found.length === 0 ? "Ingen treff" : found.length === 1 ? "1 notat" : `${found.length} notater`;
    list.append(count);

    for (const result of found) {
      const file = document.createElement("div");
      file.className = "search-file";
      file.tabIndex = -1;
      const name = document.createElement("span");
      name.className = "search-name";
      name.append(highlight(result.name, terms));
      file.append(name);
      const folder = folderOf(result.path);
      if (folder) {
        const where = document.createElement("span");
        where.className = "search-folder";
        where.textContent = folder;
        file.append(where);
      }
      if (result.total > 0) {
        const hits = document.createElement("span");
        hits.className = "search-hits";
        hits.textContent = String(result.total);
        file.append(hits);
      }
      file.addEventListener("click", () => this.open(result.path, result.lines[0]?.line));
      list.append(file);

      for (const line of result.lines) {
        const el = document.createElement("div");
        el.className = "search-line";
        el.tabIndex = -1;
        el.append(highlight(line.text, terms));
        el.addEventListener("click", () => this.open(result.path, line.line));
        list.append(el);
      }
      if (result.total > result.lines.length) {
        const more = document.createElement("div");
        more.className = "search-more";
        more.textContent = `+ ${result.total - result.lines.length} til`;
        list.append(more);
      }
    }
    return list;
  }
}

function message(text: string): HTMLElement {
  const el = document.createElement("div");
  el.className = "search-count";
  el.textContent = text;
  return el;
}

/** Teksten med søkeordene i <mark>, uten hensyn til store og små bokstaver. */
function highlight(text: string, terms: string[]): DocumentFragment {
  const fragment = document.createDocumentFragment();
  const normalized = text.normalize("NFC");
  const lower = normalized.toLowerCase();
  // Treffene som [start, slutt), slått sammen der de overlapper.
  const ranges: Array<[number, number]> = [];
  for (const term of terms) {
    for (let i = lower.indexOf(term); i >= 0 && term; i = lower.indexOf(term, i + term.length)) {
      ranges.push([i, i + term.length]);
    }
  }
  ranges.sort((a, b) => a[0] - b[0]);
  let pos = 0;
  for (const [start, end] of ranges) {
    if (end <= pos) continue;
    const from = Math.max(start, pos);
    fragment.append(normalized.slice(pos, from));
    const mark = document.createElement("mark");
    mark.textContent = normalized.slice(from, end);
    fragment.append(mark);
    pos = end;
  }
  fragment.append(normalized.slice(pos));
  return fragment;
}
