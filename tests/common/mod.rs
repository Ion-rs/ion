use std::fs::read_to_string;

pub fn read_file<T: AsRef<str>>(filename: T) -> String {
    read_to_string(filename.as_ref())
        .unwrap_or_else(|_| panic!("Failed reading of the file '{}'", filename.as_ref()))
}

pub fn read_ion<T: AsRef<str>>(filename: T) -> ion::Ion {
    ion!(read_file(filename))
}

pub fn read_err_ion<T: AsRef<str>>(filename: T) -> ion::IonError {
    read_file(filename).parse::<ion::Ion>().unwrap_err()
}
