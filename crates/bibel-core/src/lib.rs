//! Oppslag, søk og formatering av bibelvers i oversettelsene i `bibler/*.bin`.
//!
//! Kerygma (`kerygma-core::bible`) leser oversettelsene fra disk og går gjennom
//! [`app::run`], med de samme argumentene som kommandoen `bibel` i bible-repoet,
//! der denne craten kommer fra. Generatoren for `.bin`-filene og kommandoen ligger
//! fortsatt der; her er bare det appen trenger.

pub mod app;
pub mod reference;
pub mod render;
pub mod search;
pub mod translations;

pub use app::{run, Output};
pub use translations::Registry;
