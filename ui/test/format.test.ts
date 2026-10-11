import { EditorSelection, EditorState } from "@codemirror/state";
import { EditorView } from "@codemirror/view";
import { describe, expect, it } from "vitest";
import { BOLD, ITALIC, UNDERLINE, exitEmptyQuote, headingLevel, setHeading, toggleMark, toggleQuote } from "../src/format.ts";

/** En editor med teksten. `|` er markøren, og `[` og `]` er markeringen. */
function view(text: string): EditorView {
  const anchor = text.includes("[") ? text.indexOf("[") : text.indexOf("|");
  const head = text.includes("]") ? text.indexOf("]") - 1 : anchor;
  const doc = text.replace(/[[\]|]/g, "");
  // Ikke i dokumentet: happy-dom sender selectionchange midt i en oppdatering når editoren får fokus.
  return new EditorView({ state: EditorState.create({ doc, selection: EditorSelection.single(anchor, head) }) });
}

/** Teksten med markøren eller markeringen satt inn igjen. */
function show(view: EditorView): string {
  const { from, to, empty } = view.state.selection.main;
  const doc = view.state.doc.toString();
  return empty ? `${doc.slice(0, from)}|${doc.slice(from)}` : `${doc.slice(0, from)}[${doc.slice(from, to)}]${doc.slice(to)}`;
}

describe("toggleMark", () => {
  it("setter merket rundt markeringen og fjerner det igjen", () => {
    const v = view("Herren er [god] mot alle");
    toggleMark(v, BOLD);
    expect(show(v)).toBe("Herren er **[god]** mot alle");
    toggleMark(v, BOLD);
    expect(show(v)).toBe("Herren er [god] mot alle");
  });

  it("bruker ordet markøren står i", () => {
    const v = view("Herren er g|od");
    toggleMark(v, ITALIC);
    expect(v.state.doc.toString()).toBe("Herren er *god*");
  });

  it("lar mellomrom i kantene av markeringen stå utenfor", () => {
    const v = view("Herren er[ god ]mot");
    toggleMark(v, BOLD);
    expect(v.state.doc.toString()).toBe("Herren er **god** mot");
  });

  it("skiller fet og kursiv i ***tekst***", () => {
    const v = view("***[god]***");
    toggleMark(v, ITALIC);
    expect(v.state.doc.toString()).toBe("**god**");
  });

  it("understreking bruker HTML-taggen, som Obsidian", () => {
    const v = view("[god]");
    toggleMark(v, UNDERLINE);
    expect(v.state.doc.toString()).toBe("<u>god</u>");
  });
});

describe("overskrifter", () => {
  it("gir nivået på linja og endrer det", () => {
    const v = view("## Tit|tel");
    expect(headingLevel(v.state)).toBe(2);
    setHeading(v, 1);
    expect(v.state.doc.toString()).toBe("# Tittel");
    setHeading(v, 0);
    expect(v.state.doc.toString()).toBe("Tittel");
  });

  it("tomme linjer i markeringen får ikke overskrift", () => {
    const v = view("[En\n\nTo]");
    setHeading(v, 2);
    expect(v.state.doc.toString()).toBe("## En\n\n## To");
  });
});

describe("sitat", () => {
  it("gjør linjene til ett sitat, også tomme linjer imellom, og tilbake", () => {
    const v = view("[En\n\nTo]");
    toggleQuote(v);
    expect(v.state.doc.toString()).toBe("> En\n>\n> To");
    v.dispatch({ selection: EditorSelection.range(0, v.state.doc.length) });
    toggleQuote(v);
    expect(v.state.doc.toString()).toBe("En\n\nTo");
  });

  it("Enter på en tom sitatlinje avslutter sitatet", () => {
    const v = view("> Et sitat\n> |");
    expect(exitEmptyQuote(v)).toBe(true);
    expect(show(v)).toBe("> Et sitat\n\n|");
  });

  it("Enter midt i sitatet lar det fortsette", () => {
    const v = view("> En\n> |\n> To");
    expect(exitEmptyQuote(v)).toBe(false);
    expect(exitEmptyQuote(view("Vanlig tekst|"))).toBe(false);
  });
});
