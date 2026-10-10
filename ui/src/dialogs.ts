// Meldinger (som Obsidians Notice), spørsmål og et enkelt modalvindu.
import { ask } from "@tauri-apps/plugin-dialog";

/** En liten boks nederst til høyre. Forsvinner etter 4 sekunder; feil blir stående til de klikkes bort. */
export function notice(message: string, options: { error?: boolean } = {}) {
  const container = document.getElementById("notices")!;
  const el = document.createElement("div");
  el.className = options.error ? "notice error" : "notice";
  el.textContent = message;
  el.addEventListener("click", () => el.remove());
  container.append(el);
  if (!options.error) setTimeout(() => el.remove(), 4000);
}

/** «"<sti>" finnes allerede. Vil du overskrive den?» med Avbryt og Overskriv. */
export function confirmOverwrite(path: string): Promise<boolean> {
  return ask(`"${path}" finnes allerede. Vil du overskrive den?`, {
    title: "Kerygma",
    kind: "warning",
    okLabel: "Overskriv",
    cancelLabel: "Avbryt",
  });
}

export function confirmDelete(name: string): Promise<boolean> {
  return ask(`Vil du flytte "${name}" til papirkurven?`, {
    title: "Kerygma",
    kind: "warning",
    okLabel: "Slett",
    cancelLabel: "Avbryt",
  });
}

export interface Modal {
  el: HTMLElement;
  close(): void;
}

/** Et modalvindu midt på skjermen. Esc og klikk utenfor lukker det. */
export function openModal(className: string, onClose?: () => void): Modal {
  const backdrop = document.createElement("div");
  backdrop.className = "modal-backdrop";
  const el = document.createElement("div");
  el.className = `modal ${className}`;
  backdrop.append(el);
  document.body.append(backdrop);

  const previousFocus = document.activeElement as HTMLElement | null;
  let closed = false;
  const close = () => {
    if (closed) return;
    closed = true;
    // Blur først, så et felt som er endret, får `change` (og blir lagret) før det fjernes.
    if (document.activeElement instanceof HTMLElement && el.contains(document.activeElement)) {
      document.activeElement.blur();
    }
    backdrop.remove();
    document.removeEventListener("keydown", onKey, true);
    onClose?.();
    previousFocus?.focus();
  };
  const onKey = (event: KeyboardEvent) => {
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      close();
    }
  };
  document.addEventListener("keydown", onKey, true);
  backdrop.addEventListener("mousedown", (event) => {
    if (event.target === backdrop) close();
  });
  return { el, close };
}

/** En høyreklikkmeny ved musepekeren. */
export function contextMenu(event: MouseEvent, items: Array<{ label: string; action: () => void; danger?: boolean }>) {
  event.preventDefault();
  document.querySelector(".context-menu")?.remove();
  const menu = document.createElement("div");
  menu.className = "context-menu";
  for (const item of items) {
    const button = document.createElement("button");
    button.type = "button";
    button.textContent = item.label;
    if (item.danger) button.classList.add("danger");
    button.addEventListener("click", () => {
      menu.remove();
      item.action();
    });
    menu.append(button);
  }
  document.body.append(menu);
  const { innerWidth, innerHeight } = window;
  const rect = menu.getBoundingClientRect();
  menu.style.left = `${Math.min(event.clientX, innerWidth - rect.width - 4)}px`;
  menu.style.top = `${Math.min(event.clientY, innerHeight - rect.height - 4)}px`;
  const dismiss = (e: Event) => {
    if (e instanceof KeyboardEvent && e.key !== "Escape") return;
    if (e.target instanceof Node && menu.contains(e.target)) return;
    menu.remove();
    document.removeEventListener("mousedown", dismiss, true);
    document.removeEventListener("keydown", dismiss, true);
  };
  document.addEventListener("mousedown", dismiss, true);
  document.addEventListener("keydown", dismiss, true);
}
