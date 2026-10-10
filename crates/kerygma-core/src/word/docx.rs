//! Leser avsnittene i en .docx-fil: tekst, fet og kursiv, stil og om
//! avsnittet er et listepunkt. Skrevet etter `word-to-markdown/src/docx.ts`.

use std::io::{Cursor, Read};

use quick_xml::events::{BytesStart, Event};
use quick_xml::{Reader, XmlVersion};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
}

impl Run {
    pub fn plain(text: impl Into<String>) -> Run {
        Run { text: text.into(), bold: false, italic: false }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Paragraph {
    pub runs: Vec<Run>,
    /// Stil-id-en, som "Heading1" eller "Overskrift1".
    pub style: Option<String>,
    pub list_item: bool,
}

pub fn read_docx(data: &[u8]) -> Result<Vec<Paragraph>, String> {
    let mut archive = zip::ZipArchive::new(Cursor::new(data)).map_err(|_| "ikke et zip-arkiv".to_string())?;
    let mut file = archive
        .by_name("word/document.xml")
        .map_err(|_| "fant ikke word/document.xml - er det en .docx-fil?".to_string())?;
    let mut xml = String::new();
    file.read_to_string(&mut xml).map_err(|e| format!("kunne ikke lese word/document.xml: {e}"))?;
    parse_document(&xml)
}

fn value(element: &BytesStart) -> Option<String> {
    element
        .attributes()
        .flatten()
        .find(|a| a.key.as_ref() == "w:val")
        .and_then(|a| a.normalized_value(XmlVersion::Implicit1_0).ok().map(|v| v.into_owned()))
}

fn parse_document(xml: &str) -> Result<Vec<Paragraph>, String> {
    let mut reader = Reader::from_str(xml);
    let mut paragraphs = Vec::new();
    let mut paragraph: Option<Paragraph> = None;
    let mut run: Option<Run> = None;
    // Inne i <w:pPr> gjelder <w:rPr> avsnittsmerket, ikke teksten.
    let mut in_paragraph_properties = false;
    let mut in_run_properties = false;
    let mut in_text = false;

    loop {
        let event = reader.read_event().map_err(|e| format!("ugyldig XML i word/document.xml: {e}"))?;
        let (element, self_closing) = match &event {
            Event::Eof => break,
            Event::Text(text) => {
                if in_text {
                    if let Some(run) = run.as_mut() {
                        run.text.push_str(&text.xml_content(XmlVersion::Implicit1_0));
                    }
                }
                continue;
            }
            Event::GeneralRef(reference) => {
                if in_text {
                    if let Some(run) = run.as_mut() {
                        let name = reference.xml_content(XmlVersion::Implicit1_0);
                        if let Some(c) = reference.resolve_char_ref().ok().flatten() {
                            run.text.push(c);
                        } else if let Some(text) = quick_xml::escape::resolve_predefined_entity(&name) {
                            run.text.push_str(text);
                        }
                    }
                }
                continue;
            }
            Event::End(end) => {
                match end.name().as_ref() {
                    "w:p" => {
                        if let Some(p) = paragraph.take() {
                            paragraphs.push(p);
                        }
                    }
                    "w:r" => {
                        if let (Some(r), Some(p)) = (run.take(), paragraph.as_mut()) {
                            p.runs.push(r);
                        }
                    }
                    "w:pPr" => in_paragraph_properties = false,
                    "w:rPr" => in_run_properties = false,
                    "w:t" => in_text = false,
                    _ => {}
                }
                continue;
            }
            Event::Start(e) => (e.clone(), false),
            Event::Empty(e) => (e.clone(), true),
            _ => continue,
        };

        let is_on = || value(&element).is_none_or(|v| !matches!(v.as_str(), "0" | "false" | "off" | "none"));
        match element.name().as_ref() {
            "w:p" => {
                if self_closing {
                    paragraphs.push(Paragraph::default());
                } else {
                    paragraph = Some(Paragraph::default());
                }
            }
            "w:pPr" => in_paragraph_properties = !self_closing,
            "w:pStyle" => {
                if let (Some(p), Some(v)) = (paragraph.as_mut(), value(&element)) {
                    if !v.is_empty() {
                        p.style = Some(v);
                    }
                }
            }
            "w:numPr" => {
                if let Some(p) = paragraph.as_mut() {
                    if in_paragraph_properties {
                        p.list_item = true;
                    }
                }
            }
            "w:r" => {
                if !self_closing {
                    run = Some(Run::plain(""));
                }
            }
            "w:rPr" => in_run_properties = !self_closing && !in_paragraph_properties,
            "w:b" => {
                if in_run_properties {
                    if let Some(r) = run.as_mut() {
                        r.bold = is_on();
                    }
                }
            }
            "w:i" => {
                if in_run_properties {
                    if let Some(r) = run.as_mut() {
                        r.italic = is_on();
                    }
                }
            }
            "w:t" => in_text = !self_closing,
            "w:tab" => {
                if !in_run_properties && !in_paragraph_properties {
                    if let Some(r) = run.as_mut() {
                        r.text.push('\t');
                    }
                }
            }
            "w:br" | "w:cr" => {
                if let Some(r) = run.as_mut() {
                    r.text.push('\n');
                }
            }
            "w:noBreakHyphen" => {
                if let Some(r) = run.as_mut() {
                    r.text.push('-');
                }
            }
            _ => {}
        }
    }
    Ok(paragraphs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leser_avsnitt_stil_fet_kursiv_og_lister() {
        let xml = r#"<?xml version="1.0"?><w:document><w:body>
<w:p><w:pPr><w:pStyle w:val="Heading1"/><w:rPr><w:b/></w:rPr></w:pPr><w:r><w:t>Tittel</w:t></w:r></w:p>
<w:p><w:r><w:rPr><w:b/></w:rPr><w:t xml:space="preserve">Fet </w:t></w:r><w:r><w:rPr><w:i w:val="1"/><w:b w:val="0"/></w:rPr><w:t>kursiv &amp; &#229;</w:t><w:tab/><w:br/></w:r></w:p>
<w:p/>
<w:p><w:pPr><w:numPr><w:ilvl w:val="0"/></w:numPr></w:pPr><w:r><w:t>Punkt</w:t></w:r></w:p>
</w:body></w:document>"#;
        let paragraphs = parse_document(xml).unwrap();
        assert_eq!(paragraphs.len(), 4);
        assert_eq!(paragraphs[0].style.as_deref(), Some("Heading1"));
        assert_eq!(paragraphs[0].runs, [Run::plain("Tittel")]);
        assert_eq!(
            paragraphs[1].runs,
            [
                Run { text: "Fet ".into(), bold: true, italic: false },
                Run { text: "kursiv & å\t\n".into(), bold: false, italic: true },
            ]
        );
        assert!(paragraphs[2].runs.is_empty());
        assert!(paragraphs[3].list_item);
    }
}
