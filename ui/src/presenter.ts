// Talervisning: slidene vises i fullskjerm på den andre skjermen, eller i et vanlig vindu
// som kan fanges i OBS (audience.ts), og manuset vises i lesevisning i hovedvinduet, uten
// treet, med det som er på sliden markert. Sliden følger hvor langt du har rullet: når neste del av
// manuset når leselinja (en tredjedel ned), vises den. Venstre/høyre pil og PageUp/PageDown
// går til forrige og neste slide, og ruller manuset dit; pil opp og ned ruller manuset.
import { emitTo, listen, type UnlistenFn } from "@tauri-apps/api/event";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import {
  availableMonitors,
  type BackgroundThrottlingPolicy,
  currentMonitor,
  getCurrentWindow,
  type Monitor,
} from "@tauri-apps/api/window";
import { api, basename, errorMessage } from "./api.ts";
import { notice } from "./dialogs.ts";
import { setIcon } from "./icons.ts";
import { PRESENTER_EVENTS, type PresenterInit, type PresenterKey, presenterKey } from "./presenterEvents.ts";
import { setSafeHtml } from "./sanitize.ts";

/** Leselinja, som andel av høyden på manuset. */
const READING_LINE = 1 / 3;
const AUDIENCE_LABEL = "audience";
/** Tittelen på slidevinduet. Den er fast, så OBS finner vinduet igjen neste gang. */
const AUDIENCE_TITLE = "Kerygma – presentasjon";
/** Størrelse og plassering på slidevinduet når det er et vanlig vindu (localStorage). */
const WINDOW_KEY = "kerygma.presenterWindow";

/** Hvor slidene vises: i fullskjerm på en skjerm, eller i et vanlig vindu. */
export type AudienceTarget = Monitor | "window";

interface WindowBounds {
  x: number;
  y: number;
  width: number;
  height: number;
}

/** Vinduet som sist, eller 1280 × 720 (16:9, som i OBS) midt på skjermen. */
function loadWindowBounds(): WindowBounds | undefined {
  try {
    const bounds = JSON.parse(localStorage.getItem(WINDOW_KEY) ?? "null") as WindowBounds | null;
    return bounds && [bounds.x, bounds.y, bounds.width, bounds.height].every(Number.isFinite) ? bounds : undefined;
  } catch {
    return undefined;
  }
}

async function saveWindowBounds(window: WebviewWindow) {
  try {
    const scale = await window.scaleFactor();
    const position = (await window.outerPosition()).toLogical(scale);
    const size = (await window.innerSize()).toLogical(scale);
    const bounds: WindowBounds = { x: position.x, y: position.y, width: size.width, height: size.height };
    localStorage.setItem(WINDOW_KEY, JSON.stringify(bounds));
  } catch {
    // Uten lagring åpnes vinduet midt på skjermen neste gang.
  }
}

// Merkene fra `kerygma_core::presenter`: START + i der slide i starter, END + i der den slutter.
const START = 0xf0000;
const END = 0x100000;
const MARKER = /[\u{F0000}-\u{10FFFD}]/u;

/** Skjermen som ikke har hovedvinduet, når det er to eller flere. */
export async function otherMonitor(): Promise<Monitor | undefined> {
  const [monitors, current] = await Promise.all([availableMonitors(), currentMonitor()]);
  if (monitors.length < 2) return undefined;
  const same = (a: Monitor, b: Monitor | null) =>
    !!b && a.position.x === b.position.x && a.position.y === b.position.y && a.name === b.name;
  return monitors.find((monitor) => !same(monitor, current)) ?? monitors[1];
}

/** Antall skjermer, for å vise ▷ også i manus når talervisningen kan brukes. */
export async function monitorCount(): Promise<number> {
  return (await availableMonitors().catch(() => [])).length;
}

let running = false;
let current: Presenter | undefined;

export function presenterRunning(): boolean {
  return running;
}

/** Avslutter talervisningen, hvis den kjører (før hovedvinduet lukkes). */
export async function stopPresenter() {
  await current?.stop();
}

/**
 * Starter talervisningen for notatet (manuset eller presentasjonen laget fra det).
 * `content` er innholdet i editoren når notatet er manuset, så ulagrede endringer blir med.
 */
export async function startPresenter(path: string, content: string | null, target: AudienceTarget) {
  if (running) return;
  const data = await api.presenter(path, content);
  for (const warning of data.warnings) notice(warning, { error: true });
  if (data.slides.length === 0) {
    notice("Det er ingenting å vise.");
    return;
  }
  running = true;
  current = new Presenter(data.path, data.slides, data.manuscript, target);
  try {
    await current.start();
  } catch (error) {
    await current.stop();
    throw error;
  }
}

