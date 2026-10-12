// Bilder i manus: legges i bildemappa til manuset (en mappe med samme navn, ved siden av det)
// og settes inn som `![[navn.png]]`. Se `Workspace::add_image` i kerygma-core.
import { invoke } from "@tauri-apps/api/core";

/** Filendelsene som regnes som bilder, som `IMAGE_EXTENSIONS` i kerygma-core. */
export const IMAGE_EXTENSIONS = ["png", "jpg", "jpeg", "gif", "svg", "webp", "bmp", "avif"];

export function isImageFile(name: string): boolean {
  const extension = name.split(".").pop()?.toLowerCase() ?? "";
  return name.includes(".") && IMAGE_EXTENSIONS.includes(extension);
}

/** Legger et bilde som er dratt inn (fra Finder eller Utforsker) i bildemappa, og gir markdown som viser det. */
export async function addImageData(path: string, file: File): Promise<string> {
  const data = new Uint8Array(await file.arrayBuffer());
  return invoke<string>("add_image_data", data, {
    headers: { path: encodeURIComponent(path), name: encodeURIComponent(file.name) },
  });
}

/**
 * Teksten som settes inn for bildene ved `column` i en linje: hvert bilde som et eget avsnitt,
 * med tomme linjer mot tekst før og etter, så det blir en egen slide i presentasjonen.
 * `cursor` er hvor markøren skal stå etterpå, regnet fra starten av teksten.
 */
export function imageInsertion(line: string, column: number, markdowns: string[]): { text: string; cursor: number } {
  const before = line.slice(0, column);
  const after = line.slice(column);
  const prefix = before.trim() ? "\n\n" : "";
  const suffix = after.trim() ? "\n\n" : "";
  const body = markdowns.join("\n\n");
  return { text: prefix + body + suffix, cursor: prefix.length + body.length };
}

/** Et bilde som står alene på en linje: `![[kart.png]]`, `![[kart.png|300]]` eller `![tekst](mappe/kart.png)`. */
const IMAGE_LINE = new RegExp(
  String.raw`^\s*(?:!\[\[([^\]|#]+?\.(?:${IMAGE_EXTENSIONS.join("|")}))(?:[|#][^\]]*)?\]\]|!\[[^\]]*\]\(<?([^)>]+?\.(?:${IMAGE_EXTENSIONS.join("|")}))>?\))\s*$`,
  "i",
);

/** Bildet linja viser, eller undefined. */
export function imageOnLine(line: string): string | undefined {
  const match = IMAGE_LINE.exec(line);
  return match?.[1] ?? match?.[2];
}
