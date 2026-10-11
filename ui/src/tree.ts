// Treet med mapper og filer i vaulten.
import type { Tree } from "./api.ts";
import { contextMenu } from "./dialogs.ts";
import type { ExportKind } from "./export.ts";

export interface TreeActions {
  open(path: string): void;
  newNote(folder: string): void;
  newFolder(parent: string): void;
  rename(path: string, newName: string): Promise<void>;
  /** Flytter fila eller mappa til `folder` (tom for roten). */
  move(path: string, folder: string): void;
  delete(path: string, name: string): void;
  createPresentation(path: string): void;
  exportPresentation(path: string, kind: ExportKind): void;
  importWord(folder: string): void;
}

export class TreeView {
  private tree?: Tree;
  private expanded = new Set<string>();
  private storageKey = "";
  private active?: string;
  /** Mappa nye manus og mapper lages i: den som sist ble klikket, eller mappa til fila som sist ble klikket eller åpnet. */
  private location = "";
  /** Feltet for nytt navn. Treet tegnes på nytt når filer endres, og feltet skal overleve det. */
  private renaming?: { path: string; input: HTMLInputElement };
  /** Fila eller mappa som dras, og mappa den slippes i ("" er roten). */
  private dragging?: string;
  private dropFolder?: string;
  private expandTimer?: number;
  private expandTimerFolder?: string;

  constructor(
    private el: HTMLElement,
    private actions: TreeActions,
  ) {
    el.addEventListener("dragstart", (event) => this.dragStart(event));
    el.addEventListener("dragover", (event) => this.dragOver(event));
    el.addEventListener("dragleave", (event) => {
      if (!(event.relatedTarget instanceof Node && el.contains(event.relatedTarget))) this.setDropFolder(undefined);
    });
    el.addEventListener("drop", (event) => this.drop(event));
    el.addEventListener("dragend", () => this.endDrag());
  }

  /** Hvilke mapper som er åpne, huskes per vault. */
  setVault(vaultPath: string) {
    this.storageKey = `kerygma.expanded:${vaultPath}`;
    this.location = "";
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
    this.showDropFolder();
    if (renaming) {
      const label = this.label(renaming.path);
      if (label) {
        label.replaceWith(renaming.input);
        notDraggable(renaming.input);
        if (hadFocus) renaming.input.focus();
      } else {
        this.renaming = undefined; // Fila finnes ikke lenger.
      }
    }
  }

  private label(path: string): HTMLElement | null {
    return this.el.querySelector<HTMLElement>(`.tree-item[data-path="${CSS.escape(path)}"] .tree-label`);
  }

  /** Mappa nye manus og mapper skal lages i. Roten hvis mappa ikke finnes lenger. */
  get folder(): string {
    return this.tree && findFolder(this.tree, this.location) ? this.location : "";
  }

  setLocation(folder: string) {
    this.location = folder;
  }

  /** En fil eller mappe har fått ny sti; ligger stedet i den, følger det etter. */
  moved(path: string, newPath: string) {
    if (this.location === path) this.location = newPath;
    else if (this.location.startsWith(`${path}/`)) this.location = newPath + this.location.slice(path.length);
  }

  setActive(path: string | undefined) {
    this.active = path;
    if (path) for (const folder of ancestors(path)) this.expanded.add(folder);
    if (this.tree) this.render(this.tree);
    this.el.querySelector(".tree-item.active")?.scrollIntoView({ block: "nearest" });
  }

