use std::collections::BTreeMap;

#[macro_use]
mod ion;
mod parser;
mod writer;
pub use parser::{Parser, ParserError};
pub use writer::Writer;

pub type Dictionary = BTreeMap<String, Value>;
pub use self::ion::{FromIon, Ion, IonError, Section, Value};
pub type Row = Vec<Value>;
