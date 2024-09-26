use serde::{Deserialize, Serialize};

pub mod http;
#[allow(unused_imports)]
pub use http::HttpRouterClient;

pub mod in_memory;
#[allow(unused_imports)]
pub use in_memory::InMemoryRouterClient;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub enum RouterClientAdapter {
    #[serde(rename = "in_memory")]
    #[default]
    InMemory,
    #[serde(rename = "http")]
    Http,
}
