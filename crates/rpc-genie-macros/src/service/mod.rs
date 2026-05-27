mod service_parser;

use syn::{Ident, ImplItemFn, Item, ItemMod, ItemStruct, PatType, Path, Receiver, Token};

// TODO remove allow(dead_code)
#[allow(dead_code)]
pub struct Service {
    module: ItemMod,
    server: ItemStruct,
    server_remote_methods: Vec<RemoteMethod>,
    client: ItemStruct,
    client_remote_methods: Vec<RemoteMethod>,
    sub_services: Vec<SubService>,
    rest: Vec<Item>,
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
