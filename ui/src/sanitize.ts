// HTML fra markdown (lesevisningen og slidene) settes inn med innerHTML. Notatene
// kan inneholde rå HTML, og skript i vinduet kan kalle alle kommandoene i appen
// (skrive og slette filer), så skript og hendelser fjernes først, som i Obsidian.

/** Elementer som kan kjøre kode eller laste inn andre sider. */
const BLOCKED = "script, iframe, frame, frameset, object, embed, applet, base, meta, link, form, noscript, template";

/** Attributter med en URL. `javascript:` og lignende fjernes fra dem. */
const URL_ATTRIBUTES = new Set(["href", "src", "xlink:href", "action", "formaction", "poster", "background"]);

/** URL-er som er trygge å følge eller laste: vanlige lenker, bilder i vaulten og interne lenker. */
function safeUrl(name: string, value: string): boolean {
  // Kontrolltegn og mellomrom ignoreres av nettleseren foran skjemaet («java\tscript:»).
  const url = value.replace(/[\u0000- \u007f]/g, "").toLowerCase();
  const scheme = /^([a-z][a-z0-9+.-]*):/.exec(url)?.[1];
  if (!scheme) return true; // Relativ lenke eller #anker.
  if (["http", "https", "mailto", "tel", "asset"].includes(scheme)) return true;
  return scheme === "data" && name === "src" && url.startsWith("data:image/") && !url.startsWith("data:image/svg");
}

/** `html` uten skript, hendelsesattributter og farlige URL-er. */
export function sanitizeHtml(html: string): DocumentFragment {
  const template = document.createElement("template");
  template.innerHTML = html;
  const content = template.content;
  for (const el of content.querySelectorAll(BLOCKED)) el.remove();
  for (const el of content.querySelectorAll("*")) {
    for (const { name, value } of [...el.attributes]) {
      const lower = name.toLowerCase();
      if (lower.startsWith("on") || lower === "srcdoc" || lower === "srcset") el.removeAttribute(name);
      else if (URL_ATTRIBUTES.has(lower) && !safeUrl(lower, value)) el.removeAttribute(name);
    }
  }
  return content;
}

/** Setter inn HTML-en i `el` uten skript (i stedet for `el.innerHTML = html`). */
export function setSafeHtml(el: Element, html: string) {
  el.replaceChildren(sanitizeHtml(html));
}
