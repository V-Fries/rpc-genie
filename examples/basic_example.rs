// To see the generated code, you can run `cargo expand --example basic_example`

#[rpc_genie::service]
pub mod service_a {
    // Define other services to include in the current service
    sub_services! {
        // Can have any number of sub services
        // If we use the same service module multiple times, the state is not shared (i.e. it is
        // duplicated)
        pub sub_service_1: super::service_b,
        pub sub_service_2: super::service_b,
    }

    pub struct Server {
        pub server_name: String,
    }

    #[remote_methods]
    impl Server {
        // pass &self for stateful functions (use mutexes and other solutions for mutability)
        fn server_name(&self) -> String {
            self.server_name.clone()
        }

        // Don't pass &self if you don't need the state
        fn add(a: u32, b: u32) -> u32 {
            a + b
        }

        fn add_4(a: u32) -> u32 {
            // Self::add() is designated as a remote method, but it can still be called locally:
            Self::add(a, 4)
        }

        fn access_sub_service_state(&self) -> u32 {
            self.sub_service_1.some_state
        }

        fn complicated_pattern_arg((a, (b, c)): (u32, (String, f32))) {
            println!("received: ({a}, ({b}, {c}))");
        }
    }

    pub struct Client {}
}

#[rpc_genie::service]
mod service_b {
    pub struct Server {
        pub some_state: u32,
    }

    pub struct Client {}
}

#[tokio::main]
async fn main() {
    use std::sync::Arc;

    let _server = service_a::Server {
        server_name: "".to_string(),
        // Here we see that the sub services state are added to the main service state
        sub_service_1: Arc::new(service_b::Server { some_state: 1 }),
        sub_service_2: Arc::new(service_b::Server { some_state: 2 }),
    };

    let _client = service_a::Client {
        // Here we see that the sub services state are added to the main service state
        sub_service_1: Arc::new(service_b::Client {}),
        sub_service_2: Arc::new(service_b::Client {}),
    };
}
