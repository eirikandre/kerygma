// Vinduet med brukerveiledningen (Hjelp → Brukerveiledning). Teksten er bygget inn i appen
// fra docs/Brukerveiledning.md (`help_html`), med samme skrift og tema som manusene.
import { openUrl } from "@tauri-apps/plugin-opener";
import { api, errorMessage } from "./api.ts";
import { loadAppearance } from "./appearance.ts";
import { setSafeHtml } from "./sanitize.ts";
import "./styles.css";

const article = document.getElementById("help") as HTMLElement;

void loadAppearance();
api
  .helpHtml()
  .then((html) => setSafeHtml(article, html))
  .catch((error) => {
    article.textContent = errorMessage(error);
  });

// Lenker inne i veiledningen ruller dit; nettadresser åpnes i nettleseren.
article.addEventListener("click", (event) => {
  const link = (event.target as HTMLElement).closest("a");
  if (!link) return;
  event.preventDefault();
  const href = link.getAttribute("href") ?? "";
  if (href.startsWith("#")) {
    const id = decodeURIComponent(href.slice(1));
    document.getElementById(id)?.scrollIntoView({ behavior: "smooth", block: "start" });
  } else if (/^https?:/i.test(href)) {
    void openUrl(href);
  }
});