  /** Lar brukeren skrive et nytt navn rett i treet. */
  startRename(path: string) {
    const hidden = ancestors(path).filter((folder) => !this.expanded.has(folder));
    if (hidden.length && this.tree) {
      for (const folder of hidden) this.expanded.add(folder);
      this.saveExpanded();
      this.render(this.tree);
    }
    const label = this.label(path);
    if (!label || this.renaming) return;
    const name = label.textContent ?? "";
    const input = document.createElement("input");
    input.className = "tree-rename";
    input.value = name;
    label.replaceWith(input);
    notDraggable(input);
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

  private dragStart(event: DragEvent) {
    const row = (event.target as Element).closest<HTMLElement>(".tree-item");
    const path = row?.dataset.path;
    if (!path || !event.dataTransfer || this.renaming) {
      event.preventDefault();
      return;
    }
    this.dragging = path;
    event.dataTransfer.effectAllowed = "move";
    event.dataTransfer.setData(DRAG_TYPE, path);
  }

  private dragOver(event: DragEvent) {
    // Filer fra Finder/Utforsker og tekst fra editoren slippes ikke i treet.
    if (!this.dragging || !event.dataTransfer?.types.includes(DRAG_TYPE)) return;
    const row = (event.target as Element).closest<HTMLElement>(".tree-item");
    const folder = dropFolderFor(row);
    if (!canMove(this.dragging, folder)) {
      event.dataTransfer.dropEffect = "none";
      this.setDropFolder(undefined);
      return;
    }
    event.preventDefault();
    event.dataTransfer.dropEffect = "move";
    this.setDropFolder(folder);
    // En lukket mappe åpnes når man holder over den en stund.
    const closed = row?.classList.contains("folder") && !this.expanded.has(folder) ? folder : undefined;
    if (closed !== this.expandTimerFolder) {
      window.clearTimeout(this.expandTimer);
      this.expandTimerFolder = closed;
      if (closed !== undefined) {
        this.expandTimer = window.setTimeout(() => {
          this.expanded.add(closed);
          this.saveExpanded();
          if (this.tree) this.render(this.tree);
        }, EXPAND_DELAY_MS);
      }
    }
  }

  private drop(event: DragEvent) {
    const path = this.dragging;
    const folder = this.dropFolder;
    this.endDrag();
    if (!path || folder === undefined || !event.dataTransfer?.types.includes(DRAG_TYPE)) return;
    event.preventDefault();
    if (canMove(path, folder)) {
      if (folder) this.expanded.add(folder);
      this.saveExpanded();
      this.actions.move(path, folder);
    }
  }

  private endDrag() {
    this.dragging = undefined;
    window.clearTimeout(this.expandTimer);
    this.expandTimerFolder = undefined;
    this.setDropFolder(undefined);
  }

  private setDropFolder(folder: string | undefined) {
    if (folder === this.dropFolder) return;
    this.dropFolder = folder;
    this.showDropFolder();
  }

  /** Markerer mappa det slippes i. Roten markeres som hele treet. */
  private showDropFolder() {
    for (const row of this.el.querySelectorAll(".tree-item.drop-target")) row.classList.remove("drop-target");
    this.el.classList.toggle("drop-root", this.dropFolder === "");
    if (this.dropFolder) {
      this.el.querySelector(`.tree-item[data-path="${CSS.escape(this.dropFolder)}"]`)?.classList.add("drop-target");
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
    row.draggable = true;
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
        this.location = node.path;
        if (open) this.expanded.delete(node.path);
        else this.expanded.add(node.path);
        this.saveExpanded();
        if (this.tree) this.render(this.tree);
      });
      row.addEventListener("contextmenu", (event) =>
        contextMenu(event, [
          { label: "Nytt manus", action: () => this.actions.newNote(node.path) },
          { label: "Ny mappe", action: () => this.actions.newFolder(node.path) },
          { label: "Importer Word-manus hit …", action: () => this.actions.importWord(node.path) },
          { label: "Gi nytt navn", action: () => this.startRename(node.path) },
          { label: "Slett", danger: true, action: () => this.actions.delete(node.path, node.name) },
        ]),
      );
      if (open) li.append(this.list(node.children, depth + 1));
    } else {
      row.addEventListener("click", () => {
        this.location = ancestors(node.path).at(-1) ?? "";
        if (node.kind === "Note") this.actions.open(node.path);
      });
      const items = [
        { label: "Gi nytt navn", action: () => this.startRename(node.path) },
        { label: "Slett", danger: true, action: () => this.actions.delete(node.path, node.name) },
      ];
      if (node.kind === "Note") {
        items.push(
          { label: "Lag presentasjon", action: () => this.actions.createPresentation(node.path) },
          { label: "Eksporter presentasjonen som PDF …", action: () => this.actions.exportPresentation(node.path, "pdf") },
          { label: "Eksporter presentasjonen som PowerPoint …", action: () => this.actions.exportPresentation(node.path, "pptx") },
          { label: "Eksporter presentasjonen som HTML …", action: () => this.actions.exportPresentation(node.path, "html") },
        );
      }
      row.addEventListener("contextmenu", (event) => contextMenu(event, items));
    }
    return li;
  }
}

const DRAG_TYPE = "application/x-kerygma-path";
const EXPAND_DELAY_MS = 700;

/** Mappa det slippes i: mappa man holder over, mappa fila ligger i, eller roten. */
function dropFolderFor(row: HTMLElement | null): string {
  const path = row?.dataset.path;
  if (!path) return "";
  return row.classList.contains("folder") ? path : (ancestors(path).at(-1) ?? "");
}

/** Kan `path` flyttes til `folder`? Ikke inn i seg selv, og ikke dit den allerede ligger. */
function canMove(path: string, folder: string): boolean {
  return folder !== path && !folder.startsWith(`${path}/`) && (ancestors(path).at(-1) ?? "") !== folder;
}

/** Feltet for nytt navn ligger i en rad som ellers kan dras; da kan ikke teksten merkes med musa. */
function notDraggable(input: HTMLInputElement) {
  const row = input.closest<HTMLElement>(".tree-item");
  if (row) row.draggable = false;
}

/** Finnes mappa i treet? */
function findFolder(node: Tree, path: string): boolean {
  if (node.kind !== "Folder") return false;
  return node.path === path || (path.startsWith(node.path) && node.children.some((child) => findFolder(child, path)));
}

/** Mappene en sti ligger i: "a/b/c.md" gir "a" og "a/b". */
function ancestors(path: string): string[] {
  const parts = path.split("/").slice(0, -1);
  return parts.map((_, i) => parts.slice(0, i + 1).join("/"));
}
