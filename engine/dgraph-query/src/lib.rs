//! dgraph-query: Query parsing and execution
//!
//! DQL parser and executor. Clean and simple.

#![forbid(unsafe_code)]
#![deny(missing_docs, clippy::all, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

mod parser;
mod ast;
mod executor;

pub use parser::{Parser, ParseError};
pub use ast::{Query, Mutation, Operation, Filter, Value};
pub use executor::Executor;
