//! Eksport til PDF: hver slide lages som en egen PDF av nettleservinduet (appen gjør
//! det, se `src-tauri`), og her slås de sammen til én fil med én slide per side, skalert
//! til samme sidestørrelse (full HD i eksporten).

use lopdf::{dictionary, Document, Object, ObjectId, Stream};

/// Sidestørrelsen i eksporten: full HD i bredformat (16:9), i punkter.
pub const FULL_HD: (f32, f32) = (1920.0, 1080.0);

/// Egenskaper en side arver fra `Pages` over seg, og som må med når den flyttes.
const INHERITED: [&[u8]; 4] = [b"Resources", b"MediaBox", b"CropBox", b"Rotate"];

/// Slår sammen PDF-ene til én, med sidene i samme rekkefølge. Med `size` skaleres hver side
/// (som vektor, så ingenting blir uskarpt) til den størrelsen, midt på siden.
pub fn merge(documents: &[Vec<u8>], size: Option<(f32, f32)>) -> Result<Vec<u8>, String> {
    let mut merged = Document::with_version("1.7");
    let pages_id = merged.new_object_id();
    let mut kids: Vec<Object> = Vec::new();

    for data in documents {
        let mut document = Document::load_mem(data).map_err(|e| format!("Kunne ikke lese PDF-siden: {e}"))?;
        document.renumber_objects_with(merged.max_id + 1);
        let page_ids: Vec<ObjectId> = document.get_pages().into_values().collect();
        for &page_id in &page_ids {
            let inherited = inherited_attributes(&document, page_id);
            let page = document
                .get_object_mut(page_id)
                .and_then(Object::as_dict_mut)
                .map_err(|e| format!("Ugyldig PDF-side: {e}"))?;
            for (key, value) in inherited {
                if !page.has(&key) {
                    page.set(key, value);
                }
            }
            page.set("Parent", pages_id);
            kids.push(page_id.into());
        }
        if let Some(size) = size {
            for &page_id in &page_ids {
                scale_page(&mut document, page_id, size)?;
            }
        }
        // Sidene får ny `Pages` og ny `Catalog`; de gamle (og alt annet som bare hører til dem) tas ikke med.
        for (id, object) in document.objects {
            let kind = object.type_name().unwrap_or(b"");
            if kind != b"Catalog" && kind != b"Pages" {
                merged.objects.insert(id, object);
            }
        }
        merged.max_id = merged.max_id.max(document.max_id);
    }

    let count = kids.len() as i64;
    merged.objects.insert(pages_id, Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => kids, "Count" => count }));
    let catalog_id = merged.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    merged.trailer.set("Root", catalog_id);
    merged.prune_objects();
    merged.compress();

    let mut out = Vec::new();
    merged.save_to(&mut out).map_err(|e| format!("Kunne ikke lage PDF-en: {e}"))?;
    Ok(out)
}

/// Skalerer innholdet på siden til `size` (bevarer forholdet mellom bredde og høyde, midt på).
fn scale_page(document: &mut Document, page_id: ObjectId, (width, height): (f32, f32)) -> Result<(), String> {
    let invalid = |e: lopdf::Error| format!("Ugyldig PDF-side: {e}");
    let page = document.get_dictionary(page_id).map_err(invalid)?;
    let media_box: Vec<f32> = page
        .get(b"MediaBox")
        .and_then(Object::as_array)
        .map_err(invalid)?
        .iter()
        .map(|value| value.as_float().unwrap_or(0.0))
        .collect();
    let [x0, y0, x1, y1] = media_box[..] else { return Err("Ugyldig sidestørrelse i PDF-en.".to_string()) };
    let (w, h) = (x1 - x0, y1 - y0);
    if w <= 0.0 || h <= 0.0 {
        return Err("Ugyldig sidestørrelse i PDF-en.".to_string());
    }
    let scale = (width / w).min(height / h);
    let tx = (width - w * scale) / 2.0 - x0 * scale;
    let ty = (height - h * scale) / 2.0 - y0 * scale;
    let contents: Vec<Object> = match page.get(b"Contents") {
        Ok(Object::Array(parts)) => parts.clone(),
        Ok(part) => vec![part.clone()],
        Err(_) => Vec::new(),
    };
    // Innholdet pakkes inn i `q … cm` og `Q`, som skalerer og flytter alt som tegnes.
    let before = document.add_object(Stream::new(dictionary! {}, format!("q {scale} 0 0 {scale} {tx} {ty} cm\n").into_bytes()));
    let after = document.add_object(Stream::new(dictionary! {}, b"\nQ\n".to_vec()));
    let page = document.get_object_mut(page_id).and_then(Object::as_dict_mut).map_err(invalid)?;
    page.set("Contents", [vec![before.into()], contents, vec![after.into()]].concat());
    page.set("MediaBox", vec![0.into(), 0.into(), width.into(), height.into()]);
    for key in [&b"CropBox"[..], b"BleedBox", b"TrimBox", b"ArtBox"] {
        page.remove(key);
    }
    // Lenker ville stått feil etter skaleringen; slidene har ingen lenker som skal virke i PDF-en.
    page.remove(b"Annots");
    Ok(())
}

