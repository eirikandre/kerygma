import { describe, expect, it } from "vitest";
import { basename, folderOf, presentationSource } from "../src/api.ts";
import { imageInsertion, imageOnLine, isImageFile } from "../src/images.ts";

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


describe("bilder", () => {
  it("kjenner igjen bildefiler og bilder som står alene på en linje", () => {
    expect(isImageFile("Kart.PNG")).toBe(true);
    expect(isImageFile("notat.txt")).toBe(false);
    expect(isImageFile("png")).toBe(false);
    expect(imageOnLine("![[kart 2.png]]")).toBe("kart 2.png");
    expect(imageOnLine("  ![[kart.png|300]] ")).toBe("kart.png");
    expect(imageOnLine("![Jerusalem](bilder/jerusalem.jpg)")).toBe("bilder/jerusalem.jpg");
    expect(imageOnLine("Tekst ![[kart.png]]")).toBeUndefined();
    expect(imageOnLine("![[Annet manus]]")).toBeUndefined();
  });

  it("bildet blir et eget avsnitt med tomme linjer mot teksten rundt", () => {
    expect(imageInsertion("", 0, ["![[a.png]]"])).toEqual({ text: "![[a.png]]", cursor: 10 });
    expect(imageInsertion("Før etter", 4, ["![[a.png]]", "![[b.png]]"])).toEqual({
      text: "\n\n![[a.png]]\n\n![[b.png]]\n\n",
      cursor: 2 + 22,
    });
    expect(imageInsertion("Slutt på linja", 14, ["![[a.png]]"])).toEqual({ text: "\n\n![[a.png]]", cursor: 12 });
  });
});
