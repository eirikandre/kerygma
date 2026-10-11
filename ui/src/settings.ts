// Innstillingene: for vaulten (`Settings`, i .kerygma/settings.json) og for appen (utseendet, i app.json).
import { api, errorMessage, type Settings, type Theme } from "./api.ts";
import { DEFAULT_SETTINGS } from "./bindings.ts";
import { FONTS, MAX_SIZE, MIN_SIZE, THEMES, getAppearance, setAppearance } from "./appearance.ts";
import { notice, openModal } from "./dialogs.ts";
import { setIcon } from "./icons.ts";
import { SLIDE_FONTS, SLIDE_THEMES, applySlideStyle, loadSlideFonts, slideTheme } from "./slideThemes.ts";

/** Standardverdiene, de samme som `Settings::default` i kerygma-core. */
const DEFAULTS: Settings = { ...DEFAULT_SETTINGS };

/** Felt `Settings::sanitized` trimmer, og felt som får standardverdien når de er tomme. */
const TRIMMED = new Set<keyof Settings>(["translation", "defaultTranslation", "importFolder"]);
const REQUIRED = new Set<keyof Settings>(["suffix", "translation", "defaultTranslation"]);

type Field =
  | { key: keyof Settings; name: string; desc: string; type: "text"; placeholder?: string }
  | { key: keyof Settings; name: string; desc: string; type: "toggle" }
  | { key: keyof Settings; name: string; desc: string; type: "number" };

const FIELDS: Field[] = [
  {
    key: "suffix",
    name: "Postfix for presentasjon",
    desc: `Legges til navnet på presentasjonen. Standard: "${DEFAULTS.suffix}".`,
    type: "text",
  },
  {
    key: "maxChars",
    name: "Maks tegn per slide",
    desc: `Lengre bibeltekst deles over flere slides. Standard: ${DEFAULTS.maxChars}.`,
    type: "number",
  },
  {
    key: "addTranslation",
    name: "Legg til oversettelse",
    desc: "Vers som bare står på norsk i manuset, får en oversettelse ved siden av.",
    type: "toggle",
  },
  {
    key: "translation",
    name: "Oversettelse",
    desc: "Oversettelsen som legges ved siden av, som id (kjv, nb88, nb24) eller snarvei. Standard: kjv.",
    type: "text",
  },
  {
    key: "defaultTranslation",
    name: "Standardoversettelse",
    desc: `Brukes når referansen ikke har prefiks, som "joh3:16". Standard: ${DEFAULTS.defaultTranslation}.`,
    type: "text",
  },
  {
    key: "aliases",
    name: "Snarveier",
    desc:
      'Korte navn på én eller flere oversettelser, som i espanso-oppsettet. "ne=nb88+kjv" gjør at "ne:joh3:16" gir begge. ' +
      `Standard: ${DEFAULTS.aliases}.`,
    type: "text",
  },
  {
    key: "importFolder",
    name: "Mappe for importerte Word-manus",
    desc: "Hvor «Importer Word-manus» legger markdown-filene. Tom: mappa til manuset som er åpent. «Importer Word-manus hit» på en mappe i treet går foran.",
    type: "text",
    placeholder: "Manus",
  },
];

