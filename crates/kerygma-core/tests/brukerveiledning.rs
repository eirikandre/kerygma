//! Brukerveiledningen (`docs/Brukerveiledning.md`) vises i appen under Hjelp → Brukerveiledning.
//! Lenkene inne i den (`[Bilder](#4-bilder)`) må peke på en overskrift som finnes.

use kerygma_core::markdown::render_document;
use regex::Regex;

#[test]
fn alle_lenker_i_brukerveiledningen_peker_pa_en_overskrift() {
    let html = render_document(include_str!("../../../docs/Brukerveiledning.md"));
    let ids: Vec<String> =
        Regex::new(r#"id="([^"]+)""#).unwrap().captures_iter(&html).map(|caps| caps[1].to_string()).collect();
    assert!(ids.iter().any(|id| id == "4-bilder"), "{ids:?}");
    for caps in Regex::new(r##"<a href="#([^"]+)">"##).unwrap().captures_iter(&html) {
        assert!(ids.contains(&caps[1].to_string()), "lenken #{} peker ikke på noen overskrift ({ids:?})", &caps[1]);
    }
}
