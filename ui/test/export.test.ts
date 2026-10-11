import { describe, expect, it } from "vitest";
import { parseSlide } from "../src/export.ts";

describe("parseSlide (HTML til PowerPoint)", () => {
  it("overskrift og avsnitt med formatering", () => {
    expect(parseSlide("<h1>Påske</h1>\n<p><strong>Herren</strong> er <em>god</em></p>")).toEqual([
      { kind: "heading", level: 1, runs: [{ text: "Påske" }] },
      {
        kind: "paragraph",
        runs: [{ text: "Herren", bold: true }, { text: " er " }, { text: "god", italic: true }],
      },
    ]);
  });

  it("parallelle oversettelser blir kolonner, og versnumre hevet skrift", () => {
    const html =
      '<div class="bibel-parallell"><p><sup>16</sup> For så har Gud</p><p><sup>16</sup> For God so loved</p></div>';
    expect(parseSlide(html)).toEqual([
      {
        kind: "columns",
        columns: [
          [{ text: "16", superscript: true }, { text: " For så har Gud" }],
          [{ text: "16", superscript: true }, { text: " For God so loved" }],
        ],
      },
    ]);
  });

  it("sitater åpnes, og bilder blir egne blokker etter teksten", () => {
    const html =
      '<blockquote><p>Et sitat<br>over to linjer</p></blockquote><p>Se her <span class="internal-embed image-embed"><img src="asset://localhost/a.png"></span></p>';
    expect(parseSlide(html)).toEqual([
      { kind: "paragraph", runs: [{ text: "Et sitat", lineBreak: true }, { text: "over to linjer" }] },
      { kind: "paragraph", runs: [{ text: "Se her" }] },
      { kind: "image", src: "asset://localhost/a.png" },
    ]);
  });

  it("lister blir avsnitt med punkt, og skript fjernes", () => {
    expect(parseSlide("<ul><li>En</li><li>To</li></ul><script>alert(1)</script>")).toEqual([
      { kind: "paragraph", runs: [{ text: "En" }], bullet: true },
      { kind: "paragraph", runs: [{ text: "To" }], bullet: true },
    ]);
  });
});
