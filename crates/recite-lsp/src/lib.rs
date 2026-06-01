//! Language server scaffolding for Recite source files.

mod capabilities;
mod diagnostics;
mod documents;
mod paths;
mod position;
mod server;
mod summary;
mod workspace;

pub use server::{ServerError, run_stdio};

#[cfg(test)]
mod tests;
