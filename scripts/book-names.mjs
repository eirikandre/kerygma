// Lager crates/kerygma-core/src/book_names.rs fra BOOK_IDS i pluginen.
// Pluginen leses bare, den endres ikke:
//   node scripts/book-names.mjs [../obsidian-slide-creator]
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const plugin = process.argv[2] ?? "../obsidian-slide-creator";
const source = readFileSync(join(plugin, "src/bookNames.ts"), "utf8");
const entries = [...source.matchAll(/^\s*"([^"]*)": "([^"]*)",?$/gm)].map((m) => [m[1], m[2]]);
if (entries.length < 100) throw new Error(`fant bare ${entries.length} boknavn`);

const lines = entries.map(([name, id]) => `    (${JSON.stringify(name)}, ${JSON.stringify(id)}),`);
writeFileSync(
  "crates/kerygma-core/src/book_names.rs",
  `// Generert av scripts/book-names.mjs fra BOOK_IDS i obsidian-slide-creator/src/bookNames.ts.
// Boknavn og forkortelser fra alle oversettelsene, med bok-id-en \`bibel\` slår opp på.
// Rekkefølgen er den samme som i pluginen: senere navn med samme nøkkel vinner.
pub const BOOK_IDS: &[(&str, &str)] = &[
${lines.join("\n")}
];
`,
);
console.log(`skrev ${entries.length} boknavn`);
