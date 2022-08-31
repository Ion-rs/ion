mod parser;

use crate::{Ion, IonError};

pub use self::parser::{Element, Parser, ParserError};

/// Deserialize an instance of `Ion` from a type returning a reference as `str`.
pub fn from_str<Text>(text: Text) -> Result<Ion, IonError>
where
    Text: AsRef<str>,
{
    text.as_ref().parse()
}
