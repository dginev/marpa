//! The obstack chunk cache of `c/marpa_obs.c`, driven through libmarpa's internal
//! `marpa__obs_begin`/`marpa__obs_free` (global symbols of the static library, not in
//! `marpa.h`).
// Links the static libmarpa that defines them.
extern crate libmarpa_asf_sys as libmarpa_sys;

use std::ffi::c_void;

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

/// Reads an obstack's memory after `marpa__obs_free`: the chunk sits in the thread's cache,
/// still allocated, so only its poisoning (ASan) or no-access marking (valgrind) reports the
/// use-after-free. Run as a child by the test below, or by hand under valgrind.
#[test]
#[ignore = "reads freed memory on purpose"]
fn use_after_free_probe() {
    let h = unsafe { marpa__obs_begin(0) }.cast::<u8>();
    unsafe { marpa__obs_free(h.cast()) };
    let stale = unsafe { std::ptr::read_volatile(h) };
    println!("read a freed obstack: {stale}");
}

/// Under ASan, the read of a cached chunk is reported, as upstream reports it of a freed one.
#[cfg(all(marpa_asan, not(any(windows, target_family = "wasm"))))]
#[test]
fn asan_reports_a_read_of_a_cached_chunk() {
    let out = std::process::Command::new(std::env::current_exe().expect("test binary"))
        .args(["--ignored", "--exact", "use_after_free_probe", "--test-threads=1"])
        .output()
        .expect("probe");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success() && stderr.contains("use-after-poison"), "{stderr}");
}
