# ion_rs

Parser for `*.ion` files, e.g.:

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
