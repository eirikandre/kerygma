//! Markdown til HTML, for lesevisningen og presentasjonen.
//!
//! Bruker comrak (CommonMark + GFM), og gjør i tillegg det Obsidian gjør:
//! skjuler frontmatter, fjerner `%% … %%`, viser callouts, `==utheving==`,
//! `![[bilde.png|400]]` og `[[Manus|tekst]]`. Presentasjonen deles på `---`
//! (`ThematicBreak`) i AST-et, som Slides i Obsidian.

use std::sync::LazyLock;

use comrak::nodes::{AstNode, NodeValue};
use comrak::{format_html, parse_document, Arena, Options};
use regex::Regex;

use crate::vault::is_image;

/// Gir URL-en til et bilde ut fra målet i `![[mål]]` (`wiki`) eller `![](mål)`.
pub type ImageResolver<'r> = &'r dyn Fn(&str, bool) -> Option<String>;

fn options() -> Options<'static> {
    let mut options = Options::default();
    options.extension.table = true;
    options.extension.strikethrough = true;
    options.extension.tasklist = true;
    options.extension.autolink = true;
    options.extension.footnotes = true;
    options.extension.highlight = true;
    // Presentasjonen inneholder `<div class="bibel-parallell">` og `<sup>`.
    options.render.r#unsafe = true;
    // Som Obsidian uten «strenge linjeskift»: et linjeskift i teksten blir et linjeskift.
    options.render.hardbreaks = true;
    options
}

/// HTML til lesevisningen.
pub fn render_html(markdown: &str, images: ImageResolver) -> String {
    let source = prepare(markdown);
    let arena = Arena::new();
    let options = options();
    let root = parse_document(&arena, &source, &options);
    transform(&arena, root, images);
    to_html(root, &options)
}

/// HTML per slide: delt på `---`, uten tomme slides.
pub fn slides(markdown: &str, images: ImageResolver) -> Vec<String> {
    let source = prepare(markdown);
    let arena = Arena::new();
    let options = options();
    let root = parse_document(&arena, &source, &options);
    transform(&arena, root, images);

    let mut groups: Vec<Vec<&AstNode>> = vec![vec![]];
    for child in root.children() {
        if matches!(child.data.borrow().value, NodeValue::ThematicBreak) {
            groups.push(vec![]);
        } else {
            groups.last_mut().unwrap().push(child);
        }
    }
    groups
        .into_iter()
        .filter(|group| !group.is_empty())
        .map(|group| {
            let document = arena.alloc(AstNode::from(NodeValue::Document));
            for node in group {
                node.detach();
                document.append(node);
            }
            to_html(document, &options)
        })
        .collect()
}

fn to_html<'a>(root: &'a AstNode<'a>, options: &Options) -> String {
    let mut html = String::new();
    format_html(root, options, &mut html).expect("skrive HTML");
    html
}

/// Fjerner frontmatter og kommentarer, og gjør callouts om til HTML.
fn prepare(markdown: &str) -> String {
    callouts(&remove_comments(strip_frontmatter(markdown)))
}

/// Frontmatter vises ikke. Den finnes bare når første linje er nøyaktig `---`.
pub fn strip_frontmatter(markdown: &str) -> &str {
    let mut lines = markdown.split_inclusive('\n');
    let is_delimiter = |line: &str| line.trim_end_matches(['\n', '\r']) == "---";
    if !lines.next().is_some_and(is_delimiter) {
        return markdown;
    }
    let mut offset = markdown.split_inclusive('\n').next().unwrap().len();
    for line in lines {
        offset += line.len();
        if is_delimiter(line) {
            return &markdown[offset..];
        }
    }
    markdown
}