export async function openSettings() {
  let settings: Settings;
  try {
    settings = await api.getSettings();
  } catch (error) {
    notice(errorMessage(error), { error: true });
    return;
  }
  const modal = openModal("settings-modal");
  // Topplinje med × og bunnlinje med «Lukk», som blir stående når vinduet rulles.
  const header = document.createElement("div");
  header.className = "settings-header";
  const heading = document.createElement("h2");
  heading.textContent = "Innstillinger";
  const closeIcon = document.createElement("button");
  closeIcon.type = "button";
  closeIcon.className = "icon-button";
  setIcon(closeIcon, "x", "Lukk (Esc)");
  closeIcon.addEventListener("click", () => modal.close());
  header.append(heading, closeIcon);
  modal.el.append(header);

  section(modal.el, "Utseende", "Gjelder appen i alle mapper.");
  appearanceSettings(modal.el);
  section(modal.el, "Presentasjoner og bibel", "Gjelder mappa som er åpen.");

  // Lagres når et felt endres, som i Obsidian.
  let saving = Promise.resolve();
  const save = () => {
    const snapshot = { ...settings };
    saving = saving.then(async () => {
      try {
        for (const warning of await api.setSettings(snapshot)) notice(warning, { error: true });
      } catch (error) {
        notice(errorMessage(error), { error: true });
      }
    });
  };

  for (const field of FIELDS) {
    const row = document.createElement("label");
    row.className = "setting";
    const info = document.createElement("div");
    info.className = "setting-info";
    info.innerHTML = `<div class="setting-name"></div><div class="setting-desc"></div>`;
    info.querySelector(".setting-name")!.textContent = field.name;
    info.querySelector(".setting-desc")!.textContent = field.desc;
    const input = document.createElement("input");
    if (field.type === "toggle") {
      input.type = "checkbox";
      input.className = "toggle";
      input.checked = settings[field.key] as boolean;
      input.addEventListener("change", () => {
        (settings as unknown as Record<string, unknown>)[field.key] = input.checked;
        save();
      });
    } else {
      input.type = "text";
      input.value = String(settings[field.key]);
      if (field.type === "text" && field.placeholder) input.placeholder = field.placeholder;
      input.addEventListener("change", () => {
        const record = settings as unknown as Record<string, unknown>;
        if (field.type === "number") {
          const value = Number.parseInt(input.value, 10);
          record[field.key] = value > 0 ? value : DEFAULTS.maxChars;
          input.value = String(record[field.key]);
        } else {
          // Som `Settings::sanitized` i Rust, så feltet viser det som faktisk lagres.
          const value = TRIMMED.has(field.key) ? input.value.trim() : input.value;
          record[field.key] = REQUIRED.has(field.key) && value.trim() === "" ? DEFAULTS[field.key] : value;
          input.value = String(record[field.key]);
        }
        save();
      });
    }
    row.append(info, input);
    modal.el.append(row);
  }

  section(
    modal.el,
    "Utseende på presentasjonen",
    "Gjelder mappa som er åpen. Obsidian bruker alltid slides.css, som er temaet «Kveld».",
  );
  slideSettings(modal.el, settings, save);

  const hotkeys = document.createElement("p");
  hotkeys.className = "setting-desc";
  hotkeys.textContent =
    "Hurtigtaster: Cmd/Ctrl + Shift + M lager presentasjon, Cmd/Ctrl + Shift + P viser den, " +
    "Cmd/Ctrl + Shift + B setter inn bibelvers, Cmd/Ctrl + E bytter til lesevisning og Cmd/Ctrl + N lager et nytt manus. " +
    "Cmd/Ctrl + B, I og U gir fet, kursiv og understreket tekst, Cmd/Ctrl + Shift + X overstreket og Cmd/Ctrl + Shift + . sitat. " +
    "Cmd/Ctrl + Shift + F søker i alle manusene.";
  modal.el.append(hotkeys);

  const footer = document.createElement("div");
  footer.className = "settings-footer";
  const saved = document.createElement("span");
  saved.className = "setting-desc";
  saved.textContent = "Endringer lagres med en gang.";
  const close = document.createElement("button");
  close.type = "button";
  close.className = "primary";
  close.textContent = "Lukk";
  close.addEventListener("click", () => modal.close());
  footer.append(saved, close);
  modal.el.append(footer);
}

function section(parent: HTMLElement, title: string, desc: string) {
  const heading = document.createElement("h3");
  heading.className = "setting-section";
  heading.textContent = title;
  const info = document.createElement("div");
  info.className = "setting-desc";
  info.textContent = desc;
  parent.append(heading, info);
}

function settingRow(name: string, desc: string, control: HTMLElement): HTMLElement {
  const row = document.createElement("label");
  row.className = "setting";
  const info = document.createElement("div");
  info.className = "setting-info";
  info.innerHTML = `<div class="setting-name"></div><div class="setting-desc"></div>`;
  info.querySelector(".setting-name")!.textContent = name;
  info.querySelector(".setting-desc")!.textContent = desc;
  row.append(info, control);
  return row;
}

