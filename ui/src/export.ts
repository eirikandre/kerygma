// Eksport av presentasjonen til PDF, PowerPoint og HTML.
//
// PDF: hver slide vises med reveal.js i 16:9, slik den ser ut i appen, og vinduet lager en PDF-side
// av den (`createPDF` i WebKit, se src-tauri/src/capture.rs). Teksten blir tekst, med de samme
// skriftene. Sidene slås sammen og skaleres til full HD (1920 × 1080) i kerygma-core.
//
// PowerPoint: slidene gjøres om til tekstbokser og bilder med pptxgenjs, med farger og skrift
// fra temaet, så teksten kan rettes. Det ser omtrent ut som i appen, ikke nøyaktig likt.
//
// HTML: én fil med reveal.js, slides.css, temaet og bildene bygget inn, så den kan åpnes i en
// nettleser uten appen og uten nett. Slidene er de samme som i appen.
import { invoke } from "@tauri-apps/api/core";
import PptxGenJS from "pptxgenjs";
import revealCss from "reveal.js/dist/reveal.css?raw";
import revealJs from "reveal.js/dist/reveal.js?raw";
import blackCss from "reveal.js/dist/theme/black.css?raw";
import { api, basename } from "./api.ts";
import { SLIDE_HEIGHT, SLIDE_WIDTH, showDeck } from "./present.ts";
import { setSafeHtml } from "./sanitize.ts";
import { applySlideStyle, resolveSlideStyle, type SlideStyle } from "./slideThemes.ts";

export type ExportKind = "pdf" | "pptx" | "html";

// --- PDF -------------------------------------------------------------------------

/** Lager PDF-en og lagrer den på `path`. */
export async function exportPdf(slides: string[], path: string) {
  // WebKit tar med alt som ligger under sliden i PDF-en (også det som er skjult bak), så appen
  // skjules mens den lages. Hvor langt eksporten har kommet, står til høyre for sliden.
  const hidden = [document.getElementById("app"), document.getElementById("notices")].filter((el) => !!el);
  for (const el of hidden) el.style.visibility = "hidden";
  const backdrop = document.createElement("div");
  backdrop.className = "export-backdrop";
  const status = document.createElement("p");
  backdrop.append(status);
  document.body.append(backdrop);
  const deck = await showDeck(slides, { keyboard: false, exact: true });
  // Teksten står til høyre for eller under sliden, så den ikke kommer med i PDF-en.
  const area = deck.element.getBoundingClientRect();
  Object.assign(status.style, window.innerWidth - area.right > 200 ? { left: `${area.right + 24}px`, top: "24px" } : { left: "24px", top: `${area.bottom + 16}px` });
  // Sidene samles i Rust under id-en til eksporten; går noe galt, glemmes de.
  let id: number | undefined;
  try {
    const exportId = await api.pdfBegin();
    id = exportId;
    for (let i = 0; i < slides.length; i++) {
      status.textContent = `Lager PDF … slide ${i + 1} av ${slides.length}`;
      deck.slide(i);
      await ready(deck.element);
      const rect = deck.element.getBoundingClientRect();
      await api.pdfPage(exportId, { x: rect.left, y: rect.top, width: rect.width, height: rect.height });
    }
    status.textContent = "Lagrer PDF-en …";
    await api.pdfSave(exportId, path);
    id = undefined;
  } finally {
    if (id !== undefined) void api.pdfCancel(id);
    deck.destroy();
    backdrop.remove();
    for (const el of hidden) el.style.visibility = "";
  }
}

/** Venter til skriftene og bildene på sliden som vises, er lastet, og den er tegnet. */
async function ready(reveal: HTMLElement) {
  await document.fonts.ready;
  const images = [...reveal.querySelectorAll<HTMLImageElement>("section.present img")];
  await Promise.all(images.map((img) => (img.complete ? undefined : img.decode().catch(() => undefined))));
  await new Promise((resolve) => setTimeout(resolve, 50));
}

// --- PowerPoint --------------------------------------------------------------------

/** Lager PowerPoint-fila og lagrer den på `path`. */
export async function exportPptx(slides: string[], path: string, title: string) {
  const settings = await api.getSettings();
  const style = resolveSlideStyle({
    theme: settings.slideTheme,
    headingFont: settings.headingFont,
    quoteFont: settings.quoteFont,
  } satisfies SlideStyle);
  const data = await buildPptx(slides.map(parseSlide), style, title);
  await saveExport(data, path);
}

