use proc_macro_crate::FoundCrate;
use proc_macro2::{Ident, Span};

pub fn get_crate_name_of(name: &str, span: Span) -> Ident {
    let Ok(crate_name) = proc_macro_crate::crate_name(name) else {
        return Ident::new(name, span);
    };
    match crate_name {
        FoundCrate::Itself => Ident::new("crate", Span::call_site()),
        FoundCrate::Name(name) => Ident::new(&name, span),
    }
}
