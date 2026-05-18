//! ASF instrumentation counters (opt-in via `MARPA_ASF_STATS=1`).
//!
//! Mirrors the counter set codex's senior-engineer review asked for
//! in `marpa/docs/ASF_PERFORMANCE_FINDINGS.md` (Instrumentation
//! Plan section). The counters answer three questions on real
//! downstream corpora:
//!
//! 1. **Singleton fast-path hit rate** — does
//!    `try_singleton_factoring` actually fire on the workload, or
//!    is the slow `collect_factorings` chain dominant? This
//!    validates the codex optimization.
//! 2. **Factoring shape** — how many factorings per symch, how
//!    often does `factoring_max` get hit (`omitted_factorings`)?
//!    Establishes whether the deferred "flatten factoring
//!    storage" item has evidence.
//! 3. **Forest shape** — glade / symch / factoring counts per
//!    parse, max source-nids per glade. Document the typical
//!    bocage shape so future optimization decisions have grounded
//!    estimates.
//!
//! Costs when `MARPA_ASF_STATS` is unset: a single
//! `once_cell::Lazy<bool>` load per increment site (≈1ns). The
//! per-increment branch is `if STATS_ENABLED.load() {...}`.
//!
//! Costs when enabled: a thread-local `Cell<u64>` bump per
//! counter call. Single-threaded by construction (libmarpa /
//! Recognizer / Bocage aren't `Send`), so no synchronization.

use std::cell::Cell;
use std::sync::OnceLock;

/// Cached env-var lookup. Reads `MARPA_ASF_STATS` once on first
/// access and stores the result; subsequent calls are an atomic
/// load. Lifted out of every increment site so the disabled path
/// is one branch + one load.
fn enabled() -> bool {
  static ENABLED: OnceLock<bool> = OnceLock::new();
  *ENABLED.get_or_init(|| std::env::var("MARPA_ASF_STATS").is_ok())
}

/// Public accessor: is the env var set?
pub fn asf_stats_enabled() -> bool { enabled() }

/// Per-thread, per-process ASF instrumentation accumulator.
/// Counters span every ASF traversal that runs on the current
/// thread; the downstream caller decides when to read and reset
/// (e.g. once per document, or at the end of a corpus run).
#[derive(Default, Debug, Clone)]
pub struct AsfStats {
  pub asf_news: u64,
  pub glades_visited: u64,
  pub user_callbacks: u64,
  pub cache_hits: u64,
  pub compute_symches_calls: u64,
  pub symches_built: u64,
  pub factorings_built: u64,
  pub omitted_factorings: u64,
  pub singleton_fast_path_hits: u64,
  pub general_factoring_fallback: u64,
  pub or_node_cache_hits: u64,
  pub or_node_cache_misses: u64,
  pub and_node_cache_hits: u64,
  pub and_node_cache_misses: u64,
  pub max_source_nids_per_glade: u32,
  pub max_factorings_per_symch: u32,
}

impl AsfStats {
  /// Reset every counter to zero. Use between distinct measurement
  /// windows.
  pub fn reset(&mut self) { *self = AsfStats::default(); }

  /// Render the counters in a stable single-line format, suitable
  /// for `eprintln!` or appending to a corpus log.
  pub fn as_log_line(&self) -> String {
    format!(
      "MARPA_ASF_STATS: asf_news={} glades={} callbacks={} cache_hits={} \
       compute_symches={} symches={} factorings={} omitted={} \
       singleton_hits={} general_fallback={} \
       or_cache_hit/miss={}/{} and_cache_hit/miss={}/{} \
       max_source_nids={} max_factorings={}",
      self.asf_news,
      self.glades_visited,
      self.user_callbacks,
      self.cache_hits,
      self.compute_symches_calls,
      self.symches_built,
      self.factorings_built,
      self.omitted_factorings,
      self.singleton_fast_path_hits,
      self.general_factoring_fallback,
      self.or_node_cache_hits,
      self.or_node_cache_misses,
      self.and_node_cache_hits,
      self.and_node_cache_misses,
      self.max_source_nids_per_glade,
      self.max_factorings_per_symch,
    )
  }
}

thread_local! {
  /// Lazy thread-local accumulator. Initialized to default on
  /// first access; only populated if `MARPA_ASF_STATS=1`.
  static STATS: Cell<Option<AsfStats>> = const { Cell::new(None) };
}

/// Run `f` against the thread-local stats accumulator. No-op when
/// `MARPA_ASF_STATS` is unset — `enabled()` short-circuits.
/// Re-entrant safe: we `take()` the cell, run `f`, put it back.
#[inline]
pub(crate) fn with_stats<F: FnOnce(&mut AsfStats)>(f: F) {
  if !enabled() {
    return;
  }
  STATS.with(|slot| {
    let mut current = slot.take().unwrap_or_default();
    f(&mut current);
    slot.set(Some(current));
  });
}

/// Read a snapshot of the current thread-local stats (`None` when
/// `MARPA_ASF_STATS` is unset and no work has run on this thread).
/// Public API for downstream callers — used from
/// `latexml-oxide/latexml_math_parser` to emit per-document reports.
#[allow(dead_code)]
pub fn snapshot() -> Option<AsfStats> {
  if !enabled() {
    return None;
  }
  STATS.with(|slot| {
    let value = slot.take();
    let clone = value.clone();
    slot.set(value);
    clone
  })
}

/// Reset the thread-local stats to zero. Useful for taking
/// per-window measurements (per-document, per-formula, etc.).
#[allow(dead_code)]
pub fn reset() {
  STATS.with(|slot| slot.set(None));
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn disabled_with_stats_is_noop() {
    // Without the env var, `with_stats` must not allocate or
    // touch the thread-local. We can't directly assert "didn't
    // call the closure" without observable side effects, but we
    // can confirm `snapshot()` returns None.
    reset();
    with_stats(|s| s.glades_visited += 1);
    // If MARPA_ASF_STATS isn't set in the test runner, snapshot
    // should be None. If it IS set (rare), this test trivially
    // passes by inversion.
    if !asf_stats_enabled() {
      assert!(snapshot().is_none());
    }
  }

  #[test]
  fn as_log_line_contains_key_counters() {
    let mut s = AsfStats::default();
    s.glades_visited = 42;
    s.singleton_fast_path_hits = 17;
    let line = s.as_log_line();
    assert!(line.contains("glades=42"), "log: {line}");
    assert!(line.contains("singleton_hits=17"), "log: {line}");
  }

  #[test]
  fn reset_zeros_all_counters() {
    let mut s = AsfStats::default();
    s.glades_visited = 100;
    s.symches_built = 50;
    s.reset();
    assert_eq!(s.glades_visited, 0);
    assert_eq!(s.symches_built, 0);
  }
}
