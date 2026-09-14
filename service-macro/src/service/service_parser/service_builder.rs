use std::collections::HashSet;

use syn::{Ident, Item, ItemMod, ItemStruct};

use crate::service::{RemoteMethod, SubService};

use super::{Service, combine_errors};

#[derive(Default)]
pub struct ServiceBuilder {
    pub server: Option<ItemStruct>,
    pub server_remote_methods: Vec<RemoteMethod>,
    pub client: Option<ItemStruct>,
    pub client_remote_methods: Vec<RemoteMethod>,
    pub sub_services: Vec<SubService>,
    pub rest: Vec<Item>,
}

impl ServiceBuilder {
    pub fn build(self, module: ItemMod, mut errors: Option<syn::Error>) -> syn::Result<Service> {
        let sub_services_names = get_set_of_sub_services_names(&self.sub_services, &mut errors);

        self.check_that_server_and_client_field_names_are_not_also_sub_service_name(
            sub_services_names,
            &mut errors,
        );

        self.check_server_struct_is_present(&module, &mut errors);
        self.check_client_struct_is_present(&module, &mut errors);

        if let Some(errors) = errors {
            return Err(errors);
        }

        Ok(Service {
            rest: self.rest,
            server: self
                .server
                .expect("presence of server was already verified"),
            server_remote_methods: self.server_remote_methods,
            client: self
                .client
                .expect("presence of client was already verified"),
            client_remote_methods: self.client_remote_methods,
            sub_services: self.sub_services,

            module,
        })
    }

    fn check_that_server_and_client_field_names_are_not_also_sub_service_name(
        &self,
        sub_services_names: HashSet<String>,
        errors: &mut Option<syn::Error>,
    ) {
        let server_field_names_that_are_also_sub_services_names =
            get_fields_name_that_are_also_sub_services_names(&self.server, &sub_services_names);
        let client_field_names_that_are_also_sub_services_names =
            get_fields_name_that_are_also_sub_services_names(&self.client, &sub_services_names);

        server_field_names_that_are_also_sub_services_names
            .chain(client_field_names_that_are_also_sub_services_names)
            .for_each(|name| {
                let error = syn::Error::new_spanned(
                    name,
                    "field name can not be the same as a sub-service name",
                );
                combine_errors(errors, error);
            });
    }

    fn check_server_struct_is_present(&self, module: &ItemMod, errors: &mut Option<syn::Error>) {
        if self.server.as_ref().is_none() {
            combine_errors(
                errors,
                syn::Error::new_spanned(
                    module,
                    "rpc_genie::service macro requires a Server struct to be present in the module",
                ),
            );
        }
    }

    fn check_client_struct_is_present(&self, module: &ItemMod, errors: &mut Option<syn::Error>) {
        if self.client.as_ref().is_none() {
            combine_errors(
                errors,
                syn::Error::new_spanned(
                    module,
                    "rpc_genie::service macro requires a Client struct to be present in the module",
                ),
            );
        }
    }
}

/// Pushes errors for duplicate sub-service names
fn get_set_of_sub_services_names(
    sub_services: &[SubService],
    errors: &mut Option<syn::Error>,
) -> HashSet<String> {
    sub_services
        .iter()
        .fold(HashSet::new(), |mut sub_services_names, sub_service| {
            if sub_services_names.insert(sub_service.name.to_string()) {
                return sub_services_names;
            }

            let new_error = syn::Error::new_spanned(
                sub_service.name.clone(),
                "name is already taken by another sub-service",
            );
            combine_errors(errors, new_error);

            sub_services_names
        })
}

fn get_fields_name_that_are_also_sub_services_names<'a>(
    struct_item: &'a Option<ItemStruct>,
    sub_services_names: &HashSet<String>,
) -> impl Iterator<Item = &'a Ident> {
    struct_item
        .iter()
        .flat_map(|s| s.fields.iter())
        .filter_map(|field| {
            field
                .ident
                .as_ref()
                .filter(|ident| sub_services_names.contains(&ident.to_string()))
        })
}
