//! Language server scaffolding for Recite source files.

mod diagnostics;
mod documents;
mod position;
mod server;

pub use server::{ServerError, run_stdio};

#[cfg(test)]
mod tests;
