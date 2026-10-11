import { describe, expect, it } from "vitest";
import { sanitizeHtml } from "../src/sanitize.ts";

function clean(html: string): string {
  const div = document.createElement("div");
  div.append(sanitizeHtml(html));
  return div.innerHTML;
}

describe("sanitizeHtml", () => {
  it("fjerner skript og elementer som laster andre sider", () => {
    expect(clean('<p>Tekst</p><script>alert(1)</script><iframe src="x"></iframe>')).toBe("<p>Tekst</p>");
  });

  it("fjerner hendelsesattributter", () => {
    expect(clean('<img src="a.png" onerror="alert(1)">')).toBe('<img src="a.png">');
  });

  it("fjerner javascript:-lenker, også med kontrolltegn i skjemaet", () => {
    expect(clean('<a href="javascript:alert(1)">x</a>')).toBe("<a>x</a>");
    expect(clean('<a href="java\tscript:alert(1)">x</a>')).toBe("<a>x</a>");
  });

  it("beholder vanlige lenker, bilder i vaulten og data-bilder unntatt SVG", () => {
    expect(clean('<a href="https://bibel.no">x</a>')).toBe('<a href="https://bibel.no">x</a>');
    expect(clean('<img src="asset://localhost/%2Fv%2Fa.png">')).toBe('<img src="asset://localhost/%2Fv%2Fa.png">');
    expect(clean('<img src="data:image/png;base64,AA">')).toBe('<img src="data:image/png;base64,AA">');
    expect(clean('<img src="data:image/svg+xml;base64,AA">')).toBe("<img>");
  });
});
