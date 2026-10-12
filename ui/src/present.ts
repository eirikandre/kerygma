// «Vis presentasjon»: samme DOM og CSS som Slides i Obsidian, i fullskjerm.
// `showDeck` brukes også av vinduet på den andre skjermen i talervisningen (audience.ts).
import { getCurrentWindow } from "@tauri-apps/api/window";
import Reveal from "reveal.js";
import revealCss from "reveal.js/dist/reveal.css?url";
import blackCss from "reveal.js/dist/theme/black.css?url";
import { api } from "./api.ts";
import { ditherBackground } from "./dither.ts";
import { setSafeHtml } from "./sanitize.ts";
import { applySlideStyle, type SlideStyle } from "./slideThemes.ts";

/** reveal.css, temaet og så slides.css, i den rekkefølgen. Fjernes når visningen avsluttes. */
async function loadStyles(): Promise<HTMLLinkElement[]> {
  const links = [revealCss, blackCss, "/slides.css"].map((href) => {
    const link = document.createElement("link");
    link.rel = "stylesheet";
    link.href = href;
    link.dataset.presentation = "";
    return link;
  });
  await Promise.all(
    links.map(
      (link) =>
        new Promise<void>((resolve) => {
          link.addEventListener("load", () => resolve(), { once: true });
          link.addEventListener("error", () => resolve(), { once: true });
          document.head.append(link);
        }),
    ),
  );
  return links;
}

export interface Deck {
  /** Går til slide `index` (fra 0). */
  slide(index: number): void;
  /** Elementet med slidene (`.reveal`). */
  element: HTMLElement;
  destroy(): void;
}

/** Størrelsen på en slide i reveal.js. */
export const SLIDE_WIDTH = 960;
export const SLIDE_HEIGHT = 700;
/**
 * Størrelsen på en slide i eksporten: 16:9, i samme skala som på skjermen (tekst i samme
 * størrelse i forhold til høyden). PDF-siden skaleres til full HD (1920 × 1080) etterpå.
 */
export const EXPORT_WIDTH = 1280;
export const EXPORT_HEIGHT = 720;

/**
 * Slidene over hele vinduet, med temaet fra innstillingene. `keyboard`: piltaster,
 * mellomrom og PageUp/PageDown blar i reveal.js; uten styres sliden bare med `slide`.
 * `exact` (til PDF): 16:9 øverst til venstre, så stor som vinduet tillater, og uten overganger.
 */
export async function showDeck(
  slides: string[],
  { keyboard, exact = false }: { keyboard: boolean; exact?: boolean },
): Promise<Deck> {
  const [links, style] = await Promise.all([
    loadStyles(),
    api.getSettings().then(
      (settings): SlideStyle => ({
        theme: settings.slideTheme,
        headingFont: settings.headingFont,
        quoteFont: settings.quoteFont,
      }),
      (): SlideStyle => ({ theme: "", headingFont: "", quoteFont: "" }),
    ),
  ]);

  const overlay = document.createElement("div");
  overlay.className = "presentation-overlay";
  if (exact) {
    // Sliden holdes innenfor vinduet, så hele er tegnet når PDF-siden lages. Størrelsen her betyr
    // ikke noe for PDF-en: siden skaleres til full HD etterpå.
    const fit = Math.min(1, window.innerWidth / EXPORT_WIDTH, window.innerHeight / EXPORT_HEIGHT);
    Object.assign(overlay.style, {
      inset: "auto",
      left: "0",
      top: "0",
      width: `${Math.floor(EXPORT_WIDTH * fit)}px`,
      height: `${Math.floor(EXPORT_HEIGHT * fit)}px`,
    });
  }
  overlay.innerHTML = `<div class="slides-container"><div class="reveal"><div class="slides"></div></div></div>`;
  const slidesEl = overlay.querySelector(".slides")!;
  for (const html of slides) {
    const section = document.createElement("section");
    setSafeHtml(section, html);
    slidesEl.append(section);
  }
  document.body.append(overlay);

  const reveal = overlay.querySelector<HTMLElement>(".reveal")!;
  applySlideStyle(reveal, style, { opaque: exact });
  // slides.css gir .slides-container samme bakgrunn, men leser fargen fra .reveal.
  overlay.querySelector<HTMLElement>(".slides-container")!.style.backgroundColor =
    reveal.style.getPropertyValue("--r-background-color");
  // I PDF-en tegnes graderingen av CSS (som vektor); støyen i dither.ts ville blitt et stort bilde per side.
  const removeBackground = exact ? () => {} : ditherBackground(reveal);
  const deck = new Reveal(reveal, {
    embedded: true,
    controlsTutorial: false,
    overview: false,
    hash: false,
    keyboard,
    // Uten tastene styres sliden fra talervisningen: ingen piler eller framdriftslinje,
    // som ellers ville blitt med når vinduet fanges i OBS.
    controls: keyboard,
    progress: keyboard,
    width: exact ? EXPORT_WIDTH : SLIDE_WIDTH,
    height: exact ? EXPORT_HEIGHT : SLIDE_HEIGHT,
    ...(exact ? { transition: "none" as const, backgroundTransition: "none" as const } : {}),
  });
  await deck.initialize();
  reveal.focus();

  return {
    slide: (index) => deck.slide(index),
    element: reveal,
    destroy: () => {
      deck.destroy();
      removeBackground();
      overlay.remove();
      for (const link of links) link.remove();
    },
  };
}

let running = false;

export async function present(slides: string[]) {
  if (running || slides.length === 0) return;
  running = true;

  const appWindow = getCurrentWindow();
  const wasFullscreen = await appWindow.isFullscreen().catch(() => false);
  const deckPromise = showDeck(slides, { keyboard: true });
  // Menylinjen (på Windows) skjules, så slidene fyller hele skjermen.
  await api.setMenuVisible(false).catch(() => {});
  if (!wasFullscreen) await appWindow.setFullscreen(true).catch(() => {});
  const deck = await deckPromise;

  const close = async () => {
    document.removeEventListener("keydown", onKey, true);
    deck.destroy();
    if (!wasFullscreen) await appWindow.setFullscreen(false).catch(() => {});
    await api.setMenuVisible(true).catch(() => {});
    running = false;
  };
  // Piltaster, mellomrom og PageUp/PageDown (fjernkontroller) blar i reveal.js. Esc avslutter.
  const onKey = (event: KeyboardEvent) => {
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      void close();
    }
  };
  document.addEventListener("keydown", onKey, true);
}
