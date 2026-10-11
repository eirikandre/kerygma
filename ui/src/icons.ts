// Ikoner til knappene, fra Lucide (https://lucide.dev, ISC-lisens), og hover-teksten.

const PATHS = {
  eye: '<path d="M2.062 12.348a1 1 0 0 1 0-.696 10.75 10.75 0 0 1 19.876 0 1 1 0 0 1 0 .696 10.75 10.75 0 0 1-19.876 0"/><circle cx="12" cy="12" r="3"/>',
  pencil:
    '<path d="M21.174 6.812a1 1 0 0 0-3.986-3.987L3.842 16.174a2 2 0 0 0-.5.83l-1.321 4.352a.5.5 0 0 0 .623.622l4.353-1.32a2 2 0 0 0 .83-.497z"/><path d="m15 5 4 4"/>',
  "book-open":
    '<path d="M12 7v14"/><path d="M3 18a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1h5a4 4 0 0 1 4 4 4 4 0 0 1 4-4h5a1 1 0 0 1 1 1v13a1 1 0 0 1-1 1h-6a3 3 0 0 0-3 3 3 3 0 0 0-3-3z"/>',
  presentation: '<path d="M2 3h20"/><path d="M21 3v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V3"/><path d="m7 21 5-5 5 5"/>',
  play: '<polygon points="6 3 20 12 6 21 6 3"/>',
  search: '<circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/>',
  x: '<path d="M18 6 6 18"/><path d="m6 6 12 12"/>',
  "chevron-left": '<path d="m15 18-6-6 6-6"/>',
  "chevron-right": '<path d="m9 18 6-6-6-6"/>',
  bold: '<path d="M6 12h9a4 4 0 0 1 0 8H7a1 1 0 0 1-1-1V5a1 1 0 0 1 1-1h7a4 4 0 0 1 0 8"/>',
  italic: '<line x1="19" x2="10" y1="4" y2="4"/><line x1="14" x2="5" y1="20" y2="20"/><line x1="15" x2="9" y1="4" y2="20"/>',
  underline: '<path d="M6 4v6a6 6 0 0 0 12 0V4"/><line x1="4" x2="20" y1="20" y2="20"/>',
  strikethrough:
    '<path d="M16 4H9a3 3 0 0 0-2.83 4"/><path d="M14 12a4 4 0 0 1 0 8H6"/><line x1="4" x2="20" y1="12" y2="12"/>',
  quote:
    '<path d="M16 3a2 2 0 0 0-2 2v6a2 2 0 0 0 2 2 1 1 0 0 1 1 1v1a2 2 0 0 1-2 2 1 1 0 0 0-1 1v2a1 1 0 0 0 1 1 6 6 0 0 0 6-6V5a2 2 0 0 0-2-2z"/><path d="M5 3a2 2 0 0 0-2 2v6a2 2 0 0 0 2 2 1 1 0 0 1 1 1v1a2 2 0 0 1-2 2 1 1 0 0 0-1 1v2a1 1 0 0 0 1 1 6 6 0 0 0 6-6V5a2 2 0 0 0-2-2z"/>',
  "file-plus":
    '<path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z"/><path d="M14 2v4a2 2 0 0 0 2 2h4"/><path d="M9 15h6"/><path d="M12 18v-6"/>',
  "folder-plus":
    '<path d="M12 10v6"/><path d="M9 13h6"/><path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z"/>',
};

export type IconName = keyof typeof PATHS;

/** Ikonet som SVG, i tekstfargen. */
export function icon(name: IconName): string {
  return (
    '<svg class="icon" viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" ' +
    `stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${PATHS[name]}</svg>`
  );
}

/** Gir knappen et ikon og en hover-tekst. Hover-teksten er også navnet skjermlesere leser. */
export function setIcon(button: HTMLElement, name: IconName, label: string) {
  button.innerHTML = icon(name);
  button.dataset.tooltip = label;
  button.setAttribute("aria-label", label.replace(/ \(.*\)$/, ""));
}

const TOOLTIP_DELAY_MS = 400;

/**
 * Hover-tekst for alle elementer med `data-tooltip`. Én boks som plasseres under
 * elementet og holdes innenfor vinduet, så knappene helt til høyre også får den.
 */
export function initTooltips() {
  const tip = document.createElement("div");
  tip.className = "tooltip";
  tip.setAttribute("role", "tooltip");
  tip.hidden = true;
  document.body.append(tip);
  let timer: number | undefined;
  let target: HTMLElement | undefined;

  const hide = () => {
    window.clearTimeout(timer);
    target = undefined;
    tip.hidden = true;
  };
  const show = (el: HTMLElement) => {
    tip.textContent = el.dataset.tooltip ?? "";
    tip.hidden = false;
    const rect = el.getBoundingClientRect();
    const width = tip.offsetWidth;
    const left = Math.max(4, Math.min(rect.left + rect.width / 2 - width / 2, window.innerWidth - width - 4));
    const below = rect.bottom + 6;
    const top = below + tip.offsetHeight > window.innerHeight ? rect.top - tip.offsetHeight - 6 : below;
    tip.style.left = `${left}px`;
    tip.style.top = `${top}px`;
  };

  document.addEventListener("mouseover", (event) => {
    const el = (event.target as HTMLElement).closest<HTMLElement>("[data-tooltip]");
    if (el === target) return;
    hide();
    if (!el) return;
    target = el;
    timer = window.setTimeout(() => show(el), TOOLTIP_DELAY_MS);
  });
  document.addEventListener("mousedown", hide, true);
  document.addEventListener("keydown", hide, true);
  window.addEventListener("blur", hide);
}
