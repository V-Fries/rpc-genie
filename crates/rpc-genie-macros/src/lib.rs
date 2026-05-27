mod service_parser;

use proc_macro::TokenStream;
use quote::quote;
use syn::{Ident, ImplItemFn, Item, ItemMod, ItemStruct, PatType, Path, Receiver, Token};

struct Service {
    pub module: ItemMod,
    pub server: ItemStruct,
    pub server_remote_methods: Vec<RemoteMethod>,
    pub client: ItemStruct,
    pub client_remote_methods: Vec<RemoteMethod>,
    pub sub_services: Vec<SubService>,
    pub rest: Vec<Item>,
}

// TODO remove allow(dead_code)
#[allow(dead_code)]
struct RemoteMethod {
    pub ident: Ident,
    pub receiver: Option<Receiver>,
    pub args: Vec<PatType>,
    pub method: ImplItemFn,
}

// TODO remove allow(dead_code)
#[allow(dead_code)]
struct SubService {
    pub pub_keyword: Option<Token![pub]>,
    pub name: Ident,
    pub colon: Token![:],
    pub path: Path,
}

#[proc_macro_attribute]
pub fn service(_attr: TokenStream, tokens: TokenStream) -> TokenStream {
    let _service = syn::parse_macro_input!(tokens as Service);

    todo!()
}
