import { beforeEach, describe, expect, it, vi } from "vitest";

// Disken er et kart fra sti til innhold; kommandoene til Rust går hit.
const disk = new Map<string, string>();
const notices: string[] = [];

vi.mock("../src/api.ts", async (original) => ({
  ...(await original<typeof import("../src/api.ts")>()),
  api: {
    readNote: async (path: string) => {
      const content = disk.get(path);
      if (content === undefined) throw `Kunne ikke lese «${path}»`;
      return content;
    },
    writeNote: async (path: string, content: string) => {
      disk.set(path, content);
      return null;
    },
  },
}));
vi.mock("../src/dialogs.ts", () => ({ notice: (message: string) => notices.push(message) }));

const { OpenDocument } = await import("../src/document.ts");

/** En editor uten CodeMirror: bare teksten. */
class FakeEditor {
  value = "";
  getValue() {
    return this.value;
  }
  load(content: string) {
    this.value = content;
  }
  replace(content: string) {
    this.value = content;
  }
}

function setup() {
  const editor = new FakeEditor();
  const hooks = { reloaded: vi.fn(async () => {}), closed: vi.fn() };
  // Dokumentet bruker bare getValue, load og replace.
  const doc = new OpenDocument(editor as never, hooks);
  return { editor, hooks, doc };
}

beforeEach(() => {
  disk.clear();
  notices.length = 0;
  vi.useFakeTimers();
});

describe("OpenDocument", () => {
  it("lagrer et sekund etter siste endring, og før en annen fil åpnes", async () => {
    disk.set("a.md", "A").set("b.md", "B");
    const { editor, doc } = setup();
    await doc.open("a.md");
    editor.value = "A2";
    doc.changed();
    await vi.advanceTimersByTimeAsync(999);
    expect(disk.get("a.md")).toBe("A");
    await vi.advanceTimersByTimeAsync(1);
    expect(disk.get("a.md")).toBe("A2");

    editor.value = "A3";
    doc.changed();
    await doc.open("b.md");
    expect(disk.get("a.md")).toBe("A3");
    expect(editor.value).toBe("B");
  });

  it("laster inn endringer utenfra når editoren ikke har ulagrede endringer", async () => {
    disk.set("a.md", "A");
    const { editor, hooks, doc } = setup();
    await doc.open("a.md");
    disk.set("a.md", "Fra Obsidian");
    await doc.externalChange(["b.md"]);
    expect(editor.value).toBe("A");
    await doc.externalChange(["a.md"]);
    expect(editor.value).toBe("Fra Obsidian");
    expect(hooks.reloaded).toHaveBeenCalledOnce();
  });

  it("ulagrede endringer går foran endringer utenfra, med en melding", async () => {
    disk.set("a.md", "A");
    const { editor, doc } = setup();
    await doc.open("a.md");
    editor.value = "Min versjon";
    doc.changed();
    disk.set("a.md", "Fra Obsidian");
    await doc.externalChange(["a.md"]);
    expect(editor.value).toBe("Min versjon");
    expect(notices).toHaveLength(1);
    await vi.advanceTimersByTimeAsync(1000);
    expect(disk.get("a.md")).toBe("Min versjon");
  });

  it("lukkes når fila er slettet utenfra", async () => {
    disk.set("a.md", "A");
    const { hooks, doc } = setup();
    await doc.open("a.md");
    disk.delete("a.md");
    await doc.externalChange(["a.md"]);
    expect(doc.path).toBeUndefined();
    expect(hooks.closed).toHaveBeenCalledOnce();
  });

  it("følger med når fila eller mappa flyttes", async () => {
    disk.set("Prekener/a.md", "A");
    const { doc } = setup();
    await doc.open("Prekener/a.md");
    doc.moved("Prekener", "Arkiv/Prekener");
    expect(doc.path).toBe("Arkiv/Prekener/a.md");
    doc.moved("Arkiv/Prekener/a.md", "Arkiv/Prekener/b.md");
    expect(doc.path).toBe("Arkiv/Prekener/b.md");
  });
});
