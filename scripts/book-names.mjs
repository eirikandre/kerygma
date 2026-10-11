// Boknavnene `reference.rs` kjenner igjen, og fasiten med referanser testene bruker.
//
//   node scripts/book-names.mjs
//       lager crates/kerygma-core/src/book_names.rs fra scripts/book-names.json
//
//   node scripts/book-names.mjs --fra-bible [../bible]
//       henter først boknavnene og referansene på nytt fra bible-repoet (data/*/index.json
//       og `target/release/bibel`, bygget med `cargo build --release` der), og skriver
//       scripts/book-names.json og crates/kerygma-core/tests/fixtures/bibel-referanser.txt.
//       Kjør det når bible får nye oversettelser eller boknavn.
import { execFileSync } from "node:child_process";
import { existsSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";

const JSON_PATH = "scripts/book-names.json";
const RUST_PATH = "crates/kerygma-core/src/book_names.rs";
const FIXTURE_PATH = "crates/kerygma-core/tests/fixtures/bibel-referanser.txt";

if (process.argv[2] === "--fra-bible") fromBible(resolve(process.argv[3] ?? "../bible"));

const entries = JSON.parse(readFileSync(JSON_PATH, "utf8"));
if (!Array.isArray(entries) || entries.length < 100) throw new Error(`${JSON_PATH} har for få boknavn`);

const lines = entries.map(([name, id]) => `    (${JSON.stringify(name)}, ${JSON.stringify(id)}),`);
writeFileSync(
  RUST_PATH,
  `// Generert av scripts/book-names.mjs fra scripts/book-names.json. Ikke rediger for hånd.
// Boknavn og forkortelser fra alle oversettelsene, med bok-id-en \`bibel\` slår opp på.
// Sortert etter navn (norsk rekkefølge); senere navn med samme nøkkel vinner.
pub const BOOK_IDS: &[(&str, &str)] = &[
${lines.join("\n")}
];
`,
);
console.log(`${RUST_PATH}: ${entries.length} boknavn`);

/** Henter boknavnene og referansene fra bible-repoet. */
function fromBible(bibleDir) {
  const binary = join(bibleDir, "target/release/bibel");
  if (!existsSync(binary)) throw new Error(`fant ikke ${binary}; kjør cargo build --release i ${bibleDir}`);

  // Navn og forkortelser -> bok-id (OSIS i NB88/KJV, USFM i Bibel 2024).
  // `bibel` godtar begge kodeverkene i alle oversettelsene. Første navn vinner.
  const books = new Map();
  for (const dir of readdirSync(join(bibleDir, "data")).sort()) {
    const indexPath = join(bibleDir, "data", dir, "index.json");
    if (!existsSync(indexPath)) continue;
    for (const book of JSON.parse(readFileSync(indexPath, "utf8")).books) {
      const id = book.osisId ?? book.bookId;
      for (const key of ["name", "nameLong", "abbrev"]) {
        if (book[key] && !books.has(book[key])) books.set(book[key], id);
      }
    }
  }
  const sorted = [...books].sort(([a], [b]) => a.localeCompare(b, "nb"));
  writeFileSync(JSON_PATH, `[\n${sorted.map((entry) => `  ${JSON.stringify(entry)}`).join(",\n")}\n]\n`);
  console.log(`${JSON_PATH}: ${sorted.length} boknavn`);

  // "({ref})" fra `bibel` for hver bok og form i alle oversettelsene som er bygget inn.
  const translations = execFileSync(binary, ["--oversettelser"], { encoding: "utf8" })
    .split("\n")
    .map((line) => /^[ *] +(\w+) +.*\d+ bøker/.exec(line)?.[1])
    .filter(Boolean);
  const loci = ["1:1", "1:1-2", "1", "1:2-2:1"];
  const references = [];
  for (const translation of translations) {
    const bookIds = execFileSync(binary, ["-t", translation, "--bøker"], { encoding: "utf8" })
      .split("\n")
      .map((line) => /(\S+)\s+\d+ kap\./.exec(line)?.[1])
      .filter(Boolean);
    for (const book of bookIds) {
      for (const locus of loci) {
        try {
          const out = execFileSync(binary, ["--format", "({ref})", `${translation}:${book}${locus}`], {
            encoding: "utf8",
            stdio: ["ignore", "pipe", "ignore"],
          });
          references.push(out.trim());
        } catch {
          // Boka har bare ett kapittel, eller formen finnes ikke: hopp over.
        }
      }
    }
  }
  const unique = [...new Set(references)];
  writeFileSync(FIXTURE_PATH, `${unique.join("\n")}\n`);
  console.log(`${FIXTURE_PATH}: ${unique.length} referanser fra ${translations.join(", ")}`);
}
