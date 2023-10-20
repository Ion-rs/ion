# ion &emsp; [![crates-badge]][crates-link] [![docs-badge]][docs-link]

[crates-badge]: https://img.shields.io/crates/v/ion.svg
[crates-link]: https://crates.io/crates/ion
[docs-badge]: https://img.shields.io/badge/docs.rs-latest-informational
[docs-link]: https://docs.rs/ion

Parser for `*.ion` files:

``` ini
[CONTRACT]
id = "HOTEL001"
name = "Hotel001"
currency = "EUR"
active = true
markets = ["DE", "PL"]

[DEF.MEAL]
| code | description |
|------|-------------|
| RO   | Room Only   |

[DEF.ROOM]
| code | description |      occ       |
|------|-------------|----------------|
| SGL  | Single      | P1:2 A1:1 C0:1 |
| DBL  | Double      | P2:3 A2:2 C0:1 |
```

## License

Licensed under the MIT license.
