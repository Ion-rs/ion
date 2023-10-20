#[macro_use]
mod ion;
mod parser;

pub use self::ion::*;
pub use self::parser::*;
use std::collections::BTreeMap;

pub type Dictionary = BTreeMap<String, Value>;
pub type Row = Vec<Value>;
