// «Vis presentasjon»: samme DOM og CSS som Slides i Obsidian, i fullskjerm.
import { getCurrentWindow } from "@tauri-apps/api/window";
import Reveal from "reveal.js";
import revealCss from "reveal.js/dist/reveal.css?url";
import blackCss from "reveal.js/dist/theme/black.css?url";
import { api } from "./api.ts";
import { ditherBackground } from "./dither.ts";
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

let running = false;

export async function present(slides: string[]) {
  if (running || slides.length === 0) return;
  running = true;
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
    section.innerHTML = html;
    slidesEl.append(section);
  }
  document.body.append(overlay);

  const appWindow = getCurrentWindow();
  const wasFullscreen = await appWindow.isFullscreen().catch(() => false);
  if (!wasFullscreen) await appWindow.setFullscreen(true).catch(() => {});

  const reveal = overlay.querySelector<HTMLElement>(".reveal")!;
  applySlideStyle(reveal, style);
  // slides.css gir .slides-container samme bakgrunn, men leser fargen fra .reveal.
  overlay.querySelector<HTMLElement>(".slides-container")!.style.backgroundColor =
    reveal.style.getPropertyValue("--r-background-color");
  const removeBackground = ditherBackground(reveal);
  const deck = new Reveal(overlay.querySelector<HTMLElement>(".reveal")!, {
    embedded: true,
    controlsTutorial: false,
    overview: false,
    hash: false,
    width: 960,
    height: 700,
  });
  await deck.initialize();
  overlay.querySelector<HTMLElement>(".reveal")!.focus();

  const close = async () => {
    document.removeEventListener("keydown", onKey, true);
    deck.destroy();
    removeBackground();
    overlay.remove();
    for (const link of links) link.remove();
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
