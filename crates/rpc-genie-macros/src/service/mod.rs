mod service_parser;

use proc_macro2::TokenStream;
use quote::{ToTokens, TokenStreamExt, quote};
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
    ident: Ident,
    receiver: Option<Receiver>,
    args: Vec<PatType>,
    method: ImplItemFn,
}

// TODO remove allow(dead_code)
#[allow(dead_code)]
struct SubService {
    pub_keyword: Option<Token![pub]>,
    name: Ident,
    colon: Token![:],
    path: Path,
}

impl ToTokens for Service {
    fn to_tokens(&self, dst: &mut TokenStream) {
        let ItemMod {
            attrs,
            vis,
            unsafety,
            mod_token,
            ident,
            content: _,
            semi,
        } = &self.module;

        let content = self.content();

        let tokens = quote! {
            #(#attrs)*
            #vis #unsafety #mod_token #ident {
                #content
            }#semi
        };

        dst.append_all(tokens);
    }
}

impl Service {
    fn content(&self) -> TokenStream {
        let server_and_clients_structs = self.server_and_clients_structs();

        let rest = &self.rest;

        quote! {
            #server_and_clients_structs

            #(#rest)*
        }
    }

    /// Creates the final code for the Server and Client
    fn server_and_clients_structs(&self) -> TokenStream {
        let create_final_struct = |ItemStruct {
                                       attrs,
                                       vis,
                                       struct_token,
                                       ident,
                                       generics,
                                       fields,
                                       semi_token,
                                   }: &ItemStruct| {
            let sub_services_fields = self.sub_services.iter().fold(
                TokenStream::new(),
                |acc,
                 SubService {
                     pub_keyword,
                     name,
                     colon,
                     path,
                 }| {
                    quote! {
                        #acc
                        #pub_keyword #name #colon #path :: #ident,
                    }
                },
            );

            quote! {
                #(#attrs)*
                #vis #struct_token #ident #generics {
                    #fields
                    #sub_services_fields
                }#semi_token
            }
        };

        let server_state = create_final_struct(&self.server);
        let client_state = create_final_struct(&self.client);

        quote! {
            #server_state
            #client_state
        }
    }
}
