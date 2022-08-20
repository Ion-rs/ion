use std::collections::BTreeMap;

#[macro_use]
mod ion;
mod parser;

pub use self::ion::{FromIon, Ion, IonError, Section, Value};
pub use self::parser::{Parser, ParserError};

pub type Dictionary = BTreeMap<String, Value>;
pub type Row = Vec<Value>;
