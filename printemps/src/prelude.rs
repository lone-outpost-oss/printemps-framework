//! The prelude used within this crate.

#![allow(unused_imports)]

pub use crate::app::AppHandler;
pub use crate::moonbit::{HostString, MoonMem};
pub use anyhow::anyhow;
pub use anyhow::Result;
pub use std::path::Path;
pub use std::sync::Arc;
pub use std::sync::Mutex;

pub type BoxBodyResponse =
    hyper::Response<http_body_util::combinators::BoxBody<bytes::Bytes, anyhow::Error>>;
