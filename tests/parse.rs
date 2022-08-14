#[macro_use]
extern crate ion;

mod common;

use crate::common::{read_err_ion, read_file, read_ion};

mod deser_and_ser {
    use super::*;

    #[test]
    fn test_ion() {
        let ion = read_ion("tests/data/test.ion");
        let expected = read_file("tests/expected/test.ion");

        assert_eq!(expected, ion.to_string());
    }

    #[test]
    fn hotel_ion() {
        let ion = read_ion("tests/data/hotel.ion");
        let expected = read_file("tests/expected/hotel.ion");

        assert_eq!(expected, ion.to_string());
    }
}

#[test]
fn broken_array_and_eof() {
    let ion_err = read_err_ion("tests/data/broken_array_and_eof.ion");
    let expected =
        "ParserError(ParserError { section: \"CONTRACT\", desc: \"Cannot finish an array\" })";

    assert_eq!(expected, ion_err.to_string());
}

#[test]
fn broken_dictionary_and_eof() {
    let ion_err = read_err_ion("tests/data/broken_dictionary_and_eof.ion");
    let expected =
        "ParserError(ParserError { section: \"CONTRACT\", desc: \"Cannot finish a dictionary\" })";

    assert_eq!(expected, ion_err.to_string());
}
