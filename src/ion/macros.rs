#[macro_export]
macro_rules! ion {
    ($raw:expr) => {{
        $raw.parse::<$crate::Ion>()
            .expect("Failed parsing to 'Ion'")
    }};
}

#[macro_export]
macro_rules! ion_filtered {
    ($raw:expr, $accepted_sections:expr) => {
        $crate::Ion::from_str_filtered($raw, $accepted_sections)
            .expect("Failed parsing by 'from_str_filtered' to 'Ion'")
    };
}
