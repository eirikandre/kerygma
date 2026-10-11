// Bygger fontene i ui/fonts/ inn i ui/public/slides.css som base64. Da er slides.css én fil
// uten avhengigheter: presentasjonen, PDF-en og HTML-eksporten (som bygger slides.css inn)
// har fontene uten nett og uten å peke på andre filer.
//
//   node scripts/embed-fonts.mjs        (eller: cd ui && npm run fonts)
//
// Lisensene (SIL Open Font License 1.1) ligger i ui/public/lisenser/ og følger med appen.
import { readFileSync, writeFileSync } from "node:fs";

const FONTS = [
  { family: "Lora", file: "ui/fonts/Lora.woff2", style: "normal", weight: "400 700" },
  { family: "Lora", file: "ui/fonts/Lora-Italic.woff2", style: "italic", weight: "400 700" },
  { family: "Montserrat", file: "ui/fonts/Montserrat.woff2", style: "normal", weight: "100 900" },
];

const START = "/* === Fonter: generert av scripts/embed-fonts.mjs, ikke rediger === */";
const END = "/* === Slutt på fonter === */";

const faces = FONTS.map(({ family, file, style, weight }) => {
  const data = readFileSync(file).toString("base64");
  return `@font-face {
  font-family: "${family}";
  font-style: ${style};
  font-weight: ${weight};
  font-display: block;
  src: url(data:font/woff2;base64,${data}) format("woff2");
}`;
});

const path = "ui/public/slides.css";
const css = readFileSync(path, "utf8");
const start = css.indexOf(START);
const before = (start >= 0 ? css.slice(0, start) : css).trimEnd();
writeFileSync(
  path,
  `${before}\n\n${START}\n/* Lisens: ui/public/lisenser/OFL-*.txt (SIL Open Font License 1.1) */\n${faces.join("\n")}\n${END}\n`,
);
console.log(`${path}: ${FONTS.length} fonter bygget inn`);
