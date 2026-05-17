# Rust Marpa Crate (dginev fork)

This is a long-term-maintained fork of [jrobsonchase/marpa](https://github.com/jrobsonchase/marpa),
with the abstract syntax forest (ASF) traversal work continued on the
`abstract_syntax_forests` branch. Used downstream by
[latexml-oxide](https://github.com/dginev/latexml-oxide) as its math
grammar engine.

## Crates

| Crate | Path | Role |
|---|---|---|
| `marpa` | [`marpa/`](marpa/) | Safe Rust bindings + grammar/recognizer/ASF API |
| `libmarpa-sys` | [`libmarpa-sys/`](libmarpa-sys/) | Low-level `bindgen`-generated FFI to the bundled libmarpa C source (8.6.2 tarball, statically linked) |

## Building

```sh
cargo build --workspace
cargo test --workspace
```

Requires a working C toolchain (`cc`, `make`, `configure`) to build
libmarpa from the bundled tarball, and Rust 1.85+ for edition 2024.

## License

Dual-licensed under Apache-2.0 OR MIT:

* [LICENSE-APACHE](LICENSE-APACHE)
* [LICENSE-MIT](LICENSE-MIT)

### Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.
