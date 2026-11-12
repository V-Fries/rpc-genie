use syn::{Ident, LitStr};

pub fn ident_to_lit_str(ident: &Ident) -> LitStr {
    syn::LitStr::new(&ident.to_string(), ident.span())
}