/** Tema, skrift og størrelse. Endres med en gang, så appen bak vinduet viser valget. */
function appearanceSettings(parent: HTMLElement) {
  const current = getAppearance();

  const theme = document.createElement("select");
  for (const option of THEMES) {
    const el = document.createElement("option");
    el.value = option.id;
    el.textContent = option.name;
    theme.append(el);
  }
  theme.value = current.theme;
  theme.addEventListener("change", () => setAppearance({ theme: theme.value as Theme }));

  const font = document.createElement("select");
  for (const option of FONTS) {
    const el = document.createElement("option");
    el.value = option.id;
    el.textContent = option.name;
    el.style.fontFamily = option.stack;
    font.append(el);
  }
  font.value = current.font;
  font.addEventListener("change", () => setAppearance({ font: font.value }));

  const size = document.createElement("input");
  size.type = "number";
  size.min = String(MIN_SIZE);
  size.max = String(MAX_SIZE);
  size.step = "1";
  size.value = String(current.fontSize);
  size.addEventListener("input", () => {
    const value = Number.parseInt(size.value, 10);
    if (value >= MIN_SIZE && value <= MAX_SIZE) setAppearance({ fontSize: value });
  });
  size.addEventListener("change", () => {
    setAppearance({ fontSize: Number.parseInt(size.value, 10) || current.fontSize });
    size.value = String(getAppearance().fontSize);
  });

  parent.append(
    settingRow("Tema", "Lyst eller mørkt. Standard: følg systemet.", theme),
    settingRow("Skrift", "Skrifttypen i editoren og lesevisningen.", font),
    settingRow(
      "Skriftstørrelse",
      `I piksler, fra ${MIN_SIZE} til ${MAX_SIZE}. Du kan også holde inne Cmd/Ctrl og rulle med musa over teksten.`,
      size,
    ),
  );
}

/** Tema og skrifter for presentasjonen, med en liten forhåndsvisning av hvert tema. */
function slideSettings(parent: HTMLElement, settings: Settings, save: () => void) {
  void loadSlideFonts();
  const grid = document.createElement("div");
  grid.className = "slide-themes";
  grid.setAttribute("role", "radiogroup");
  grid.setAttribute("aria-label", "Tema");
  const desc = document.createElement("div");
  desc.className = "setting-desc slide-theme-desc";

  const cards = SLIDE_THEMES.map((theme) => {
    const card = document.createElement("button");
    card.type = "button";
    card.className = "slide-theme";
    card.setAttribute("role", "radio");
    card.innerHTML =
      `<div class="slide-theme-preview"><div class="slide-theme-heading">Overskrift</div>` +
      `<div class="slide-theme-quote">Herren er min hyrde, jeg mangler ingen ting.</div></div>` +
      `<div class="slide-theme-name"></div>`;
    card.querySelector(".slide-theme-name")!.textContent = theme.name;
    card.addEventListener("click", () => {
      settings.slideTheme = theme.id;
      update();
      save();
    });
    grid.append(card);
    return { theme, card };
  });

  const fontSelect = (key: "headingFont" | "quoteFont") => {
    const select = document.createElement("select");
    select.addEventListener("change", () => {
      settings[key] = select.value;
      update();
      save();
    });
    return select;
  };
  const headingFont = fontSelect("headingFont");
  const quoteFont = fontSelect("quoteFont");

  /** Fyller lista på nytt, så «Temaets skrift» viser skriften til temaet som er valgt. */
  const fillFonts = (select: HTMLSelectElement, value: string, themeFont: string) => {
    select.replaceChildren();
    const themeOption = document.createElement("option");
    themeOption.value = "";
    themeOption.textContent = `Temaets skrift (${SLIDE_FONTS.find((f) => f.id === themeFont)?.name ?? themeFont})`;
    select.append(themeOption);
    for (const font of SLIDE_FONTS) {
      const option = document.createElement("option");
      option.value = font.id;
      option.textContent = font.name;
      option.style.fontFamily = font.stack;
      select.append(option);
    }
    select.value = SLIDE_FONTS.some((f) => f.id === value) ? value : "";
  };

  const update = () => {
    const selected = slideTheme(settings.slideTheme);
    for (const { theme, card } of cards) {
      applySlideStyle(card.querySelector(".slide-theme-preview")!, {
        theme: theme.id,
        headingFont: settings.headingFont,
        quoteFont: settings.quoteFont,
      });
      card.setAttribute("aria-checked", String(theme === selected));
    }
    desc.textContent = selected.desc;
    fillFonts(headingFont, settings.headingFont, selected.headingFont);
    fillFonts(quoteFont, settings.quoteFont, selected.quoteFont);
  };
  update();

  const themeRow = document.createElement("div");
  themeRow.className = "setting slide-theme-setting";
  themeRow.append(grid, desc);
  parent.append(
    themeRow,
    settingRow("Skrift i overskrifter", "Overskriftene på slidene og versnumrene.", headingFont),
    settingRow("Skrift i sitater", "Sitater og bibelvers på slidene.", quoteFont),
  );
}
