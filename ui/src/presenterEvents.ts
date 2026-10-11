// Det hovedvinduet og vinduet med slidene i talervisningen sender til hverandre.

export const PRESENTER_EVENTS = {
  /** Slidevinduet er lastet og venter på slidene (til hovedvinduet). */
  ready: "presenter:ready",
  /** Slidene og hvilken som vises (til slidevinduet). */
  init: "presenter:init",
  /** Nummeret på sliden som skal vises (til slidevinduet). */
  slide: "presenter:slide",
  /** En tast i slidevinduet (til hovedvinduet). */
  key: "presenter:key",
} as const;

export interface PresenterInit {
  slides: string[];
  index: number;
}

export type PresenterKey = "next" | "previous" | "down" | "up" | "stop";

/**
 * Tastene som styrer talervisningen: høyre/venstre pil og PageDown/PageUp (det de fleste
 * fjernkontroller sender) gir neste og forrige slide, pil ned og opp ruller manuset, og Esc
 * avslutter. Rullingen bytter slide som vanlig når neste del når leselinja.
 */
export function presenterKey(event: KeyboardEvent): PresenterKey | undefined {
  if (event.metaKey || event.ctrlKey || event.altKey) return undefined;
  switch (event.key) {
    case "ArrowRight":
    case "PageDown":
      return "next";
    case "ArrowLeft":
    case "PageUp":
      return "previous";
    case "ArrowDown":
      return "down";
    case "ArrowUp":
      return "up";
    case "Escape":
      return "stop";
    default:
      return undefined;
  }
}
