# Rust Marpa Crate (dginev fork)

Safe Rust bindings for the [libmarpa](https://jeffreykegler.github.io/Marpa-web-site/libmarpa.html)
Earley parser, currently being extended with abstract-syntax-forest
(ASF) traversal support on the `abstract_syntax_forests` branch.

This is a long-term-maintained fork of
[jrobsonchase/marpa](https://github.com/jrobsonchase/marpa). Used by
[latexml-oxide](https://github.com/dginev/latexml-oxide) as its math
grammar engine.

The bindings are a thin layer over the C library (see the parent
workspace for the FFI sys crate); a usable frontend that constructs
grammars from a high-level description is one of the long-term goals.
