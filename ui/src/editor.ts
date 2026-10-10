// Editoren: CodeMirror 6 med markdown, linjebryting og ingen linjenumre.
import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
import { markdown, markdownLanguage } from "@codemirror/lang-markdown";
import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { languages } from "@codemirror/language-data";
import { highlightSelectionMatches, search, searchKeymap } from "@codemirror/search";
import { EditorState, Transaction, type Extension } from "@codemirror/state";
import { EditorView, drawSelection, keymap, placeholder } from "@codemirror/view";
import { tags } from "@lezer/highlight";
import { BOLD, ITALIC, STRIKETHROUGH, UNDERLINE, toggleMark } from "./format.ts";

/** Markdown med fargene fra styles.css, så det virker i både lyst og mørkt tema. */
const highlightStyle = HighlightStyle.define([
  { tag: tags.heading, fontWeight: "bold" },
  { tag: tags.emphasis, fontStyle: "italic" },
  { tag: tags.strong, fontWeight: "bold" },
  { tag: tags.strikethrough, textDecoration: "line-through" },
  { tag: [tags.link, tags.url], color: "var(--accent)" },
  { tag: tags.quote, color: "var(--text-muted)" },
  { tag: tags.monospace, fontFamily: "var(--font-mono)", fontSize: "0.9em" },
  { tag: [tags.processingInstruction, tags.meta, tags.contentSeparator], color: "var(--text-faint)" },
]);

/** Markeringen tegnes av CodeMirror (drawSelection), med en farge som bare passer lyst tema. */
const selectionTheme = EditorView.theme({
  "&.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection":
    { backgroundColor: "var(--selection)" },
});

export class Editor {
  readonly view: EditorView;

  /** `onChange` når teksten endres av brukeren, `onCursor` når markøren eller markeringen flyttes. */
  constructor(parent: HTMLElement, onChange: () => void, onCursor: () => void = () => {}) {
    this.onChange = onChange;
    this.onCursor = onCursor;
    this.view = new EditorView({ parent, state: this.newState("") });
  }

  private onChange: () => void;
  private onCursor: () => void;

  private newState(doc: string): EditorState {
    const onChange = this.onChange;
    const onCursor = this.onCursor;
    const extensions: Extension[] = [
      history(),
      drawSelection(),
      search({ top: true }),
      highlightSelectionMatches(),
      markdown({ base: markdownLanguage, codeLanguages: languages }),
      syntaxHighlighting(highlightStyle),
      selectionTheme,
      EditorView.lineWrapping,
      placeholder("Skriv her …"),
      keymap.of([
        { key: "Mod-b", run: (view) => (toggleMark(view, BOLD), true) },
        { key: "Mod-i", run: (view) => (toggleMark(view, ITALIC), true) },
        { key: "Mod-u", run: (view) => (toggleMark(view, UNDERLINE), true) },
        { key: "Mod-Shift-x", run: (view) => (toggleMark(view, STRIKETHROUGH), true) },
        ...defaultKeymap,
        ...historyKeymap,
        ...searchKeymap,
        indentWithTab,
      ]),
      EditorView.updateListener.of((update) => {
        // Endringer fra setValue er merket, så de ikke regnes som ulagrede.
        const external = update.transactions.some((tr) => tr.annotation(Transaction.userEvent) === "kerygma.load");
        if (update.docChanged && !external) onChange();
        if (update.docChanged || update.selectionSet) onCursor();
      }),
    ];
    return EditorState.create({ doc, extensions });
  }

  getValue(): string {
    return this.view.state.doc.toString();
  }

  /** Ny fil: nytt dokument uten angrehistorikk fra forrige fil. */
  load(content: string) {
    this.view.setState(this.newState(content));
    this.onCursor();
  }

  /** Samme fil, endret utenfra: bytter teksten, men beholder markøren der det går. */
  replace(content: string) {
    const head = Math.min(this.view.state.selection.main.head, content.length);
    this.view.dispatch({
      changes: { from: 0, to: this.view.state.doc.length, insert: content },
      selection: { anchor: head },
      annotations: Transaction.userEvent.of("kerygma.load"),
    });
  }

  focus() {
    this.view.focus();
  }

  /** Teksten før og etter markeringen på linjene den starter og slutter på. */
  around(): { before: string; after: string } {
    const { state } = this.view;
    const { from, to } = state.selection.main;
    const fromLine = state.doc.lineAt(from);
    const toLine = state.doc.lineAt(to);
    return { before: fromLine.text.slice(0, from - fromLine.from), after: toLine.text.slice(to - toLine.from) };
  }

  replaceSelection(text: string) {
    this.view.dispatch(this.view.state.replaceSelection(text));
    this.view.focus();
  }
}
