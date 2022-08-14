#![feature(test)]

extern crate ion;
extern crate test;

use ion::{Ion, Parser};
use test::{black_box, Bencher};

// all these files have the same number of section and lines.
// they differ by the location of the section in the file only.
const CONTRACT_AND_DEF_HOTEL_ON_START: &str =
    include_str!("../tests/data/contract_and_def_hotel_on_start.ion");
const CONTRACT_ON_START_DEF_HOTEL_ON_END: &str =
    include_str!("../tests/data/contract_on_start_def_hotel_on_end.ion");
const CONTRACT_AND_DEF_HOTEL_ON_END: &str =
    include_str!("../tests/data/contract_and_def_hotel_on_end.ion");

mod parse {
    use super::*;
    use std::str::FromStr;

    #[bench]
    fn contract_and_def_hotel_on_start(bencher: &mut Bencher) {
        bencher.iter(|| {
            let result = Ion::from_str(CONTRACT_AND_DEF_HOTEL_ON_START);
            black_box(result.unwrap())
        })
    }

    #[bench]
    fn contract_and_def_hotel_on_start_and_tuned_parser(bencher: &mut Bencher) {
        bencher.iter(|| {
            let result = Parser::new(CONTRACT_AND_DEF_HOTEL_ON_START)
                .with_row_capacity(12)
                .with_array_capacity(4)
                .with_section_capacity(1024)
                .read();

            black_box(result.unwrap())
        })
    }

    #[bench]
    fn contract_and_def_hotel_on_start_and_no_prealloc(bencher: &mut Bencher) {
        bencher.iter(|| {
            let result = Parser::new(CONTRACT_AND_DEF_HOTEL_ON_START)
                .with_row_capacity(0)
                .with_array_capacity(0)
                .with_section_capacity(0)
                .read();

            black_box(result.unwrap())
        })
    }

    mod when_filtering {
        use super::*;

        const FILTERED_SECTIONS: &[&str] = &["CONTRACT", "DEF.HOTEL"];

        #[bench]
        fn contract_and_def_hotel_on_start(bencher: &mut Bencher) {
            bencher.iter(|| {
                let result = Ion::from_str_filtered(
                    CONTRACT_AND_DEF_HOTEL_ON_START,
                    FILTERED_SECTIONS.to_vec(),
                );
                black_box(result.unwrap())
            })
        }

        #[bench]
        fn contract_on_start_def_hotel_on_end(bencher: &mut Bencher) {
            bencher.iter(|| {
                let result = Ion::from_str_filtered(
                    CONTRACT_ON_START_DEF_HOTEL_ON_END,
                    FILTERED_SECTIONS.to_vec(),
                );
                black_box(result.unwrap())
            })
        }

        #[bench]
        fn contract_and_def_hotel_on_end(bencher: &mut Bencher) {
            bencher.iter(|| {
                let result = Ion::from_str_filtered(
                    CONTRACT_AND_DEF_HOTEL_ON_END,
                    FILTERED_SECTIONS.to_vec(),
                );
                black_box(result.unwrap())
            })
        }
    }
}