class Presenter {
  private root = document.createElement("div");
  private scroller = document.createElement("div");
  private counter = document.createElement("span");
  /** Det som er markert for hver slide; tomt for slides som ikke kunne finnes i manuset. */
  private parts: HTMLElement[][] = [];
  private index = 0;
  /** Rullingen tastene satte; rulling dit skal ikke velge sliden på nytt. */
  private scrolledTo: number | undefined;
  private unlisten: UnlistenFn[] = [];
  /** Holder plassen over manuset like høy som avstanden til leselinja når vinduet endres. */
  private resize = new ResizeObserver(() => this.fitTopSpace());
  private article = document.createElement("article");
  private audience: WebviewWindow | undefined;
  private stopped = false;
  /** Det som hadde fokus før talervisningen, og får det igjen etterpå. */
  private previousFocus = document.activeElement as HTMLElement | null;

  constructor(
    private path: string,
    private slides: string[],
    private manuscript: string,
    private target: AudienceTarget,
  ) {}

  async start() {
    this.render();
    this.unlisten.push(
      await listen(PRESENTER_EVENTS.ready, () => {
        const init: PresenterInit = { slides: this.slides, index: this.index };
        void emitTo(AUDIENCE_LABEL, PRESENTER_EVENTS.init, init);
      }),
      await listen<PresenterKey>(PRESENTER_EVENTS.key, (event) => this.onKey(event.payload)),
    );
    document.addEventListener("keydown", this.onKeydown, true);
    await this.openAudience();
  }

  /** Vinduet med slidene: i fullskjerm på den andre skjermen, eller et vanlig vindu. */
  private async openAudience() {
    // Et vindu fra en talervisning som ikke ble avsluttet riktig, lukkes først. Vi venter til
    // det er borte, ellers kan hendelsen om at det er lukket, avslutte den nye talervisningen.
    const old = await WebviewWindow.getByLabel(AUDIENCE_LABEL);
    if (old) {
      await new Promise<void>((resolve) => {
        void old.once("tauri://destroyed", () => resolve());
        void old.destroy().catch(() => resolve());
        setTimeout(resolve, 2000);
      });
    }
    const common = {
      url: "audience.html",
      title: AUDIENCE_TITLE,
      focus: false,
      // Slidene skal byttes selv om macOS mener vinduet ikke er synlig (en annen Space,
      // skjermsparer, eller bak andre vinduer mens OBS fanger det).
      backgroundThrottling: "disabled" as BackgroundThrottlingPolicy,
    };
    let audience: WebviewWindow;
    if (this.target === "window") {
      const bounds = loadWindowBounds();
      audience = new WebviewWindow(AUDIENCE_LABEL, {
        ...common,
        ...(bounds ?? { width: 1280, height: 720, center: true }),
        resizable: true,
      });
    } else {
      const scale = this.target.scaleFactor;
      const position = this.target.position.toLogical(scale);
      const size = this.target.size.toLogical(scale);
      audience = new WebviewWindow(AUDIENCE_LABEL, {
        ...common,
        x: position.x,
        y: position.y,
        width: size.width,
        height: size.height,
        decorations: false,
      });
    }
    this.audience = audience;
    await new Promise<void>((resolve, reject) => {
      void audience.once("tauri://created", () => resolve());
      void audience.once("tauri://error", (event) => reject(event.payload));
    }).catch((error) => {
      void this.stop();
      throw new Error(`Kunne ikke åpne vinduet med slidene: ${errorMessage(error)}`);
    });
    this.unlisten.push(await audience.once("tauri://destroyed", () => void this.stop(false)));
    if (this.target === "window") {
      // Lukkes vinduet med den røde knappen, huskes størrelsen og plasseringen før det er borte.
      this.unlisten.push(await audience.onCloseRequested(() => saveWindowBounds(audience)));
    } else {
      await audience.setFullscreen(true).catch((error) => console.error(error));
    }
    // Hovedvinduet beholder fokus, så tastene og rullingen virker der med en gang.
    await getCurrentWindow().setFocus().catch(() => {});
  }

