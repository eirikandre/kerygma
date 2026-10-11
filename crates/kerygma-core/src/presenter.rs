//! Talervisningen: slidene vises på den andre skjermen, og manuset i lesevisning
//! på skjermen til taleren, med det som vises markert.
//!
//! Slidene lages fra manuset med de samme reglene som «Lag presentasjon». Hvor
//! hver slide står i manuset, merkes med usynlige tegn i teksten før den gjøres om
//! til HTML: [`START`] + nummeret der sliden starter og [`END`] + nummeret der den
//! slutter. Grensesnittet finner tegnene, fjerner dem og markerer teksten mellom.
//! En slide kan ha flere par, når oversettelsene står på hver sin linje.

use std::ops::Range;

use crate::extract::{extract_slides, SlideOptions};
use crate::markdown::{self, ImageResolver};

/// Tegnet der slide `i` starter er `START + i` (privat bruk, plan 15).
pub const START: u32 = 0xF0000;
/// Tegnet der slide `i` slutter er `END + i` (privat bruk, plan 16).
pub const END: u32 = 0x10_0000;
/// Så mange slides kan merkes; resten vises, men kan bare nås med tastene.
const MAX_MARKED: usize = 0xFFFE;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresenterView {
    /// HTML per slide, som i «Vis presentasjon».
    pub slides: Vec<String>,
    /// Manuset som HTML til lesevisningen, med merkene.
    pub manuscript: String,
}

/// Slidene og manuset med merker. Gir et utdrag av manuset flere slides (sjelden),
/// merkes bare den første.
pub fn presenter_view(manuscript: &str, options: &SlideOptions, images: ImageResolver) -> PresenterView {
    let mut slides = Vec::new();
    let mut marks: Vec<(Range<usize>, usize)> = Vec::new();
    for slide in extract_slides(manuscript, options) {
        let html = markdown::slides(&slide.markdown, images);
        if html.is_empty() {
            continue;
        }
        if slides.len() < MAX_MARKED {
            marks.extend(slide.source.into_iter().filter(|range| !range.is_empty()).map(|range| (range, slides.len())));
        }
        slides.extend(html);
    }
    PresenterView { slides, manuscript: markdown::render_html(&with_marks(manuscript, &marks), images) }
}

/// Setter inn merkene. Et merke for slutt flyttes forbi mellomrom etter teksten, så
/// det ikke står inntil `_` eller `*` (det kunne hindret kursiv i å virke).
fn with_marks(manuscript: &str, marks: &[(Range<usize>, usize)]) -> String {
    let mut points: Vec<(usize, bool, usize)> = Vec::new();
    for (range, index) in marks {
        let mut end = range.end;
        while manuscript[end.min(manuscript.len())..].starts_with([' ', '\t']) {
            end += 1;
        }
        if manuscript.is_char_boundary(range.start) && manuscript.is_char_boundary(end) {
            points.push((range.start, true, *index));
            points.push((end, false, *index));
        }
    }
    // Ved samme posisjon slutter den forrige sliden før den neste starter.
    points.sort_by_key(|&(at, start, index)| (at, start, index));
    let mut out = String::with_capacity(manuscript.len() + points.len() * 4);
    let mut copied = 0;
    for (at, start, index) in points {
        out.push_str(&manuscript[copied..at]);
        let base = if start { START } else { END };
        out.extend(char::from_u32(base + index as u32));
        copied = at;
    }
    out.push_str(&manuscript[copied..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const NO_IMAGES: ImageResolver = &|_, _| None;

    fn start(i: u32) -> char {
        char::from_u32(START + i).unwrap()
    }

    fn end(i: u32) -> char {
        char::from_u32(END + i).unwrap()
    }

    /// Manuset med merkene, før det gjøres om til HTML.
    fn marked(manuscript: &str, options: &SlideOptions) -> String {
        let mut marks = Vec::new();
        for (i, slide) in extract_slides(manuscript, options).into_iter().enumerate() {
            marks.extend(slide.source.into_iter().map(|range| (range, i)));
        }
        with_marks(manuscript, &marks)
    }

    #[test]
    fn hver_slide_merkes_der_den_star_i_manuset() {
        // Et vers er teksten fra forrige referanse, som i pluginen.
        let manuscript = "---\ntags: a\n---\n# Påske\n\nInnledning. Elsket verden (Joh 3,16) og mer. Herren er min hyrde (Sal 23,1)\n\n> Et sitat\n> over to linjer.\n\n- ![[bilde.png]]\n";
        assert_eq!(
            marked(manuscript, &SlideOptions::default()),
            format!(
                "---\ntags: a\n---\n# {}Påske{}\n\n{}Innledning. Elsket verden (Joh 3,16) {}{}og mer. Herren er min hyrde (Sal 23,1){}\n\n> {}Et sitat\n> over to linjer.{}\n\n- {}![[bilde.png]]{}\n",
                start(0), end(0), start(1), end(1), start(2), end(2), start(3), end(3), start(4), end(4)
            )
        );
    }

    #[test]
    fn delte_vers_og_oversettelser_pa_egne_linjer_merkes_del_for_del() {
        let options = SlideOptions { max_chars: Some(30), ..Default::default() };
        let manuscript = "> Herren er min hyrde. Jeg mangler ingen ting. (Sal 23,1)\n";
        assert_eq!(
            marked(manuscript, &options),
            format!("> {}Herren er min hyrde. {}{}Jeg mangler ingen ting. (Sal 23,1){}\n", start(0), end(0), start(1), end(1))
        );

        // Samme vers i to oversettelser blir kolonner: begge linjene hører til sliden.
        let manuscript = "> Gud er kjærlighet. (1 Joh 4,8)\n> God is love. (1 John 4:8)\n";
        assert_eq!(
            marked(manuscript, &SlideOptions::default()),
            format!(
                "> {}Gud er kjærlighet. (1 Joh 4,8){}\n> {}God is love. (1 John 4:8){}\n",
                start(0), end(0), start(0), end(0)
            )
        );
    }

    #[test]
    fn manuset_blir_html_uten_a_odelegge_formateringen() {
        let view = presenter_view("# Tittel\n\n**Herren** er _god_ (Sal 23,1)\n", &SlideOptions::default(), NO_IMAGES);
        assert_eq!(view.slides.len(), 2);
        assert_eq!(
            view.manuscript,
            format!("<h1>{}Tittel{}</h1>\n<p>{}<strong>Herren</strong> er <em>god</em> (Sal 23,1){}</p>\n", start(0), end(0), start(1), end(1))
        );
    }
}
