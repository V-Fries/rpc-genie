mod service_parser;
mod code_generation;

use syn::{Ident, Item, ItemMod, ItemStruct, PatType, Path, Receiver, Token};

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
    ident: Ident,
    receiver: Option<Receiver>,
    args: Vec<PatType>,
}

struct SubService {
    pub_keyword: Option<Token![pub]>,
    name: Ident,
    colon: Token![:],
    path: Path,
}
