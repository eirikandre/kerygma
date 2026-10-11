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
  destroy(): void;
}

/**
 * Slidene over hele vinduet, med temaet fra innstillingene. `keyboard`: piltaster,
 * mellomrom og PageUp/PageDown blar i reveal.js; uten styres sliden bare med `slide`.
 */
export async function showDeck(slides: string[], { keyboard }: { keyboard: boolean }): Promise<Deck> {
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
  overlay.innerHTML = `<div class="slides-container"><div class="reveal"><div class="slides"></div></div></div>`;
  const slidesEl = overlay.querySelector(".slides")!;
  for (const html of slides) {
    const section = document.createElement("section");
    setSafeHtml(section, html);
    slidesEl.append(section);
  }
  document.body.append(overlay);

  const reveal = overlay.querySelector<HTMLElement>(".reveal")!;
  applySlideStyle(reveal, style);
  // slides.css gir .slides-container samme bakgrunn, men leser fargen fra .reveal.
  overlay.querySelector<HTMLElement>(".slides-container")!.style.backgroundColor =
    reveal.style.getPropertyValue("--r-background-color");
  const removeBackground = ditherBackground(reveal);
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
    width: 960,
    height: 700,
  });
  await deck.initialize();
  reveal.focus();

  return {
    slide: (index) => deck.slide(index),
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
  if (!wasFullscreen) await appWindow.setFullscreen(true).catch(() => {});
  const deck = await deckPromise;

  const close = async () => {
    document.removeEventListener("keydown", onKey, true);
    deck.destroy();
    if (!wasFullscreen) await appWindow.setFullscreen(false).catch(() => {});
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