/** Lagrer innholdet i fila på `path` (valgt av brukeren). */
async function saveExport(data: Uint8Array, path: string) {
  await invoke("save_export", data, { headers: { path: encodeURIComponent(path) } });
}

/** Tekst med formatering, som i en tekstboks i PowerPoint. */
interface Run {
  text: string;
  bold?: boolean;
  italic?: boolean;
  underline?: boolean;
  strike?: boolean;
  superscript?: boolean;
  highlight?: boolean;
  /** Linjeskift etter teksten (`<br>`). */
  lineBreak?: boolean;
}

type Block =
  | { kind: "heading"; level: number; runs: Run[] }
  | { kind: "paragraph"; runs: Run[]; bullet?: boolean }
  | { kind: "columns"; columns: Run[][] }
  | { kind: "image"; src: string };

/** Innholdet på en slide, fra HTML-en som vises i appen. */
export function parseSlide(html: string): Block[] {
  const section = document.createElement("section");
  setSafeHtml(section, html);
  const blocks: Block[] = [];
  collectBlocks(section, blocks);
  return blocks;
}

function collectBlocks(parent: Element, blocks: Block[]) {
  for (const el of parent.children) {
    const tag = el.tagName;
    if (/^H[1-6]$/.test(tag)) {
      blocks.push({ kind: "heading", level: Number(tag[1]), runs: inlineRuns(el) });
    } else if (tag === "P") {
      paragraph(el, blocks);
    } else if (el.classList.contains("bibel-parallell")) {
      blocks.push({ kind: "columns", columns: [...el.children].map((column) => inlineRuns(column)) });
    } else if (tag === "UL" || tag === "OL") {
      for (const item of el.children) blocks.push({ kind: "paragraph", runs: inlineRuns(item), bullet: true });
    } else if (tag === "IMG") {
      blocks.push({ kind: "image", src: (el as HTMLImageElement).src });
    } else if (tag === "PRE") {
      blocks.push({ kind: "paragraph", runs: [{ text: el.textContent ?? "" }] });
    } else if (tag === "TABLE") {
      for (const row of el.querySelectorAll("tr")) {
        const cells = [...row.children].map((cell) => cell.textContent?.trim() ?? "");
        blocks.push({ kind: "paragraph", runs: [{ text: cells.join("   ") }] });
      }
    } else if (tag !== "HR" && tag !== "STYLE") {
      // Sitater, callouts og andre beholdere: innholdet hver for seg.
      collectBlocks(el, blocks);
    }
  }
}

/** Et avsnitt. Bilder i avsnittet blir egne blokker (et avsnitt med bare et bilde er en bildeslide). */
function paragraph(p: Element, blocks: Block[]) {
  const images = [...p.querySelectorAll("img")];
  for (const img of images) {
    blocks.push({ kind: "image", src: img.src });
    (img.closest(".image-embed") ?? img).remove();
  }
  const runs = inlineRuns(p);
  if (runs.some((run) => run.text.trim() !== "")) {
    // Bildene kommer etter teksten i den rekkefølgen de ble funnet; teksten først.
    blocks.splice(blocks.length - images.length, 0, { kind: "paragraph", runs });
  }
}

/** Teksten i elementet som formaterte biter, med mellomrom slått sammen som i HTML. */
function inlineRuns(el: Element): Run[] {
  const runs: Run[] = [];
  const walk = (node: Node, format: Omit<Run, "text">) => {
    if (node.nodeType === Node.TEXT_NODE) {
      const text = (node.nodeValue ?? "").replace(/\s+/g, " ");
      if (text) runs.push({ text, ...format });
      return;
    }
    if (!(node instanceof Element)) return;
    const tag = node.tagName;
    if (tag === "BR") {
      if (runs.length > 0) runs[runs.length - 1].lineBreak = true;
      return;
    }
    const next = { ...format };
    if (tag === "STRONG" || tag === "B") next.bold = true;
    if (tag === "EM" || tag === "I") next.italic = true;
    if (tag === "U") next.underline = true;
    if (tag === "DEL" || tag === "S") next.strike = true;
    if (tag === "SUP") next.superscript = true;
    if (tag === "MARK") next.highlight = true;
    for (const child of node.childNodes) walk(child, next);
  };
  for (const child of el.childNodes) walk(child, {});
  // Mellomrom først og sist i avsnittet, og etter linjeskift, vises ikke.
  if (runs.length > 0) {
    runs[0].text = runs[0].text.trimStart();
    runs[runs.length - 1].text = runs[runs.length - 1].text.trimEnd();
  }
  runs.forEach((run, i) => {
    if (i > 0 && runs[i - 1].lineBreak) run.text = run.text.trimStart();
    if (run.lineBreak) run.text = run.text.trimEnd();
  });
  return runs.filter((run) => run.text !== "" || run.lineBreak);
}

