// «Importer Word-manus»: velg .docx-filer, spør før overskriving, og vis en rapport per fil.
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { api, basename, type ImportedFile } from "./api.ts";
import { confirmOverwrite, notice, openModal } from "./dialogs.ts";

/** Første linje i boksen rundt et sitat som bør sjekkes (`> [!warning|sjekk] …`), som `word::convert`. */
const WARNING_MARKER = "[!warning|sjekk]";

/**
 * Gjør valgte .docx-filer om til markdown med bibelsitatene som "> tekst (ref)".
 * Er det bare én fil, åpnes den. `search` søker i alle manusene, for å finne advarslene.
 */
export async function importWord(
  currentFolder: string,
  openNote: (path: string) => Promise<void>,
  search: (query: string) => void,
  folderChosen = false,
) {
  const chosen = await openDialog({
    multiple: true,
    title: "Importer Word-manus",
    filters: [{ name: "Word-manus", extensions: ["docx"] }],
  });
  const files = Array.isArray(chosen) ? chosen : typeof chosen === "string" ? [chosen] : [];
  if (files.length === 0) return;

  const targets = await api.importTargets(files, currentFolder, folderChosen);
  const skipped: ImportedFile[] = [];
  const requests: Array<{ file: string; path: string }> = [];
  for (const target of targets) {
    if (target.isFolder) {
      skipped.push({ name: target.name, path: null, quotes: 0, uncertain: 0, unmatched: [], error: `"${target.path}" er en mappe` });
    } else if (target.exists && !(await confirmOverwrite(target.path))) {
      skipped.push({ name: target.name, path: null, quotes: 0, uncertain: 0, unmatched: [], error: "finnes fra før, ikke overskrevet" });
    } else {
      requests.push({ file: target.file, path: target.path });
    }
  }

  let imported: ImportedFile[] = [];
  if (requests.length > 0) {
    notice(`Importerer ${requests.length === 1 ? basename(requests[0].path) : `${requests.length} filer`} …`);
    imported = await api.importWord(requests);
  }
  const all = [...imported, ...skipped];
  const done = all.filter((file) => file.path);
  if (done.length === 1 && all.length === 1) await openNote(done[0].path!);
  showReport(all, openNote, search);
}

/** Viser hvor mange sitater som ble funnet i hver fil, og hva som bør sjekkes. */
function showReport(imported: ImportedFile[], openNote: (path: string) => Promise<void>, search: (query: string) => void) {
  const modal = openModal("import-modal");
  const heading = document.createElement("h2");
  heading.textContent = "Importerte Word-manus";
  modal.el.append(heading);

  let warnings = 0;
  const list = document.createElement("ul");
  for (const file of imported) {
    const item = document.createElement("li");
    if (file.path) {
      const link = document.createElement("a");
      link.href = "#";
      link.textContent = file.name;
      link.addEventListener("click", (event) => {
        event.preventDefault();
        modal.close();
        void openNote(file.path!);
      });
      item.append(link);
    } else {
      const name = document.createElement("strong");
      name.textContent = file.name;
      item.append(name);
    }
    if (file.error || !file.path) {
      item.append(`: ikke importert, ${file.error}`);
      list.append(item);
      continue;
    }
    warnings += file.uncertain;
    item.append(`: ${file.quotes} ${file.quotes === 1 ? "bibelsitat" : "bibelsitater"}` + (file.uncertain > 0 ? `, ${file.uncertain} med advarsel` : ""));
    if (file.unmatched.length > 0) {
      // Disse er ikke merket i manuset, siden de står midt i teksten.
      const small = document.createElement("small");
      small.textContent = `Referanser uten sitat: ${file.unmatched.join(", ")}`;
      item.append(document.createElement("br"), small);
    }
    list.append(item);
  }
  modal.el.append(list);

  if (warnings > 0) {
    const help = document.createElement("div");
    help.className = "import-help";
    help.innerHTML = `
      <h3>Sjekke sitatene med advarsel</h3>
      <p>Sitatene verktøyet er usikre på, står i en gul boks i manuset. Øverst i boksen står det hvorfor,
        for eksempel at verset ble funnet med søk fordi manuset ikke hadde referanse.</p>
      <ol>
        <li>Trykk «Søk etter advarsler», så vises manusene med advarsler i sidefeltet. Trykk på en linje for å gå dit.
          Du kan også søke etter <code>[!warning|sjekk]</code> i manuset med Cmd/Ctrl + F.</li>
        <li>Les sitatet og referansen i parentes. Rett det som er feil. Er det ikke et bibelsitat,
          fjern &gt; foran linjene og referansen bak.</li>
        <li>Slett den første linja: <code>&gt; [!warning|sjekk] …</code>. Da blir resten et vanlig sitat, og boksen forsvinner.</li>
      </ol>
      <p class="setting-desc">Advarslene blir ikke med i presentasjonen, så du kan lage den før alt er sjekket.</p>`;
    modal.el.append(help);
  }

  const buttons = document.createElement("div");
  buttons.className = "import-buttons";
  if (warnings > 0) {
    const find = document.createElement("button");
    find.type = "button";
    find.textContent = "Søk etter advarsler";
    find.addEventListener("click", () => {
      modal.close();
      search(WARNING_MARKER);
    });
    buttons.append(find);
  }
  const close = document.createElement("button");
  close.type = "button";
  close.textContent = "Lukk";
  close.className = "primary";
  close.addEventListener("click", () => modal.close());
  buttons.append(close);
  modal.el.append(buttons);
}