  private render() {
    this.root.className = "presenter";
    // Fokus her, ikke i editoren bak, så taster ikke skriver i manuset.
    this.root.tabIndex = -1;
    const header = document.createElement("header");
    header.className = "presenter-header";
    const title = document.createElement("h1");
    title.textContent = basename(this.path);
    this.counter.className = "presenter-counter";
    const button = (name: "chevron-left" | "chevron-right" | "x", label: string, action: () => void) => {
      const el = document.createElement("button");
      el.type = "button";
      el.className = "icon-button";
      setIcon(el, name, label);
      el.addEventListener("click", action);
      return el;
    };
    header.append(
      title,
      this.counter,
      button("chevron-left", "Forrige slide (←, PageUp)", () => this.onKey("previous")),
      button("chevron-right", "Neste slide (→, PageDown)", () => this.onKey("next")),
      button("x", "Avslutt talervisningen (Esc)", () => void this.stop()),
    );

    this.scroller.className = "presenter-scroll";
    const article = this.article;
    article.className = "markdown presenter-manuscript";
    setSafeHtml(article, this.manuscript);
    this.scroller.append(article);
    // Starten av manuset står under leselinja, så de første slidene også kan rulles opp til den.
    this.resize.observe(this.scroller);
    const line = document.createElement("div");
    line.className = "presenter-reading-line";
    line.style.top = `${READING_LINE * 100}%`;
    const body = document.createElement("div");
    body.className = "presenter-body";
    body.append(this.scroller, line);

    this.root.append(header, body);
    document.body.append(this.root);
    this.parts = markSlides(article, this.slides.length);
    this.scroller.addEventListener("scroll", this.onScroll, { passive: true });
    this.fitTopSpace();
    this.root.focus();
    this.show(0, false);
  }

  /** Hvor sliden starter og slutter, i piksler fra toppen av manuset (uten rulling). */
  private extent(index: number): { top: number; bottom: number } | undefined {
    const parts = this.parts[index];
    if (!parts?.length) return undefined;
    const offset = this.scroller.scrollTop - this.scroller.getBoundingClientRect().top;
    return {
      top: parts[0].getBoundingClientRect().top + offset,
      bottom: Math.max(...parts.map((part) => part.getBoundingClientRect().bottom)) + offset,
    };
  }

  /**
   * Plassen over manuset er like høy som avstanden ned til leselinja, så teksten starter
   * rett under linja. Rullingen justeres så det samme står ved linja etter endringen.
   */
  private fitTopSpace() {
    const space = Math.round(this.readingLine());
    const old = Number.parseFloat(this.article.style.paddingTop) || 0;
    if (space === old) return;
    this.article.style.paddingTop = `${space}px`;
    if (old > 0) {
      this.scroller.scrollTop += space - old;
      this.scrolledTo = this.scroller.scrollTop;
    }
  }

  private readingLine(): number {
    return this.scroller.clientHeight * READING_LINE;
  }

  /**
   * Sliden ved leselinja: den første som har nådd linja og ikke er rullet forbi ennå.
   * Er alle som har nådd linja rullet forbi, står den siste til neste når linja, og før
   * noen har nådd den, vises den første. Slik vises også to vers i samme avsnitt etter tur.
   */
  private slideAtReadingLine(): number {
    const line = this.scroller.scrollTop + this.readingLine();
    let first: number | undefined;
    let lastStarted: number | undefined;
    for (let i = 0; i < this.parts.length; i++) {
      const extent = this.extent(i);
      if (!extent) continue;
      first ??= i;
      if (extent.top > line) break;
      lastStarted = i;
      if (extent.bottom > line) return i;
    }
    return lastStarted ?? first ?? this.index;
  }

  private onScroll = () => {
    if (this.scrolledTo !== undefined && Math.abs(this.scroller.scrollTop - this.scrolledTo) < 2) return;
    this.scrolledTo = undefined;
    const index = this.slideAtReadingLine();
    if (index !== this.index) this.show(index, false);
  };

  private onKeydown = (event: KeyboardEvent) => {
    const key = presenterKey(event);
    if (!key) return;
    // Tastene i feltene i en melding eller et vindu oppå skal ikke bla.
    if (!this.root.contains(event.target as Node) && event.target !== document.body) return;
    event.preventDefault();
    event.stopPropagation();
    this.onKey(key);
  };

  private onKey(key: PresenterKey) {
    if (key === "stop") void this.stop();
    // Én linje om gangen, så en slide på én linje ikke hoppes over. Holdes tasten inne, gjentas den.
    else if (key === "down" || key === "up") this.scrollLines(key === "down" ? 1 : -1);
    else this.show(Math.min(Math.max(this.index + (key === "next" ? 1 : -1), 0), this.slides.length - 1), true);
  }

  /** Ruller manuset `lines` linjer (negativt: oppover). Sliden følger med via `onScroll`. */
  private scrollLines(lines: number) {
    const lineHeight = Number.parseFloat(getComputedStyle(this.article).lineHeight) || 24;
    this.scroller.scrollBy({ top: lines * lineHeight });
  }

