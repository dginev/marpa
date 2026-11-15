//! The obstack chunk cache of `c/marpa_obs.c`, driven through libmarpa's internal
//! `marpa__obs_begin`/`marpa__obs_free` (global symbols of the static library, not in
//! `marpa.h`).
// Links the static libmarpa that defines them.
extern crate libmarpa_asf_sys as libmarpa_sys;

use std::ffi::c_void;

#[cfg_attr(not(feature = "chunk-cache-stats"), allow(dead_code))]
unsafe extern "C" {
    fn marpa__obs_begin(size: usize) -> *mut c_void;
    fn marpa__obs_free(h: *mut c_void);
}

/// The stats build behaves as production: a chunk request never creates the thread's
/// cache, only a release of a default-size chunk does, so an oversize obstack begun and
/// freed in a fresh thread leaves no cache and no counts.
#[cfg(feature = "chunk-cache-stats")]
#[test]
fn oversize_alloc_creates_no_cache() {
    let stats = std::thread::spawn(|| {
        let h = unsafe { marpa__obs_begin(1 << 16) };
        assert!(!h.is_null());
        let during = libmarpa_sys::chunk_cache_stats();
        unsafe { marpa__obs_free(h) };
        (during, libmarpa_sys::chunk_cache_stats())
    })
    .join()
    .expect("thread");
    assert_eq!(
        stats,
        (libmarpa_sys::ChunkCacheStats::default(), libmarpa_sys::ChunkCacheStats::default())
    );
}
