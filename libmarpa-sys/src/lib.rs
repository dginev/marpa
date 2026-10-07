//! Generated bindings to [libmarpa]
//!
//! [libmarpa]: https://jeffreykegler.github.io/Marpa-web-site/libmarpa.html

#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]

// bindgen's output trips lints that newer toolchains raise (unsafe fns without `# Safety` docs,
// pointer offsets cast from usize, transmutes in bitfield accessors, libc declarations whose
// integer widths differ from std's); generated code is not this crate's to rewrite.
#[allow(
    unknown_lints,
    clippy::missing_safety_doc,
    clippy::ptr_offset_with_cast,
    unnecessary_transmutes,
    suspicious_runtime_symbol_definitions
)]
mod raw {
    include!(concat!(env!("OUT_DIR"), "/raw.rs"));
}
pub use crate::raw::*;

#[cfg(test)]
mod test;