  /** Viser sliden på den andre skjermen og markerer den i manuset. `scroll`: rull manuset dit. */
  private show(index: number, scroll: boolean) {
    for (const part of this.parts[this.index] ?? []) part.classList.remove("current");
    this.index = index;
    for (const part of this.parts[index] ?? []) part.classList.add("current");
    this.counter.textContent = `${index + 1} / ${this.slides.length}`;
    // Før vinduet med slidene finnes, går meldingen ingen steder; det får sliden med `init`.
    emitTo(AUDIENCE_LABEL, PRESENTER_EVENTS.slide, index).catch(() => {});
    const extent = this.extent(index);
    if (scroll && extent) {
      // Leselinja rett under starten av sliden og forbi slutten av den forrige, så det er
      // denne sliden som er ved linja (to slides kan starte på samme linje).
      const previous = this.extent(index - 1);
      const at = Math.min(Math.max(extent.top, previous?.bottom ?? extent.top) + 1, extent.bottom - 1);
      this.scroller.scrollTop = Math.max(0, at - this.readingLine());
      this.scrolledTo = this.scroller.scrollTop;
    }
  }

  /** Avslutter. `closeAudience`: lukk vinduet med slidene (ikke når det allerede er lukket). */
  async stop(closeAudience = true) {
    if (this.stopped) return;
    this.stopped = true;
    document.removeEventListener("keydown", this.onKeydown, true);
    for (const unlisten of this.unlisten) unlisten();
    this.resize.disconnect();
    this.root.remove();
    this.previousFocus?.focus();
    running = false;
    if (current === this) current = undefined;
    if (closeAudience && this.audience) {
      if (this.target === "window") await saveWindowBounds(this.audience);
      await this.audience.destroy().catch(() => {});
    }
  }
}

/**
 * Finner merkene i manuset, fjerner dem og pakker teksten mellom START + i og END + i
 * inn i `<span class="slide-part">` (og merker bilder). Gir det som hører til hver slide.
 */
function markSlides(root: HTMLElement, count: number): HTMLElement[][] {
  // Hvert merke blir et tomt element, så teksten mellom kan finnes med en Range.
  const starts = new Map<number, HTMLElement[]>();
  const ends = new Map<number, HTMLElement[]>();
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
  const texts: Text[] = [];
  for (let node = walker.nextNode(); node; node = walker.nextNode()) {
    if (MARKER.test(node.nodeValue ?? "")) texts.push(node as Text);
  }
  for (const text of texts) {
    let node = text;
    for (;;) {
      const value = node.nodeValue ?? "";
      const match = MARKER.exec(value);
      if (!match) break;
      const code = match[0].codePointAt(0)!;
      const after = node.splitText(match.index);
      after.nodeValue = (after.nodeValue ?? "").slice(match[0].length);
      const marker = document.createElement("span");
      marker.className = "slide-marker";
      after.before(marker);
      const [map, index] = code >= END ? [ends, code - END] : [starts, code - START];
      map.set(index, [...(map.get(index) ?? []), marker]);
      node = after;
    }
  }

  const parts: HTMLElement[][] = [];
  for (let i = 0; i < count; i++) {
    const slideParts: HTMLElement[] = [];
    const slideStarts = starts.get(i) ?? [];
    const slideEnds = ends.get(i) ?? [];
    slideStarts.forEach((start, k) => {
      const end = slideEnds[k];
      if (!end) return;
      const range = document.createRange();
      range.setStartAfter(start);
      range.setEndBefore(end);
      slideParts.push(...wrapRange(range, i));
    });
    parts.push(slideParts);
  }
  for (const marker of root.querySelectorAll(".slide-marker")) marker.remove();
  return parts;
}

/** Pakker tekstnodene og bildene i `range` inn i elementer som kan markeres. */
function wrapRange(range: Range, index: number): HTMLElement[] {
  const container = range.commonAncestorContainer;
  const scope = container.nodeType === Node.ELEMENT_NODE ? container : container.parentNode!;
  const walker = document.createTreeWalker(scope, NodeFilter.SHOW_TEXT | NodeFilter.SHOW_ELEMENT);
  const nodes: Node[] = [];
  for (let node = walker.nextNode(); node; node = walker.nextNode()) {
    if (!range.intersectsNode(node)) continue;
    if (node.nodeType === Node.TEXT_NODE && (node.nodeValue ?? "").trim() !== "") nodes.push(node);
    else if (node instanceof HTMLImageElement) nodes.push(node);
  }
  return nodes.map((node) => {
    if (node instanceof HTMLImageElement) {
      node.classList.add("slide-part");
      node.dataset.slide = String(index);
      return node;
    }
    const span = document.createElement("span");
    span.className = "slide-part";
    span.dataset.slide = String(index);
    node.parentNode!.insertBefore(span, node);
    span.append(node);
    return span;
  });
}
