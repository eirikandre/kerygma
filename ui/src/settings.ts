// Innstillingene, med de samme navnene og beskrivelsene som SlideCreatorSettingTab i pluginen.
import { api, errorMessage, type Settings, type Theme } from "./api.ts";
import { FONTS, MAX_SIZE, MIN_SIZE, THEMES, getAppearance, setAppearance } from "./appearance.ts";
import { notice, openModal } from "./dialogs.ts";
import { setIcon } from "./icons.ts";

const DEFAULTS: Settings = {
  suffix: " - presentasjon",
  openCopy: true,
  maxChars: 350,
  addTranslation: true,
  translation: "kjv",
  defaultTranslation: "nb88",
  aliases: "n=nb88,e=kjv,ne=nb88+kjv",
  importFolder: "",
};

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
    desc: "Hvor «Importer Word-manus» legger markdown-filene. Tom: mappa til notatet som er åpent.",
    type: "text",
    placeholder: "Manus",
  },
  {
    key: "openCopy",
    name: "Åpne presentasjonen",
    desc: "Åpne presentasjonen etter at den er laget.",
    type: "toggle",
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
          record[field.key] = input.value;
        }
        save();
      });
    }
    row.append(info, input);
    modal.el.append(row);
  }

  const hotkeys = document.createElement("p");
  hotkeys.className = "setting-desc";
  hotkeys.textContent =
    "Hurtigtaster: Cmd/Ctrl + Shift + M lager presentasjon, Cmd/Ctrl + Shift + P viser den, " +
    "Cmd/Ctrl + Shift + B setter inn bibelvers, Cmd/Ctrl + E bytter til lesevisning og Cmd/Ctrl + N lager en ny note. " +
    "Cmd/Ctrl + B, I og U gir fet, kursiv og understreket tekst, og Cmd/Ctrl + Shift + X overstreket. " +
    "Cmd/Ctrl + Shift + F søker i alle notatene.";
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
