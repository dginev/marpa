# ASF Performance — Current State

Status: **landed**. Last update 2026-05-18.

Hybrid routing is default; allocation-trim and large-bocage Tree-iter
fallback are merged via PR #4 (commit `0bf24111`, master). Downstream
latexml-oxide tracks marpa master.

## What's in master today

1. **Hybrid `Parser::parse_hybrid_with_and_node_limit(...)`**.
   Reads tokens once, builds one bocage, branches on
   `Bocage::ambiguity_metric()`:
   - `metric == 1` → ordinary `Tree` iterator path
     (`HybridParseResult::Unambiguous(Tree)`).
   - `metric >= 2`, bocage and-node count ≤ `max_and_nodes` → ASF
     traversal (`HybridParseResult::Ambiguous(PT, PS)`).
   - `metric >= 2`, bocage exceeds `max_and_nodes` → fallback
     `Tree` iterator (`HybridParseResult::AmbiguousTree(Tree, BocageStats)`).
   The convenience entry point `parse_hybrid(...)` calls the
   `max_and_nodes = None` variant.

2. **`Order::or_node_and_node_count_opt(usize)`** — new thin API
   for cheap bocage size inspection without holding the whole
   and-node list.

3. **ASF allocation trims** (cumulative, all in `marpa/src/asf.rs`):
   - `obtain_singleton_nidset_id(i32)` — flat `HashMap<i32, usize>`
     fast path for the dominant `vec![cause_nid]` shape.
   - Avoid cloning singleton source nidsets in `compute_symches`.
   - Skip cache-resident children when collecting recursion
     targets in `traverse_glade_recursive`.
   - Bocage metadata cache (OR-node IRL/XRL/LHS, AND-node
     cause/predecessor/symbol) on `Vec<Option<_>>`.

4. **`MARPA_ASF_STATS=1`** instrumentation — opt-in counters
   (asf_news, glades_visited, singleton_fast_path_hits,
   general_factoring_fallback, factorings_built, or/and cache
   hit/miss, max source nids per glade, max factorings per
   symch). No overhead when unset.

## Validation

### Article-2025.tex (579 formulae, 87.3 % raw-unambiguous, single-thread bench)

| Mode | Wall (3-run avg) | vs LEGACY |
|---|---:|---:|
| **HYBRID default** | **12.45 s** | **1.01×** |
| `LATEXML_MARPA_LEGACY=1` | 12.32 s | 1.00× |
| `LATEXML_MARPA_ASF_ONLY=1` | 16.80 s | 1.36× |

### latexml-oxide 100-paper math-bound sample (quiet host, marpa `0bf24111`)

100 papers selected by top `phase_math_parse_us` in wp4 telemetry.
Release+native+cortex, 8 workers, 180 s timeout, 8 GB ulimit.

| Mode | OK / 100 | OOM aborts | Wall (n=98) | Δ vs LEGACY |
|---|---:|---:|---:|---:|
| LEGACY | 98 | 0 | 2227.1 s | — |
| HYBRID (cap = 500) | 98 | **0** | 2238.6 s | **+0.5 %** |

HYBRID at parity with LEGACY (median per-paper +0.0 %, mean +1.0 %,
76 of 98 papers within ±5 %), zero OOM aborts. Without the cap,
HYBRID OOM-aborted 19 papers on this fixture; the
500-and-node cap routes those through the Tree-iter fallback.

Cap rationale: 500 is what downstream consumers (pragmatics
selection + XMath builders in latexml-oxide) can usefully handle.
Bigger bocages are treated as a pipeline-flaw signal — candidates
for grammar tightening / earlier action-time pruning, not a
target for raising the cap.

### `MARPA_ASF_STATS` shape (Article-2025, ASF_ONLY mode)

```
glades_visited=4_999_219
singleton_fast_path_hits=2_974_789   (99.98 % of factorings)
general_factoring_fallback=564       (0.02 %)
omitted_factorings=0                 (FACTORING_MAX=42 never hit)
max_factorings_per_symch=4
or_node_cache_hit/miss=5_950_020 / 2_975_353  (67 % hit)
and_node_cache_hit/miss=4_049_248 / 5_002_490 (45 % hit)
```

