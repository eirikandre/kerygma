// «Sett inn bibelvers»: "1mos3:1" viser verset, og Enter setter det inn ved
// markøren som sitatet "> tekst (ref)". "s=tro håp" søker. Etter verseModal.ts i pluginen.
import { api, errorMessage, type Suggestion } from "./api.ts";
import { openModal } from "./dialogs.ts";
import type { Editor } from "./editor.ts";

const PREVIEW_CHARS = 300;

export function openVerseModal(editor: Editor) {
  const modal = openModal("verse-modal", () => editor.focus());
  modal.el.innerHTML = `
    <input class="verse-input" type="text" spellcheck="false" autocomplete="off"
      placeholder="Bibelreferanse, som joh3:16, ne:sal23 eller s=tro håp" />
    <div class="verse-suggestions"></div>
    <div class="verse-instructions"><span><kbd>↑↓</kbd> velg</span><span><kbd>↵</kbd> sett inn</span><span><kbd>esc</kbd> avbryt</span></div>`;
  const input = modal.el.querySelector<HTMLInputElement>(".verse-input")!;
  const list = modal.el.querySelector<HTMLElement>(".verse-suggestions")!;

  let suggestions: Suggestion[] = [];
  let selected = -1;
  let request = 0;

  const selectable = () => suggestions.flatMap((s, i) => (s.text !== undefined ? [i] : []));

  const render = () => {
    list.replaceChildren();
    if (suggestions.length === 0) {
      const empty = document.createElement("div");
      empty.className = "verse-empty";
      empty.textContent = "Skriv en referanse, som 1mos3:1 eller ne:joh3:16. s=tro håp søker.";
      list.append(empty);
      return;
    }
    suggestions.forEach((suggestion, i) => {
      const el = document.createElement("div");
      if (suggestion.message !== undefined) {
        el.className = "verse-message";
        el.textContent = suggestion.message;
      } else {
        const text = suggestion.text ?? "";
        el.className = i === selected ? "verse-suggestion selected" : "verse-suggestion";
        el.textContent = text.length > PREVIEW_CHARS ? `${text.slice(0, PREVIEW_CHARS)}…` : text;
        el.addEventListener("mousemove", () => {
          if (selected !== i) {
            selected = i;
            render();
          }
        });
        el.addEventListener("click", () => void choose(i));
      }
      list.append(el);
    });
    list.querySelector(".selected")?.scrollIntoView({ block: "nearest" });
  };

  const update = async () => {
    const id = ++request;
    let result: Suggestion[];
    try {
      result = await api.lookupVerse(input.value);
    } catch (error) {
      result = [{ message: errorMessage(error) }];
    }
    if (id !== request) return; // Et nyere tastetrykk har allerede svart.
    suggestions = result;
    selected = selectable()[0] ?? -1;
    render();
  };

  // En melding kan ikke settes inn, så vinduet blir stående for å rette referansen.
  const choose = async (index: number) => {
    const text = suggestions[index]?.text;
    if (text === undefined) return;
    const { before, after } = editor.around();
    const quote = await api.quoteVerse(text, before, after);
    modal.close();
    editor.replaceSelection(quote);
  };

  input.addEventListener("input", () => void update());
  input.addEventListener("keydown", (event) => {
    const indices = selectable();
    const position = indices.indexOf(selected);
    if (event.key === "ArrowDown" && indices.length > 0) {
      event.preventDefault();
      selected = indices[(position + 1) % indices.length];
      render();
    } else if (event.key === "ArrowUp" && indices.length > 0) {
      event.preventDefault();
      selected = indices[(position - 1 + indices.length) % indices.length];
      render();
    } else if (event.key === "Enter") {
      event.preventDefault();
      if (selected >= 0) void choose(selected);
    }
  });

  render();
  input.focus();
}
