#[macro_use]
extern crate ion;

mod common;

use crate::common::{read_err_ion, read_file, read_ion};

mod de_ser_ion {
    use super::*;
    use test_case::test_case;

    #[test_case("tests/de_ser/test1"; "test 1")]
    #[test_case("tests/de_ser/test2"; "test 2")]
    fn checks_deser_and_display(test_dir: &str) {
        let ion = read_ion(format!("{}/actual.ion", test_dir));
        let expected = read_file(format!("{}/expected.ion", test_dir));

        assert_eq!(expected, ion.to_string());
    }

    mod err {
        use super::*;

        #[test]
        fn broken_array_and_eof() {
            let ion_err = read_err_ion("tests/data/broken_array_and_eof.ion");
            let expected = "ParserError(ParserError { section: \"CONTRACT\", desc: \"Cannot finish an array\" })";

            assert_eq!(expected, ion_err.to_string());
        }

        #[test]
        fn broken_dictionary_and_eof() {
            let ion_err = read_err_ion("tests/data/broken_dictionary_and_eof.ion");
            let expected = "ParserError(ParserError { section: \"CONTRACT\", desc: \"Cannot finish a dictionary\" })";

            assert_eq!(expected, ion_err.to_string());
        }
    }
}

#[cfg(feature = "serde-json")]
mod de_ser_json {
    use super::*;
    use test_case::test_case;

    #[test_case("tests/de_ser/test1"; "test 1")]
    #[test_case("tests/de_ser/test2"; "test 2")]
    fn checks_serialize_to_json(test_dir: &str) {
        let ion = read_ion(format!("{}/actual.ion", test_dir));
        let actual = serde_json::to_string_pretty(&ion).unwrap();
        let expected = read_file(format!("{}/expected.json", test_dir));

        pretty_assertions::assert_eq!(expected, actual);
    }
}