// Bredformat (16:9), som PowerPoint lager bilder av i full HD: 13,33 × 7,5 tommer. Med 96 piksler
// per tomme er det 1280 × 720 piksler, samme mål som PDF-en lages i, så skriftstørrelsene fra
// slides.css (i piksler) gjelder som de er.
const WIDTH = 1280 / 96;
const HEIGHT = 720 / 96;
/** reveal.js har 4 % marg rundt innholdet. */
const MARGIN_X = WIDTH * 0.04;
const MARGIN_Y = HEIGHT * 0.04;
/** Skriftstørrelser i punkter, fra slides.css: 44 px tekst, overskrifter 1,8 / 1,4 / 1,15 em. */
const BODY_PT = 44 * 0.75;
const HEADING_SCALE = [1.8, 1.4, 1.15, 1, 1, 1];
/** Kolonner med oversettelser har 0,75 em, og 1,5 em mellom. */
const COLUMN_SCALE = 0.75;
/** Omtrent hvor bredt et tegn er, i forhold til skriftstørrelsen. */
const CHAR_WIDTH = 0.5;
const PT_PER_INCH = 72;

interface Style {
  background: string;
  text: string;
  heading: string;
  headingFont: string;
  quoteFont: string;
}

/** Fargen uten `#`, som PowerPoint vil ha den. */
function hex(color: string): string {
  return color.replace(/^#/, "").toUpperCase();
}

/** Det første skriftnavnet i en CSS font-family som finnes utenfor nettleseren. */
function fontName(stack: string): string {
  const names = stack.split(",").map((name) => name.trim().replace(/^["']|["']$/g, ""));
  return names.find((name) => !name.startsWith("-") && !/^(BlinkMacSystemFont|system-ui|serif|sans-serif|monospace)$/.test(name)) ?? "Georgia";
}

/** Omtrent hvor høy teksten blir (i tommer) i en boks som er `width` tommer bred. */
function textHeight(runs: Run[], fontPt: number, lineHeight: number, width: number): number {
  const lines = runs
    .map((run) => run.text + (run.lineBreak ? "\n" : ""))
    .join("")
    .split("\n")
    .reduce((sum, line) => sum + Math.max(1, Math.ceil((line.length * CHAR_WIDTH * fontPt) / PT_PER_INCH / width)), 0);
  return (lines * fontPt * lineHeight) / PT_PER_INCH;
}

/** Bildet som data-URL og størrelsen, så det kan legges inn i fila. */
async function loadImage(src: string): Promise<{ data: string; width: number; height: number } | undefined> {
  try {
    const blob = await (await fetch(src)).blob();
    const bitmap = await createImageBitmap(blob);
    const data = await readDataUrl(blob);
    return { data: data.replace(/^data:/, ""), width: bitmap.width, height: bitmap.height };
  } catch {
    return undefined;
  }
}

async function buildPptx(slides: Block[][], style: Style, title: string): Promise<Uint8Array> {
  const pptx = new PptxGenJS();
  pptx.defineLayout({ name: "KERYGMA", width: WIDTH, height: HEIGHT });
  pptx.layout = "KERYGMA";
  pptx.title = title;
  const headingFont = fontName(style.headingFont);
  const bodyFont = fontName(style.quoteFont);

  for (const blocks of slides) {
    const slide = pptx.addSlide();
    slide.background = { color: hex(style.background) };
    const images = blocks.filter((block) => block.kind === "image");
    const hasText = blocks.some((block) => block.kind !== "image");

    // En slide med bare et bilde: bildet fyller sliden (hele bildet vises), som i slides.css.
    if (!hasText && images.length === 1) {
      const image = await loadImage(images[0].src);
      // Litt luft rundt, som `padding: 4%` (av bredden) for bildeslides i slides.css.
      const air = WIDTH * 0.04;
      if (image) addContained(slide, image, air, air, WIDTH - 2 * air, HEIGHT - 2 * air);
      continue;
    }

    // Uten overskrift er det et vers: kursiv, midtstilt og litt smalere, som i slides.css.
    const verse = !blocks.some((block) => block.kind === "heading");
    const width = WIDTH - 2 * MARGIN_X;
    const textWidth = verse ? width * 0.85 : width;
    const lineHeight = verse ? 1.5 : 1.3;

    // Høyden på hver del ved full størrelse; er det for mye, gjøres all teksten mindre.
    const parts = blocks.map((block) => {
      switch (block.kind) {
        case "heading": {
          const size = BODY_PT * HEADING_SCALE[block.level - 1];
          return { block, height: textHeight(block.runs, size, 1.2, width) + 0.15 };
        }
        case "paragraph":
          return { block, height: textHeight(block.runs, BODY_PT, lineHeight, textWidth) + 0.1 };
        case "columns": {
          const columnWidth = (width * 0.95 - 0.25 * (block.columns.length - 1)) / block.columns.length;
          const size = BODY_PT * COLUMN_SCALE;
          return { block, height: Math.max(...block.columns.map((runs) => textHeight(runs, size, 1.5, columnWidth))) };
        }
        case "image":
          return { block, height: HEIGHT * 0.45 };
      }
    });
    const available = HEIGHT - 2 * MARGIN_Y;
    const total = parts.reduce((sum, part) => sum + part.height, 0);
    const scale = Math.max(0.45, Math.min(1, available / total));

    // Innholdet er midtstilt i høyden, som i reveal.js.
    let y = MARGIN_Y + Math.max(0, (available - total * scale) / 2);
    for (const { block, height } of parts) {
      const h = height * scale;
      if (block.kind === "heading") {
        slide.addText(runsToText(block.runs, { size: BODY_PT * HEADING_SCALE[block.level - 1] * scale, bold: true, superscriptFont: headingFont }), {
          x: MARGIN_X, y, w: width, h,
          fontFace: headingFont,
          fontSize: BODY_PT * HEADING_SCALE[block.level - 1] * scale,
          color: hex(style.heading),
          bold: true,
          align: "center",
          valign: "middle",
          margin: 0,
          fit: "shrink",
        });
      } else if (block.kind === "paragraph") {
        slide.addText(runsToText(block.runs, { size: BODY_PT * scale, italic: verse, superscriptFont: headingFont, bullet: block.bullet }), {
          x: (WIDTH - textWidth) / 2, y, w: textWidth, h,
          fontFace: bodyFont,
          fontSize: BODY_PT * scale,
          color: hex(style.text),
          italic: verse,
          align: block.bullet ? "left" : "center",
          valign: "middle",
          lineSpacingMultiple: lineHeight,
          margin: 0,
          fit: "shrink",
        });
      } else if (block.kind === "columns") {
        // Kolonnene står side om side med en strek mellom, og oversettelsen litt svakere.
        const n = block.columns.length;
        const gap = 0.25;
        const columnsWidth = width * 0.95;
        const columnWidth = (columnsWidth - gap * (n - 1)) / n;
        const left = (WIDTH - columnsWidth) / 2;
        block.columns.forEach((runs, i) => {
          const x = left + i * (columnWidth + gap);
          if (i > 0) {
            slide.addShape(pptx.ShapeType.line, {
              x: x - gap / 2, y, w: 0, h,
              line: { color: hex(style.text), width: 0.75, transparency: 40 },
            });
          }
          slide.addText(runsToText(runs, { size: BODY_PT * COLUMN_SCALE * scale, italic: true, superscriptFont: headingFont }), {
            x, y, w: columnWidth, h,
            fontFace: bodyFont,
            fontSize: BODY_PT * COLUMN_SCALE * scale,
            color: hex(style.text),
            italic: true,
            transparency: i > 0 ? 15 : 0,
            align: "left",
            valign: "middle",
            lineSpacingMultiple: 1.5,
            margin: 0,
            fit: "shrink",
          });
        });
      } else {
        const image = await loadImage(block.src);
        if (image) addContained(slide, image, MARGIN_X, y, width, h);
      }
      y += h;
    }
  }
  return (await pptx.write({ outputType: "uint8array" })) as Uint8Array;
}

/** Bildet så stort det kan bli innenfor rammen, uten å strekkes, midt i den. */
function addContained(
  slide: PptxGenJS.Slide,
  image: { data: string; width: number; height: number },
  x: number,
  y: number,
  w: number,
  h: number,
) {
  const ratio = Math.min(w / image.width, h / image.height);
  const width = image.width * ratio;
  const height = image.height * ratio;
  slide.addImage({ data: image.data, x: x + (w - width) / 2, y: y + (h - height) / 2, w: width, h: height });
}

/** Bitene som tekst til pptxgenjs, med formateringen. Versnumre blir små og hevet, som i slides.css. */
function runsToText(
  runs: Run[],
  base: { size: number; bold?: boolean; italic?: boolean; superscriptFont: string; bullet?: boolean },
): PptxGenJS.TextProps[] {
  return runs.map((run, i) => ({
    text: run.text,
    options: {
      // Versnumre: halv størrelse, ikke kursiv og litt svakere, som i slides.css. Størrelsen settes
      // selv, siden ikke alle programmer gjør hevet skrift mindre.
      bold: run.superscript ? false : run.bold || base.bold,
      italic: run.superscript ? false : run.italic || base.italic,
      fontSize: run.superscript ? base.size * 0.5 : undefined,
      transparency: run.superscript ? 30 : undefined,
      underline: run.underline ? { style: "sng" as const } : undefined,
      strike: run.strike ? ("sngStrike" as const) : undefined,
      superscript: run.superscript || undefined,
      fontFace: run.superscript ? base.superscriptFont : undefined,
      highlight: run.highlight ? "FFF3A3" : undefined,
      breakLine: run.lineBreak || undefined,
      bullet: i === 0 && base.bullet ? true : undefined,
    },
  }));
}

// --- HTML --------------------------------------------------------------------------

/** Lager HTML-fila og lagrer den på `path`. */
export async function exportHtml(slides: string[], path: string, title: string) {
  const settings = await api.getSettings();
  const style: SlideStyle = { theme: settings.slideTheme, headingFont: settings.headingFont, quoteFont: settings.quoteFont };
  const slidesCss = await (await fetch("/slides.css")).text();
  const sections = await Promise.all(slides.map(htmlSection));
  // Temaet settes på `.reveal` som i appen, og tas med som style-attributtet.
  const reveal = document.createElement("div");
  applySlideStyle(reveal, style);
  const html = `<!doctype html>
<html lang="nb">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>${escapeHtml(title)}</title>
<style>${inline(revealCss)}</style>
<style>${inline(withoutImports(blackCss))}</style>
<style>${inline(slidesCss)}</style>
<style>html, body, .slides-container { height: 100%; margin: 0; }</style>
</head>
<body>
<div class="slides-container" style="background-color: ${escapeHtml(reveal.style.getPropertyValue("--r-background-color"))}">
<div class="reveal" style="${escapeHtml(reveal.getAttribute("style") ?? "")}">
<div class="slides">
${sections.join("\n")}
</div>
</div>
</div>
<script>${inline(revealJs)}</script>
<script>Reveal.initialize({ hash: true, controlsTutorial: false, width: ${SLIDE_WIDTH}, height: ${SLIDE_HEIGHT} });</script>
</body>
</html>
`;
  await saveExport(new TextEncoder().encode(html), path);
}

/** Sliden som `<section>`, uten skript og med bildene bygget inn som data-URL-er. */
async function htmlSection(html: string): Promise<string> {
  const section = document.createElement("section");
  setSafeHtml(section, html);
  await Promise.all(
    [...section.querySelectorAll("img")].map(async (img) => {
      const data = await dataUrl(img.src);
      if (data) img.src = data;
      else img.remove();
    }),
  );
  return section.outerHTML;
}

/** Bildet som data-URL, så HTML-fila virker uten vaulten. */
async function dataUrl(src: string): Promise<string | undefined> {
  if (src.startsWith("data:")) return src;
  try {
    return await readDataUrl(await (await fetch(src)).blob());
  } catch {
    return undefined;
  }
}

function readDataUrl(blob: Blob): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result));
    reader.onerror = () => reject(reader.error);
    reader.readAsDataURL(blob);
  });
}

/** Skript og stiler inne i `<script>`/`<style>` kan ikke inneholde `</script` eller `</style`. */
function inline(code: string): string {
  return code.replace(/<\/(script|style)/gi, "<\\/$1");
}

/** `@import` av skrifter fra temaet i reveal.js: filene er ikke med, og skriftene settes av slides.css. */
function withoutImports(css: string): string {
  return css.replace(/@import\s+url\([^)]*\)\s*;?/g, "");
}

function escapeHtml(text: string): string {
  return text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
}

/** Navnet på fila som foreslås: presentasjonen med ny filtype. */
export function exportFileName(presentation: string, extension: ExportKind): string {
  return `${basename(presentation)}.${extension}`;
}