/// Det siden arver fra `Pages`-nodene over seg (den nærmeste vinner).
fn inherited_attributes(document: &Document, page_id: ObjectId) -> Vec<(Vec<u8>, Object)> {
    let mut found: Vec<(Vec<u8>, Object)> = Vec::new();
    let mut parent = document.get_dictionary(page_id).ok().and_then(|page| page.get(b"Parent").ok()?.as_reference().ok());
    while let Some(id) = parent {
        let Ok(node) = document.get_dictionary(id) else { break };
        for key in INHERITED {
            if !found.iter().any(|(k, _)| k == key) {
                if let Ok(value) = node.get(key) {
                    found.push((key.to_vec(), value.clone()));
                }
            }
        }
        parent = node.get(b"Parent").ok().and_then(|p| p.as_reference().ok());
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::content::{Content, Operation};

    /// En PDF med én side der teksten står, og MediaBox arves fra `Pages`, som WebKit kan gjøre.
    fn page(text: &str) -> Vec<u8> {
        let mut document = Document::with_version("1.5");
        let pages_id = document.new_object_id();
        let font_id = document.add_object(dictionary! { "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica" });
        let content = Content {
            operations: vec![
                Operation::new("BT", vec![]),
                Operation::new("Tf", vec!["F1".into(), 24.into()]),
                Operation::new("Td", vec![100.into(), 600.into()]),
                Operation::new("Tj", vec![Object::string_literal(text)]),
                Operation::new("ET", vec![]),
            ],
        };
        let content_id = document.add_object(Stream::new(dictionary! {}, content.encode().unwrap()));
        let page_id = document.add_object(dictionary! { "Type" => "Page", "Parent" => pages_id, "Contents" => content_id });
        document.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! {
                "Type" => "Pages",
                "Kids" => vec![page_id.into()],
                "Count" => 1,
                "MediaBox" => vec![0.into(), 0.into(), 960.into(), 700.into()],
                "Resources" => dictionary! { "Font" => dictionary! { "F1" => font_id } },
            }),
        );
        let catalog_id = document.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
        document.trailer.set("Root", catalog_id);
        let mut out = Vec::new();
        document.save_to(&mut out).unwrap();
        out
    }

    fn media_box(document: &Document, page: u32) -> Vec<f32> {
        let page = document.get_dictionary(document.get_pages()[&page]).unwrap();
        page.get(b"MediaBox").unwrap().as_array().unwrap().iter().map(|v| v.as_float().unwrap()).collect()
    }

    #[test]
    fn sidene_skaleres_til_full_hd_midt_pa_siden() {
        let merged = Document::load_mem(&merge(&[page("En"), page("To")], Some(FULL_HD)).unwrap()).unwrap();
        assert_eq!(merged.get_pages().len(), 2);
        assert_eq!(media_box(&merged, 1), [0.0, 0.0, 1920.0, 1080.0]);
        // 960 × 700 får plass i høyden: skala 1080/700, og like mye luft på hver side i bredden.
        let content = String::from_utf8(merged.get_page_content(merged.get_pages()[&1])).unwrap();
        let scale = 1080.0_f32 / 700.0;
        let tx = (1920.0 - 960.0 * scale) / 2.0;
        assert!(content.starts_with(&format!("q {scale} 0 0 {scale} {tx} 0 cm")), "{content}");
        assert!(content.trim_end().ends_with('Q'));
        assert_eq!(merged.extract_text(&[2]).unwrap().trim(), "To");
    }

    #[test]
    fn sidene_kommer_i_rekkefolge_med_storrelse_og_skrift() {
        let merged = Document::load_mem(&merge(&[page("En"), page("To"), page("Tre")], None).unwrap()).unwrap();
        let pages = merged.get_pages();
        assert_eq!(pages.len(), 3);
        for (i, text) in ["En", "To", "Tre"].iter().enumerate() {
            assert_eq!(merged.extract_text(&[i as u32 + 1]).unwrap().trim(), *text);
            let page = merged.get_dictionary(pages[&(i as u32 + 1)]).unwrap();
            let media_box: Vec<f32> =
                page.get(b"MediaBox").unwrap().as_array().unwrap().iter().map(|v| v.as_float().unwrap()).collect();
            assert_eq!(media_box, [0.0, 0.0, 960.0, 700.0]);
        }
    }
}
