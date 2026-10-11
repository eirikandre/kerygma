// Graderingen i bakgrunnen av presentasjonen, tegnet med dithering.
//
// Chromium (Obsidian) blander inn litt støy i CSS-graderinger, så de ser jevne ut.
// WebKit (Tauri på macOS) gjør det ikke: graderingen i slides.css har bare noen få
// fargetrinn over hele skjermen, og de ses som ringer. Her tegnes samme gradering i et
// canvas med samme matematikk som CSS, og hvert piksel rundes med litt tilfeldig støy.

/** `radial-gradient(circle at X% Y%, farge, transparent S%)` over en bakgrunnsfarge. */
interface RadialGradient {
  cx: number;
  cy: number;
  color: [number, number, number, number];
  stop: number;
  base: [number, number, number];
}

const NUMBER = String.raw`([0-9.]+)`;
const RGBA = String.raw`rgba?\(${NUMBER},\s*${NUMBER},\s*${NUMBER}(?:,\s*${NUMBER})?\)`;
const GRADIENT = new RegExp(
  String.raw`^radial-gradient\(circle at ${NUMBER}% ${NUMBER}%,\s*${RGBA},\s*rgba\(0,\s*0,\s*0,\s*0\)\s+${NUMBER}%\)$`,
);
const COLOR = new RegExp(`^${RGBA}$`);

/** Leser graderingen fra den beregnede stilen. Andre bakgrunner gir `undefined` og tegnes av CSS. */
function parse(style: CSSStyleDeclaration): RadialGradient | undefined {
  // Laget med bare farge i `background` gir `none` til slutt i lista over bilder.
  const gradient = GRADIENT.exec(style.backgroundImage.replace(/(,\s*none)+\s*$/, "").trim());
  const base = COLOR.exec(style.backgroundColor.trim());
  if (!gradient || !base || (base[4] !== undefined && Number(base[4]) !== 1)) return undefined;
  const n = gradient.slice(1).map((v) => (v === undefined ? 1 : Number(v)));
  return {
    cx: n[0] / 100,
    cy: n[1] / 100,
    color: [n[2], n[3], n[4], n[5]],
    stop: n[6] / 100,
    base: [Number(base[1]), Number(base[2]), Number(base[3])],
  };
}

function draw(canvas: HTMLCanvasElement, g: RadialGradient) {
  const scale = window.devicePixelRatio || 1;
  const width = Math.round(canvas.clientWidth * scale);
  const height = Math.round(canvas.clientHeight * scale);
  if (width === 0 || height === 0) return;
  canvas.width = width;
  canvas.height = height;
  const ctx = canvas.getContext("2d")!;
  const image = ctx.createImageData(width, height);
  const data = image.data;

  // «circle» uten størrelse er farthest-corner: radien går til hjørnet lengst unna.
  const cx = g.cx * width;
  const cy = g.cy * height;
  const radius = Math.hypot(Math.max(cx, width - cx), Math.max(cy, height - cy));
  const end = g.stop * radius;
  const [r0, g0, b0] = g.base;
  const [r1, g1, b1, a1] = g.color;

  for (let y = 0; y < height; y++) {
    const dy2 = (y + 0.5 - cy) ** 2;
    let i = y * width * 4;
    for (let x = 0; x < width; x++, i += 4) {
      const d = Math.sqrt((x + 0.5 - cx) ** 2 + dy2);
      // Mot transparent interpoleres med forhåndsmultiplisert alfa, så bare dekningen avtar.
      const alpha = d >= end ? 0 : a1 * (1 - d / end);
      // Dithering: støy på ±0,5 før avrunding, så trinnene blandes i stedet for å bli bånd.
      data[i] = r0 + alpha * (r1 - r0) + Math.random() - 0.5;
      data[i + 1] = g0 + alpha * (g1 - g0) + Math.random() - 0.5;
      data[i + 2] = b0 + alpha * (b1 - b0) + Math.random() - 0.5;
      data[i + 3] = 255;
    }
  }
  ctx.putImageData(image, 0, 0);
}

/**
 * Legger et canvas med graderingen nederst i `reveal`, under slidene og bakgrunnene
 * reveal.js lager. Gjør ingenting hvis bakgrunnen ikke er en slik gradering.
 * Returnerer en funksjon som fjerner canvaset.
 */
export function ditherBackground(reveal: HTMLElement): () => void {
  const gradient = parse(getComputedStyle(reveal));
  if (!gradient) return () => {};
  const canvas = document.createElement("canvas");
  canvas.className = "dithered-background";
  reveal.prepend(canvas);
  // Tegnes på nytt når størrelsen endres (overgangen til fullskjerm).
  let timer: number | undefined;
  let drawn = false;
  const observer = new ResizeObserver(() => {
    window.clearTimeout(timer);
    if (!drawn) {
      drawn = true;
      draw(canvas, gradient);
    } else {
      timer = window.setTimeout(() => draw(canvas, gradient), 100);
    }
  });
  observer.observe(canvas);
  return () => {
    window.clearTimeout(timer);
    observer.disconnect();
    canvas.remove();
  };
}
