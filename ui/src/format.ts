// Enkel tekstformatering i editoren: overskrift på linja og fet, kursiv,
// understreket og overstreket rundt markeringen. Alt slår av og på.
import { EditorSelection, type EditorState } from "@codemirror/state";
import type { EditorView } from "@codemirror/view";

export interface Mark {
  open: string;
  close: string;
}

export const BOLD: Mark = { open: "**", close: "**" };
export const ITALIC: Mark = { open: "*", close: "*" };
export const STRIKETHROUGH: Mark = { open: "~~", close: "~~" };
/** Markdown har ikke understreking; Obsidian bruker HTML-taggen. */
export const UNDERLINE: Mark = { open: "<u>", close: "</u>" };

/** Antall like tegn på starten (eller slutten) av teksten. */
function run(text: string, char: string, atEnd: boolean): number {
  let n = 0;
  while (n < text.length && text[atEnd ? text.length - 1 - n : n] === char) n++;
  return n;
}

/**
 * Om merket står rundt teksten, gitt antall `*` på hver side. `*` og `**` deler
 * tegn: "***tekst***" er både fet og kursiv, "**tekst**" bare fet.
 */
function hasStars(mark: Mark, left: number, right: number): boolean {
  const n = Math.min(left, right, 3);
  return mark.open.length === 2 ? n >= 2 : n === 1 || n === 3;
}

/** Om `text` selv starter og slutter med merket, som når "**fet**" er markert. */
function wrappedInside(mark: Mark, text: string): boolean {
  if (text.length < mark.open.length + mark.close.length) return false;
  if (mark.open[0] === "*") return hasStars(mark, run(text, "*", false), run(text, "*", true));
  return text.startsWith(mark.open) && text.endsWith(mark.close);
}

/** Om merket står rett før og etter, som når bare "fet" i "**fet**" er markert. */
function wrappedOutside(mark: Mark, before: string, after: string): boolean {
  if (mark.open[0] === "*") return hasStars(mark, run(before, "*", true), run(after, "*", false));
  return before.endsWith(mark.open) && after.startsWith(mark.close);
}

/**
 * Setter merket rundt markeringen, eller fjerner det hvis det er der. Uten
 * markering brukes ordet markøren står i; står den ikke i et ord, settes
 * merket inn med markøren imellom.
 */
export function toggleMark(view: EditorView, mark: Mark) {
  const { state } = view;
  const open = mark.open.length;
  const close = mark.close.length;
  view.dispatch(
    state.changeByRange((range) => {
      let { from, to } = range;
      if (range.empty) {
        const word = state.wordAt(from);
        if (word) ({ from, to } = word);
      } else {
        // "** fet**" er ikke fet i markdown, så mellomrom i kantene blir utenfor.
        const text = state.sliceDoc(from, to);
        from += text.length - text.trimStart().length;
        to -= text.length - text.trimEnd().length;
        if (from > to) to = from;
      }
      const text = state.sliceDoc(from, to);
      const before = state.sliceDoc(state.doc.lineAt(from).from, from);
      const after = state.sliceDoc(to, state.doc.lineAt(to).to);

      if (from < to && wrappedInside(mark, text)) {
        return {
          changes: [
            { from, to: from + open },
            { from: to - close, to },
          ],
          range: EditorSelection.range(from, to - open - close),
        };
      }
      if (wrappedOutside(mark, before, after)) {
        return {
          changes: [
            { from: from - open, to: from },
            { from: to, to: to + close },
          ],
          range: range.empty ? EditorSelection.cursor(range.head - open) : EditorSelection.range(from - open, to - open),
        };
      }
      return {
        changes: [
          { from, insert: mark.open },
          { from: to, insert: mark.close },
        ],
        range: range.empty ? EditorSelection.cursor(range.head + open) : EditorSelection.range(from + open, to + open),
      };
    }),
  );
  view.focus();
}

const HEADING = /^(#{1,6})[ \t]+/;

/** Overskriftsnivået på linja markøren står på (0: vanlig tekst). */
export function headingLevel(state: EditorState): number {
  const line = state.doc.lineAt(state.selection.main.head);
  return HEADING.exec(line.text)?.[1].length ?? 0;
}

/** Gjør linjene i markeringen til overskrifter på nivået, eller til vanlig tekst (0). */
export function setHeading(view: EditorView, level: number) {
  const { state } = view;
  const prefix = level > 0 ? `${"#".repeat(level)} ` : "";
  const changes: Array<{ from: number; to: number; insert: string }> = [];
  const seen = new Set<number>();
  for (const range of state.selection.ranges) {
    for (let pos = range.from; pos <= range.to; ) {
      const line = state.doc.lineAt(pos);
      if (!seen.has(line.number)) {
        seen.add(line.number);
        const existing = HEADING.exec(line.text)?.[0] ?? "";
        // Tomme linjer får ikke overskrift, med mindre det er den eneste linja.
        if (line.text.trim() !== "" || range.from === range.to) {
          changes.push({ from: line.from, to: line.from + existing.length, insert: prefix });
        }
      }
      pos = line.to + 1;
    }
  }
  view.dispatch({ changes });
  view.focus();
}
