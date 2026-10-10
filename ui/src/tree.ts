// Treet med mapper og filer i vaulten.
import type { Tree } from "./api.ts";
import { contextMenu } from "./dialogs.ts";

export interface TreeActions {
  open(path: string): void;
  newNote(folder: string): void;
  newFolder(parent: string): void;
  rename(path: string, newName: string): Promise<void>;
  delete(path: string, name: string): void;
  createPresentation(path: string): void;
}

export class TreeView {
  private tree?: Tree;
  private expanded = new Set<string>();
  private storageKey = "";
  private active?: string;
  /** Feltet for nytt navn. Treet tegnes på nytt når filer endres, og feltet skal overleve det. */
  private renaming?: { path: string; input: HTMLInputElement };

  constructor(
    private el: HTMLElement,
    private actions: TreeActions,
  ) {}

  /** Hvilke mapper som er åpne, huskes per vault. */
  setVault(vaultPath: string) {
    this.storageKey = `kerygma.expanded:${vaultPath}`;
    try {
      this.expanded = new Set(JSON.parse(localStorage.getItem(this.storageKey) ?? "[]"));
    } catch {
      this.expanded = new Set();
    }
  }

  render(tree: Tree) {
    this.tree = tree;
    const renaming = this.renaming;
    const hadFocus = renaming !== undefined && document.activeElement === renaming.input;
    this.el.replaceChildren(this.list(tree.children, 0));
    if (renaming) {
      const label = this.label(renaming.path);
      if (label) {
        label.replaceWith(renaming.input);
        if (hadFocus) renaming.input.focus();
      } else {
        this.renaming = undefined; // Fila finnes ikke lenger.
      }
    }
  }

  private label(path: string): HTMLElement | null {
    return this.el.querySelector<HTMLElement>(`.tree-item[data-path="${CSS.escape(path)}"] .tree-label`);
  }

  setActive(path: string | undefined) {
    this.active = path;
    if (path) for (const folder of ancestors(path)) this.expanded.add(folder);
    if (this.tree) this.render(this.tree);
    this.el.querySelector(".tree-item.active")?.scrollIntoView({ block: "nearest" });
  }

  /** Lar brukeren skrive et nytt navn rett i treet. */
  startRename(path: string) {
    const label = this.label(path);
    if (!label || this.renaming) return;
    const name = label.textContent ?? "";
    const input = document.createElement("input");
    input.className = "tree-rename";
    input.value = name;
    label.replaceWith(input);
    this.renaming = { path, input };
    input.focus();
    input.select();
    let done = false;
    const finish = async (save: boolean) => {
      if (done) return;
      done = true;
      const value = input.value.trim();
      if (save && value && value !== name) {
        // Går det ikke (navnet finnes, ugyldige tegn), vises feilen, og det gamle navnet kommer tilbake.
        await this.actions.rename(path, value).catch(() => {});
      }
      if (this.renaming?.input === input) this.renaming = undefined;
      if (this.tree) this.render(this.tree);
    };
    input.addEventListener("keydown", (event) => {
      event.stopPropagation();
      if (event.key === "Enter") void finish(true);
      if (event.key === "Escape") void finish(false);
    });
    // Blur fra omtegning (feltet flyttes i treet) skal ikke lagre.
    input.addEventListener("blur", () => {
      if (input.isConnected && document.hasFocus()) void finish(true);
    });
    input.addEventListener("click", (event) => event.stopPropagation());
  }

  private saveExpanded() {
    try {
      localStorage.setItem(this.storageKey, JSON.stringify([...this.expanded]));
    } catch {
      // Uten lagring huskes bare mappene til appen lukkes.
    }
  }

  private list(children: Tree[], depth: number): HTMLElement {
    const ul = document.createElement("ul");
    ul.className = "tree-list";
    for (const child of children) ul.append(this.item(child, depth));
    return ul;
  }

  private item(node: Tree, depth: number): HTMLElement {
    const li = document.createElement("li");
    const row = document.createElement("div");
    row.className = `tree-item ${node.kind.toLowerCase()}`;
    row.dataset.path = node.path;
    row.style.paddingLeft = `${8 + depth * 14}px`;
    row.setAttribute("role", "treeitem");
    if (node.path === this.active) row.classList.add("active");

    const icon = document.createElement("span");
    icon.className = "tree-icon";
    const label = document.createElement("span");
    label.className = "tree-label";
    label.textContent = node.name;
    row.append(icon, label);
    li.append(row);

    if (node.kind === "Folder") {
      const open = this.expanded.has(node.path);
      icon.textContent = open ? "▾" : "▸";
      row.setAttribute("aria-expanded", String(open));
      row.addEventListener("click", () => {
        if (open) this.expanded.delete(node.path);
        else this.expanded.add(node.path);
        this.saveExpanded();
        if (this.tree) this.render(this.tree);
      });
      row.addEventListener("contextmenu", (event) =>
        contextMenu(event, [
          { label: "Ny note", action: () => this.actions.newNote(node.path) },
          { label: "Ny mappe", action: () => this.actions.newFolder(node.path) },
          { label: "Gi nytt navn", action: () => this.startRename(node.path) },
          { label: "Slett", danger: true, action: () => this.actions.delete(node.path, node.name) },
        ]),
      );
      if (open) li.append(this.list(node.children, depth + 1));
    } else {
      row.addEventListener("click", () => {
        if (node.kind === "Note") this.actions.open(node.path);
      });
      const items = [
        { label: "Gi nytt navn", action: () => this.startRename(node.path) },
        { label: "Slett", danger: true, action: () => this.actions.delete(node.path, node.name) },
      ];
      if (node.kind === "Note") {
        items.push({ label: "Lag presentasjon", action: () => this.actions.createPresentation(node.path) });
      }
      row.addEventListener("contextmenu", (event) => contextMenu(event, items));
    }
    return li;
  }
}

/** Mappene en sti ligger i: "a/b/c.md" gir "a" og "a/b". */
function ancestors(path: string): string[] {
  const parts = path.split("/").slice(0, -1);
  return parts.map((_, i) => parts.slice(0, i + 1).join("/"));
}
