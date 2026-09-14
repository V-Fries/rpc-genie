use proc_macro2::TokenStream;
use quote::quote;
use syn::ReturnType;

use crate::service::{RemoteMethod, Service, SubService};

impl Service {
    pub fn stubs(&self) -> TokenStream {
        let client_stub = self.stub(quote!(ClientStub), &self.client_remote_methods);
        let server_stub = self.stub(quote!(ServerStub), &self.server_remote_methods);

        quote! {
            #server_stub
            #client_stub
        }
    }

    fn stub(
        &self,
        stub_struct_name: TokenStream,
        associated_remote_methods: &[RemoteMethod],
    ) -> TokenStream {
        let struct_fields = self.stub_struct_fields(&stub_struct_name);
        let remote_methods_callers =
            remote_methods_callers(&stub_struct_name, associated_remote_methods);
        let impl_stub_trait = self.impl_stub_trait(&stub_struct_name);
        let impl_subscribable_stub_trait = impl_subscribable_stub_trait(&stub_struct_name);

        quote! {
            #[derive(Clone)]
            pub struct #stub_struct_name<RequestSender> {
                __rpc_genie_request_sender__: RequestSender,
                #struct_fields
            }

            #impl_stub_trait
            #impl_subscribable_stub_trait

            #remote_methods_callers
        }
    }

    fn stub_struct_fields(&self, stub_struct_name: &TokenStream) -> TokenStream {
        self.sub_services.iter().fold(
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
                    #pub_keyword #name #colon #path::#stub_struct_name<RequestSender>,
                }
            },
        )
    }

    fn impl_stub_trait(&self, stub_struct_name: &TokenStream) -> TokenStream {
        let field_constructor = self.sub_services.iter().map(
            |SubService {
                 pub_keyword: _,
                 name,
                 colon,
                 path,
             }| {
                quote! {
                    #name #colon #path::#stub_struct_name::<RequestSender>::new(request_sender.clone())
                }
            },
        );

        quote! {
            impl<RequestSender> rpc_genie::Stub<RequestSender> for #stub_struct_name<RequestSender>
            where
                RequestSender: rpc_genie::send_request::SendRequest,
            {
                fn new(request_sender: RequestSender) -> Self {
                    Self {
                        #(#field_constructor,)*
                        __rpc_genie_request_sender__: request_sender,
                    }
                }
            }
        }
    }
}

fn remote_methods_callers(
    stub_struct_name: &TokenStream,
    associated_remote_methods: &[RemoteMethod],
) -> TokenStream {
    let associated_remote_methods = associated_remote_methods.iter().map(remote_method_caller);

    quote! {
        impl<RequestSender> #stub_struct_name<RequestSender>
        where
            RequestSender: rpc_genie::send_request::SendRequest,
        {
            #(#associated_remote_methods)*
        }
    }
}

fn remote_method_caller(
    RemoteMethod {
        vis,
        ident,
        receiver: _,
        args,
        output,
    }: &RemoteMethod,
) -> TokenStream {
    let output = match output {
        ReturnType::Default => quote!(-> rpc_genie::SingleRequestSender<()>),
        ReturnType::Type(arrow, type_ast) => {
            quote!(#arrow rpc_genie::SingleRequestSender<#type_ast>)
        }
    };

    // TODO remove this once we finish generating the actual fn content (maybe next PR?)
    let remove_unused_var_warning = args.iter().map(|arg| {
        let pat = &arg.pat;
        quote!(let _ = #pat;)
    });

    quote! {
        #vis fn #ident(&self, #(#args,)*) #output {
            // TODO remove this once we finish generating the actual fn content (maybe next PR?)
            #(#remove_unused_var_warning)*

            todo!("Send the request to server and receive the response")
        }
    }
}

fn impl_subscribable_stub_trait(stub_struct_name: &TokenStream) -> TokenStream {
    quote! {
        impl<RequestSender> rpc_genie::SubscribableStub for #stub_struct_name<RequestSender>
        where
            RequestSender: rpc_genie::SubscribableStub + Sync,
        {
            async fn add_registered_topic(
                &self,
                topic_id: rpc_genie::topic::TopicId,
                stub_died_notification_sender: rpc_genie::topic::StubDiedNotificationSender,
            ) -> bool {
                rpc_genie::SubscribableStub::add_registered_topic(
                    &self.__rpc_genie_request_sender__,
                    topic_id,
                    stub_died_notification_sender,
                ).await
            }

            fn remove_registered_topic(
                &self,
                topic_id: rpc_genie::topic::TopicId
            ) {
                rpc_genie::SubscribableStub::remove_registered_topic(
                    &self.__rpc_genie_request_sender__,
                    topic_id,
                )
            }

            fn stream_id(&self) -> Option<rpc_genie::stream_handler::StreamId> {
                rpc_genie::SubscribableStub::stream_id(&self.__rpc_genie_request_sender__)
            }
        }
    }
}