static FENCE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*(```|~~~)").unwrap());

/// Går gjennom linjene og sier om hver er inne i en kodeblokk.
fn with_fences(markdown: &str) -> Vec<(&str, bool)> {
    let mut fence: Option<String> = None;
    markdown
        .split('\n')
        .map(|line| {
            if let Some(open) = &fence {
                if line.trim().starts_with(open.as_str()) {
                    fence = None;
                }
                (line, true)
            } else if let Some(caps) = FENCE.captures(line) {
                fence = Some(caps[1].to_string());
                (line, true)
            } else {
                (line, false)
            }
        })
        .collect()
}

/// Fjerner `%% … %%`, også over flere linjer, men ikke inne i kodeblokker.
fn remove_comments(markdown: &str) -> String {
    let mut in_comment = false;
    let mut lines = Vec::new();
    for (line, in_code) in with_fences(markdown) {
        if in_code && !in_comment {
            lines.push(line.to_string());
            continue;
        }
        let mut kept = String::new();
        let mut rest = line;
        loop {
            match rest.find("%%") {
                Some(i) => {
                    if !in_comment {
                        kept.push_str(&rest[..i]);
                    }
                    in_comment = !in_comment;
                    rest = &rest[i + 2..];
                }
                None => {
                    if !in_comment {
                        kept.push_str(rest);
                    }
                    break;
                }
            }
        }
        // En linje som bare var kommentar, forsvinner helt.
        if kept.trim().is_empty() && line.contains("%%") {
            continue;
        }
        lines.push(kept);
    }
    lines.join("\n")
}

static CALLOUT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*>\s?\[!([^\]|]+)(?:\|[^\]]*)?\][+-]?\s*(.*)$").unwrap());
static QUOTE_LINE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*>\s?").unwrap());

/// `> [!type] Tittel` blir `<div class="callout" data-callout="type">` med
/// tittel og innhold. Innholdet står mellom tomme linjer, så det fortsatt er markdown.
fn callouts(markdown: &str) -> String {
    let lines = with_fences(markdown);
    let mut out: Vec<String> = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let (line, in_code) = lines[i];
        let starts_quote = out.last().is_none_or(|previous| !QUOTE_LINE.is_match(previous));
        let callout = if in_code || !starts_quote { None } else { CALLOUT.captures(line) };
        let Some(caps) = callout else {
            out.push(line.to_string());
            i += 1;
            continue;
        };
        let kind = caps[1].trim().to_lowercase();
        let title = match caps[2].trim() {
            "" => capitalize(&kind),
            title => title.to_string(),
        };
        let mut content = Vec::new();
        i += 1;
        while i < lines.len() && !lines[i].1 && QUOTE_LINE.is_match(lines[i].0) {
            content.push(QUOTE_LINE.replace(lines[i].0, "").into_owned());
            i += 1;
        }
        if out.last().is_some_and(|previous| !previous.trim().is_empty()) {
            out.push(String::new());
        }
        out.push(format!(r#"<div class="callout" data-callout="{}">"#, escape_attribute(&kind)));
        out.push(format!(r#"<div class="callout-title">{}</div>"#, escape_text(&title)));
        out.push(r#"<div class="callout-content">"#.to_string());
        out.push(String::new());
        out.extend(content);
        out.push(String::new());
        out.push("</div>".to_string());
        out.push("</div>".to_string());
        out.push(String::new());
    }
    out.join("\n")
}

fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map_or_else(String::new, |first| first.to_uppercase().chain(chars).collect())
}

static WIKILINK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(!?)\[\[([^\[\]\n]+?)\]\]").unwrap());
static SIZE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^([0-9]+)(?:x([0-9]+))?$").unwrap());
static HAS_SCHEME: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[a-zA-Z][a-zA-Z0-9+.-]*:").unwrap());

/// Gjør wikilenker og bilder om til HTML. Tekst i kode er egne noder, så den røres ikke.
fn transform<'a>(arena: &'a Arena<'a>, root: &'a AstNode<'a>, images: ImageResolver) {
    let nodes: Vec<&AstNode> = root.descendants().collect();
    for node in nodes {
        let value = node.data.borrow().value.clone();
        match value {
            NodeValue::Text(text) if text.contains("[[") => {
                let mut last = 0;
                for caps in WIKILINK.captures_iter(&text) {
                    let whole = caps.get(0).unwrap();
                    if whole.start() > last {
                        let before = text[last..whole.start()].to_string();
                        node.insert_before(arena.alloc(AstNode::from(NodeValue::Text(before.into()))));
                    }
                    let html = if caps[1].is_empty() { link_html(&caps[2]) } else { embed_html(&caps[2], images) };
                    node.insert_before(arena.alloc(AstNode::from(NodeValue::HtmlInline(html))));
                    last = whole.end();
                }
                if last == 0 {
                    continue;
                }
                if last < text.len() {
                    let after = text[last..].to_string();
                    node.insert_before(arena.alloc(AstNode::from(NodeValue::Text(after.into()))));
                }
                node.detach();
            }
            NodeValue::Image(link) if !HAS_SCHEME.is_match(&link.url) && !link.url.starts_with('#') => {
                if let Some(url) = images(&link.url, false) {
                    if let NodeValue::Image(link) = &mut node.data.borrow_mut().value {
                        link.url = url;
                    }
                }
            }
            _ => {}
        }
    }
}

/// `![[bilde.png|400]]` som i Obsidian, så CSS-en for bilder i `slides.css` treffer.
fn embed_html(inner: &str, images: ImageResolver) -> String {
    let (target, alias) = inner.split_once('|').map_or((inner, None), |(t, a)| (t, Some(a.trim())));
    let target = target.trim();
    if !is_image(target.split('#').next().unwrap_or(target)) {
        return format!(r#"<span class="internal-embed">{}</span>"#, escape_text(target));
    }
    let mut size = String::new();
    let mut alt = target.to_string();
    if let Some(alias) = alias {
        match SIZE.captures(alias) {
            Some(caps) => {
                size.push_str(&format!(r#" width="{}""#, &caps[1]));
                if let Some(height) = caps.get(2) {
                    size.push_str(&format!(r#" height="{}""#, height.as_str()));
                }
            }
            None => alt = alias.to_string(),
        }
    }
    match images(target, true) {
        Some(url) => format!(
            r#"<span class="internal-embed image-embed" src="{}"><img src="{}" alt="{}"{size}></span>"#,
            escape_attribute(target),
            escape_attribute(&url),
            escape_attribute(&alt),
        ),
        None => format!(
            r#"<span class="internal-embed image-embed is-unresolved" src="{0}">{0}</span>"#,
            escape_text(target)
        ),
    }
}

/// `[[Manus]]` og `[[Manus|tekst]]` som en lenke appen åpner selv.
fn link_html(inner: &str) -> String {
    let (target, text) = inner.split_once('|').map_or((inner, inner), |(t, a)| (t, a));
    format!(
        r##"<a class="internal-link" href="#" data-href="{}">{}</a>"##,
        escape_attribute(target.trim()),
        escape_text(text.trim())
    )
}

fn escape_text(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn escape_attribute(text: &str) -> String {
    escape_text(text).replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_images(_: &str, _: bool) -> Option<String> {
        None
    }

    fn images(target: &str, wiki: bool) -> Option<String> {
        Some(format!("asset://{}{target}", if wiki { "w/" } else { "m/" }))
    }

    #[test]
    fn frontmatter_og_kommentarer_vises_ikke() {
        let html = render_html("---\ntags: a\n---\n# Tittel\n\nTekst %% skjult %% synlig\n\n%%\nflere\nlinjer\n%%\n\n```\n%% kode %%\n```\n", &no_images);
        assert!(html.contains("<h1>Tittel</h1>"));
        assert!(!html.contains("tags"));
        assert!(html.contains("Tekst  synlig"));
        assert!(!html.contains("flere"));
        assert!(html.contains("%% kode %%"));
    }

    #[test]
    fn utheving_callout_og_wikilenker() {
        let html = render_html("==viktig==\n\n> [!warning|sjekk] Sjekk dette\n> Innhold **fet**\n\n[[Mappe/Notat|tekst]] `[[kode]]`", &no_images);
        assert!(html.contains("<mark>viktig</mark>"), "{html}");
        assert!(html.contains(r#"<div class="callout" data-callout="warning">"#), "{html}");
        assert!(html.contains(r#"<div class="callout-title">Sjekk dette</div>"#), "{html}");
        assert!(html.contains("<p>Innhold <strong>fet</strong></p>"), "{html}");
        assert!(html.contains(r##"<a class="internal-link" href="#" data-href="Mappe/Notat">tekst</a>"##), "{html}");
        assert!(html.contains("<code>[[kode]]</code>"), "{html}");
    }

    #[test]
    fn bilder_far_samme_dom_som_i_obsidian() {
        let html = render_html("![[kart.png|400]] ![[Notat]]\n\n![alt](bilder/x.png) ![nett](https://e.no/x.png)", &images);
        assert!(
            html.contains(r#"<span class="internal-embed image-embed" src="kart.png"><img src="asset://w/kart.png" alt="kart.png" width="400"></span>"#),
            "{html}"
        );
        assert!(html.contains(r#"<span class="internal-embed">Notat</span>"#), "{html}");
        assert!(html.contains(r#"<img src="asset://m/bilder/x.png" alt="alt" />"#), "{html}");
        assert!(html.contains(r#"src="https://e.no/x.png""#), "{html}");
    }

    #[test]
    fn presentasjonen_deles_pa_tematiske_skiller() {
        let presentation = "---\ntags: preken\n---\n\n# Del 1\n\n---\n\n<div class=\"bibel-parallell\">\n<p><sup>1</sup> a (Sal 1:1)</p>\n<p>b (Ps 1:1)</p>\n</div>\n\n---\n\n---\n\n![[kart.png]]\n";
        let slides = slides(presentation, &images);
        assert_eq!(slides.len(), 3, "{slides:?}");
        assert_eq!(slides[0], "<h1>Del 1</h1>\n");
        assert!(slides[1].starts_with("<div class=\"bibel-parallell\">\n<p><sup>1</sup> a (Sal 1:1)</p>"), "{}", slides[1]);
        assert!(slides[2].contains("asset://w/kart.png"), "{}", slides[2]);
    }

    #[test]
    fn linjeskift_i_sitater_beholdes() {
        assert_eq!(slides("Den som vil vinne.\n– Ukjent\n", &no_images), ["<p>Den som vil vinne.<br />\n– Ukjent</p>\n"]);
    }
}
