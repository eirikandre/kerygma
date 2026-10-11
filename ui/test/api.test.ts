import { describe, expect, it } from "vitest";
import { basename, folderOf, presentationSource } from "../src/api.ts";

describe("stier", () => {
  it("navn og mappe", () => {
    expect(basename("Prekener/Påske.md")).toBe("Påske");
    expect(basename("bilde.png")).toBe("bilde.png");
    expect(folderOf("Prekener/2024/Påske.md")).toBe("Prekener/2024");
    expect(folderOf("Påske.md")).toBe("");
  });
});

describe("presentationSource", () => {
  // Samme tilfeller som testene av `presentation_source` i kerygma-core.
  it("finner manuset i markøren, også etter frontmatter og med CRLF", () => {
    expect(presentationSource("<!-- kerygma-presentasjon: Påske.md -->\n\n# Påske\n")).toBe("Påske.md");
    expect(presentationSource("---\ntags: a\n---\n\n<!-- kerygma-presentasjon: Påske.md -->\n")).toBe("Påske.md");
    expect(presentationSource("<!-- kerygma-presentasjon: Påske.md -->\r\n\r\n# Påske\r\n")).toBe("Påske.md");
  });

  it("andre kommentarer er ikke markøren", () => {
    expect(presentationSource("# Påske\n<!-- annen kommentar -->\n")).toBeUndefined();
  });
});
