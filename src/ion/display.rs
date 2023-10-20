use crate::{Ion, Section, Value};

use std::fmt;

impl fmt::Display for Ion {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        for (name, section) in &self.sections {
            f.write_fmt(format_args!("[{name}]\n"))?;
            section.fmt(f)?;
            f.write_str("\n")?;
        }
        Ok(())
    }
}

impl fmt::Display for Section {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        for (k, v) in &self.dictionary {
            if v.type_str() == "string" {
                f.write_fmt(format_args!("{k} = \"{v}\"\n"))?;
            } else {
                f.write_fmt(format_args!("{k} = {v}\n"))?;
            }
        }

        for row in &self.rows {
            for cell in row {
                fmt::Display::fmt(&format!("| {cell} "), f)?;
            }
            f.write_str("|\n")?;
        }
        Ok(())
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match *self {
            Value::String(ref v) => {
                for c in v.chars() {
                    match c {
                        '\\' => write!(f, "\\\\")?,
                        '\n' => write!(f, "\\n")?,
                        '\"' => write!(f, "\\\"")?,
                        c => write!(f, "{c}")?,
                    }
                }

                Ok(())
            }
            Value::Integer(ref v) => v.fmt(f),
            Value::Float(ref v) => v.fmt(f),
            Value::Boolean(ref v) => v.fmt(f),
            Value::Array(ref v) => {
                f.write_str("[ ")?;

                let mut first = true;
                for i in v {
                    if first {
                        first = false
                    } else {
                        f.write_str(", ")?
                    }
                    if i.is_string() {
                        f.write_str("\"")?;
                        i.fmt(f)?;
                        f.write_str("\"")?;
                    } else {
                        i.fmt(f)?;
                    }
                }
                f.write_str(" ]")
            }
            Value::Dictionary(ref d) => {
                f.write_str("{ ")?;

                let mut first = true;
                for (k, v) in d {
                    if first {
                        first = false
                    } else {
                        f.write_str(", ")?
                    }
                    k.fmt(f)?;
                    f.write_str(" = ")?;
                    if v.type_str() == "string" {
                        f.write_str("\"")?;
                        v.fmt(f)?;
                        f.write_str("\"")?;
                    } else {
                        v.fmt(f)?;
                    }
                }
                f.write_str(" }")
            }
        }
    }
}
