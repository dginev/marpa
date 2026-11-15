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

/// Per-thread counters of the obstack chunk cache in `c/marpa_obs.c` (test-only, the
/// `chunk-cache-stats` feature). All zero on targets without the cache (Windows, wasm).
#[cfg(feature = "chunk-cache-stats")]
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ChunkCacheStats {
    /// Chunk requests served from the cache.
    pub hits: u64,
    /// Freed default-size chunks the cache kept.
    pub kept: u64,
    /// Freed default-size chunks freed because the cache was full.
    pub over_cap: u64,
    /// Chunk requests of a non-default (larger) size.
    pub oversize: u64,
    /// Chunks cached now.
    pub count: u64,
    /// Most chunks cached at once.
    pub peak: u64,
}

#[cfg(feature = "chunk-cache-stats")]
unsafe extern "C" {
    fn marpa_asf_obs_chunk_cache_stats(out: *mut ChunkCacheStats);
}

/// The calling thread's chunk-cache counters.
#[cfg(feature = "chunk-cache-stats")]
pub fn chunk_cache_stats() -> ChunkCacheStats {
    let mut out = ChunkCacheStats::default();
    // SAFETY: the C function writes one `struct marpa_asf_obs_chunk_cache_stats`, whose six
    // `unsigned long long` fields this `repr(C)` struct mirrors.
    unsafe { marpa_asf_obs_chunk_cache_stats(&mut out) };
    out
}

#[cfg(test)]
mod test;
