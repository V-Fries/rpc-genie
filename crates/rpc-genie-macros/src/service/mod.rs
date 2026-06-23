mod code_generation;
mod service_parser;

use syn::{
    Ident, Item, ItemMod, ItemStruct, PatType, Path, Receiver, ReturnType, Token, Visibility,
};

pub struct Service {
    module: ItemMod,
    server: ItemStruct,
    server_remote_methods: Vec<RemoteMethod>,
    client: ItemStruct,
    client_remote_methods: Vec<RemoteMethod>,
    sub_services: Vec<SubService>,
    rest: Vec<Item>,
}

struct RemoteMethod {
    vis: Visibility,
    ident: Ident,
    receiver: Option<Receiver>,
    args: Vec<PatType>,
    output: ReturnType,
}

struct SubService {
    pub_keyword: Option<Token![pub]>,
    name: Ident,
    colon: Token![:],
    path: Path,
}
