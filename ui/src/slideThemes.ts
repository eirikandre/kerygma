// Temaer og skrifter for presentasjonen. slides.css gir temaet «Kveld»; de andre temaene,
// og skriftene som velges i innstillingene, legges oppå som variabler på `.reveal`.
import { FONTS } from "./appearance.ts";

export interface SlideTheme {
  id: string;
  name: string;
  desc: string;
  background: string;
  text: string;
  heading: string;
  /** Lyset bak teksten: `radial-gradient(...)` som i slides.css, eller ingenting. */
  glow?: { color: string; at: string };
  headingFont: string;
  quoteFont: string;
}

export interface SlideFont {
  id: string;
  name: string;
  stack: string;
}

/** Lora og Montserrat er bygget inn i slides.css og virker uten at de er installert. */
export const SLIDE_FONTS: SlideFont[] = [
  { id: "lora", name: "Lora (innebygd)", stack: '"Lora", Georgia, serif' },
  { id: "montserrat", name: "Montserrat (innebygd)", stack: '"Montserrat", Helvetica, sans-serif' },
  ...FONTS.filter((font) => font.id !== "mono"),
];

export const SLIDE_THEMES: SlideTheme[] = [
  {
    id: "kveld",
    name: "Kveld",
    desc: "Mørk blå med gylne overskrifter. Som slides.css i Obsidian.",
    background: "#1d2733",
    text: "#f4efe6",
    heading: "#e8c27a",
    glow: { color: "rgba(232, 194, 122, 0.12)", at: "30% 20%" },
    headingFont: "montserrat",
    quoteFont: "lora",
  },
  {
    id: "papir",
    name: "Papir",
    desc: "Lys og varm, som en trykt side. Passer i lyse rom.",
    background: "#f6f1e7",
    text: "#2d2823",
    heading: "#8c3a1b",
    glow: { color: "rgba(255, 255, 255, 0.7)", at: "50% 35%" },
    headingFont: "lora",
    quoteFont: "lora",
  },
  {
    id: "kontrast",
    name: "Kontrast",
    desc: "Hvit tekst på svart. Lettest å lese på svake prosjektorer og i strømming.",
    background: "#000000",
    text: "#ffffff",
    heading: "#ffd166",
    headingFont: "montserrat",
    quoteFont: "helvetica",
  },
  {
    id: "skog",
    name: "Skog",
    desc: "Dyp grønn med lysegrønne overskrifter.",
    background: "#1c2a24",
    text: "#eef1e8",
    heading: "#b5d69a",
    glow: { color: "rgba(181, 214, 154, 0.1)", at: "70% 20%" },
    headingFont: "montserrat",
    quoteFont: "palatino",
  },
];

export function slideTheme(id: string): SlideTheme {
  return SLIDE_THEMES.find((theme) => theme.id === id) ?? SLIDE_THEMES[0];
}

function fontStack(id: string, fallback: string): string {
  return (SLIDE_FONTS.find((font) => font.id === id) ?? SLIDE_FONTS.find((font) => font.id === fallback)!).stack;
}

/** Tema og skrifter, med tom skrift som temaets. */
export interface SlideStyle {
  theme: string;
  headingFont: string;
  quoteFont: string;
}

/** Fargene og skriftene (CSS font-family) temaet og skriftvalgene gir, til PowerPoint. */
export function resolveSlideStyle(style: SlideStyle) {
  const theme = slideTheme(style.theme);
  return {
    background: theme.background,
    text: theme.text,
    heading: theme.heading,
    headingFont: fontStack(style.headingFont, theme.headingFont),
    quoteFont: fontStack(style.quoteFont, theme.quoteFont),
  };
}

/**
 * Setter temaet på elementet (`.reveal`, eller forhåndsvisningen i innstillingene).
 * Variablene er de reveal.js og slides.css bruker.
 */
export function applySlideStyle(el: HTMLElement, style: SlideStyle, { opaque = false }: { opaque?: boolean } = {}) {
  const theme = slideTheme(style.theme);
  const css = el.style;
  css.setProperty("--r-background-color", theme.background);
  css.setProperty("--r-main-color", theme.text);
  css.setProperty("--r-heading-color", theme.heading);
  css.setProperty("--r-main-font", fontStack(style.quoteFont, theme.quoteFont));
  css.setProperty("--r-heading-font", fontStack(style.headingFont, theme.headingFont));
  // Samme form som i slides.css, så dither.ts kjenner den igjen. `opaque` (PDF): samme gradering
  // uten gjennomsiktighet, som PDF-en ellers mister (det gjennomsiktige blir svart).
  css.background = !theme.glow
    ? theme.background
    : opaque
      ? `radial-gradient(circle at ${theme.glow.at}, ${blend(theme.glow.color, theme.background)}, ${theme.background} 60%)`
      : `radial-gradient(circle at ${theme.glow.at}, ${theme.glow.color}, transparent 60%), ${theme.background}`;
}

let fontsLoaded: Promise<void> | undefined;

/**
 * Lora og Montserrat til forhåndsvisningen i innstillingene. @font-face-reglene hentes
 * ut av slides.css, så resten av den fila ikke påvirker appen.
 */
export function loadSlideFonts(): Promise<void> {
  fontsLoaded ??= fetch("/slides.css")
    .then((response) => response.text())
    .then((css) => {
      const style = document.createElement("style");
      style.textContent = (css.match(/@font-face\s*\{[^}]*\}/g) ?? []).join("\n");
      document.head.append(style);
    })
    .catch((error) => console.error(error));
  return fontsLoaded;
}

/**
 * `rgba(...)` lagt over bakgrunnsfargen (`#rrggbb`), som en farge uten gjennomsiktighet. En
 * gradering fra den til bakgrunnen er lik en fra `rgba(...)` til `transparent` over bakgrunnen.
 */
function blend(rgba: string, background: string): string {
  const [r, g, b, a = 1] = (rgba.match(/[0-9.]+/g) ?? []).map(Number);
  const base = [1, 3, 5].map((i) => Number.parseInt(background.slice(i, i + 2), 16));
  const mix = [r, g, b].map((c, i) => Math.round(c * a + base[i] * (1 - a)));
  return `rgb(${mix.join(", ")})`;
}