In HYBRID mode only the 12.7 % raw-ambiguous fraction enters ASF,
so absolute counters drop ~13×; the singleton-vs-general ratio
and `general_factoring_fallback=564` are paper-invariant — every
non-singleton factoring comes from raw-ambiguous formulae.

## Tested principles

- **Hybrid routing is the main win**, not micro-optimization.
  Allocation tweaks below moved the needle by ≤6 % cumulative on
  Article-2025; hybrid routing alone moved 17.0 s → 12.4 s.
- **The residual ASF_ONLY → LEGACY gap is structural.** ASF
  builds a Rust-side glade/Nidset view + walks the bocage in
  `compute_symches`. Step-iteration skips that entirely. The
  gap (~37 % on Article-2025) is recoverable only via libmarpa
  C-side surgery (out of scope) or by skipping ASF entirely on
  wholly-unambiguous forests — which is what HYBRID already does
  from the user's perspective.
- **`compute_symches` per-glade cost is fixed overhead**, not
  amortized by subtree sharing on unambiguous input. This is why
  HYBRID's `metric == 1` fast path matters.
- **The singleton factoring path dominates** in real workloads
  (99.98 % of factorings on Article-2025). Optimising the
  general path further has diminishing returns; optimising the
  singleton path has near-zero returns because it's already
  flat.
- **Flat factoring storage** (`SmallVec<[Vec<usize>; 1]>` over
  `Vec<Vec<usize>>`) is contra-indicated by counter data — most
  symches have one factoring already, inlining the header would
  *increase* memory by ~72 MB on a 3 M-symch workload for ~zero
  runtime gain.

## Open work

1. **`RefCell` → `&mut self` API on bocage metadata caches.**
   Hit rates are non-trivial (OR 67 %, AND 45 %) so the caches
   are useful, but the prior wall measurement suggested
   `RefCell::borrow_mut` overhead roughly offset the FFI
   savings. A non-RefCell API would need a before/after profile
   against `LATEXML_MARPA_ASF_ONLY=1` (the mode where the
   overhead matters most) before adopting.

2. **Demand-driven `rh_value(i)` API.** Closer to Perl's
   `Marpa::R2::ASF::rh_value` model — defer per-RHS-position
   evaluation until the caller asks for it, so semantic pragmas
   that reject a parent can skip child construction. Large API
   change. Gated on corpus evidence that the math parser can
   reject parent alternatives before constructing children;
   without that evidence it would not pay back the complexity.

3. **Caps and downstream cooperation.** The 500-and-node cap is
   a latexml-oxide knob. If new downstream consumers want
   different defaults, surface the routing decision as a
   user-supplied closure rather than a single threshold.

## Test coverage

- `cargo test -p marpa --lib`: 26 unit tests, including the
  `MARPA_ASF_STATS` snapshot/reset round-trip.
- `marpa/tests/asf_traverse_parse.rs`: covers the three
  `HybridParseResult` variants
  (`Unambiguous`/`Ambiguous`/`AmbiguousTree`). The `PanicTraverser`
  proves the ASF callback isn't invoked for unambiguous input.
- `marpa/tests/asf_perf_compare.rs`: wrapper microbench
  (panda + arith Catalan-explosion), now also asserting
  per-paradigm correctness on the same inputs.

## Reproducing the corpus measurements

Math-heavy fixture (100 papers, top by `phase_math_parse_us`),
release+native+cortex profile, 8 workers, 180 s timeout, 8 GB
ulimit. To replicate or check for regression:

```bash
# In latexml-oxide:
cargo build --release --bin cortex_worker --features cortex

# HYBRID (default):
tools/benchmark_canvas.sh \
  --input-dir <math-bound-100-zips>/in \
  --output-dir /tmp/out_hybrid \
  --workers 8 --timeout 180

# LEGACY (control):
env LATEXML_MARPA_LEGACY=1 tools/benchmark_canvas.sh \
  --input-dir <same input> --output-dir /tmp/out_legacy \
  --workers 8 --timeout 180
```

Capture: results.tsv (per-paper wall, status, category) and
`telemetry.jsonl` (phase breakdown). Compare on the both-OK
subset; flag if HYBRID wall climbs back above LEGACY parity.
