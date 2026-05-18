# ASF Performance Findings — Consensus Plan

Date: 2026-05-17. Author: original codex analysis + latexml-oxide
session findings (this revision).

This note records optimization opportunities found in the Rust ASF
wrapper implementation, with `latexml-oxide/latexml_math_parser` as the
primary downstream consumer, and the consensus action plan agreed
between the two analyses.

## Context

The ASF implementation is functionally complete enough for downstream
use. `latexml_math_parser` routes math parsing through
`Parser::parse_and_traverse_forest` by default and keeps the legacy
tree-iteration path as an escape hatch behind `LATEXML_MARPA_LEGACY=1`.

The important downstream shape is:

- `MathTraverser::ParseTree = Rc<Vec<Option<XM>>>`.
- `XM::Lexeme` stores `Rc<str>`.
- The latexml workspace patches its `marpa` dependency to this local
  checkout during development (`[patch."https://github.com/dginev/marpa"]`).
- **The latexml legacy `parse_marpa` path has convergence caps (~10
  parses).** Without those caps, legacy Step-iteration would re-walk
  shared subtrees per tree and explode on heavy ambiguity. With the
  caps in place, legacy is bounded and ASF's amortization-of-sharing
  story doesn't get to demonstrate the algorithmic win.

## Baseline

Wrapper microbenchmark (synthetic, no downstream actions):

```text
cargo test --release --test asf_perf_compare -- --nocapture
```

Results:

```text
panda short:   tree 277195 ns, ASF 326666 ns, ratio 0.85x   (ASF slower)
panda long:    tree 4726749 ns, ASF 840393 ns, ratio 5.62x  (ASF faster)
arith 8 ops:   tree 341103 ns, ASF 170627 ns, ratio 2.00x   (ASF faster)
arith 12 ops:  tree 30003612 ns, ASF 293406 ns, ratio 102.26x (combinatorial)
```

The wrapper microbench *confirms ASF asymptotically wins*. The shape
that's missing from `asf_perf_compare` is **per-glade action cost** —
latexml's `Actions::action_on` plus pragma validation dominates real
workload time and isn't simulated by the trivial `CountTraverser`.

Downstream measurements on `Article-2025.tex` (579 math-heavy
formulas, release build, single-thread):

| Stage | ASF wall | LEGACY wall | Delta |
|---|---:|---:|---:|
| Pre-optimization | 18.05s | 12.0s | baseline |
| + `XM::Lexeme(Rc<str>, _)` + thread-local ASCII byte cache | 18.0s | 12.0s | **~0%** |
| + `MathTraverser::ParseTree = Rc<Vec<Option<XM>>>` | 18.0s | 12.0s | **~0%** |
| + marpa cache `HashMap<usize, PT>` → `Vec<Option<PT>>`, slice children API | 17.4s | 12.0s | ~3% |
| + marpa `glades` and `nidset_by_id` → `Vec<Option<_>>` | 16.85s | 12.0s | ~3% more |

**Critical reading of the cumulative ~6% win**: the Rc<str> Lexeme
and Rc<Vec> ParseTree changes contributed *essentially nothing* on
the math-heavy fixture. All the measurable gain came from the
marpa-side HashMap→Vec swaps. This contradicts a common assumption
that string/clone overhead is the bottleneck — for this workload
it's not, the bottleneck is **per-glade `compute_symches` work**
plus possibly **per-glade action dispatch**.

## Critical Review Notes

The codex first-pass analysis had the right broad direction. The
revisions below were agreed across both analyses:

- **Do not call the recognizer twice.** Any hybrid routing must
  read tokens once, build bocage once, then branch on ambiguity.
  Calling `Parser::ambiguity_metric()` followed by
  `Parser::parse_and_traverse_forest()` repeats recognizer work
  and erases the intended win.
- **Instrument before broad refactors**, but instrument cheaply.
  A minimal hybrid prototype on real latexml fixtures is the
  fastest way to prove or disprove the main thesis — don't gate
  the architectural test on a polished counter dashboard.
- **The Rc/clone microcosm doesn't matter as much as it appears
  in profiles**. The downstream baseline measurements on
  `Article-2025.tex` (above) showed the Rc changes flat at ~0%.
  Treat further allocation-style microoptimizations with extreme
  scepticism — `compute_symches` is the structural cost.
- **The `latexml_math_parser` legacy convergence caps must be
  considered when comparing paths.** Legacy is bounded by these
  caps; ASF is bounded only by `factoring_max`. Direct A/B is
  fair on unambiguous input but skewed on ambiguous input.

Treat every optimization below as a hypothesis until measured
against a math-heavy downstream fixture. The wrapper
microbenchmarks are useful for shape, but they are not
representative of `latexml_math_parser`'s semantic action cost or
grammar distribution.

## High-Impact Findings

### 1. Route unambiguous parses away from ASF (highest ROI)

After recognition, build one bocage and check `ambiguity_metric()`.
If the parse is unambiguous, use a tree/value path. If it is
ambiguous, use ASF traversal.

**Important semantic boundary**: `ambiguity_metric()` reports raw
Marpa grammar ambiguity. It does not know about
`latexml_math_parser` semantic actions, pruning, deduplication,
or forest pragmas. A raw-unambiguous tree can still be
semantically rejected, and a raw-ambiguous forest can collapse to
one semantic result. The hybrid branch is a performance choice,
not a semantic classifier.

The current public APIs make this awkward because
`Parser::ambiguity_metric` and `Parser::parse_and_traverse_forest`
each consume a token stream. The wrapper should expose an API
that branches after `Parser::read`, so recognizer work is not
repeated.

**Required marpa API change**: an `ASF::from_bocage(recce,
bocage)` constructor (or equivalent post-`R` state extraction)
so the bocage built for the ambiguity check can be reused for
ASF traversal. Currently `ASF::new(recce)` builds bocage
internally, so the hybrid path would otherwise double the bocage
construction.

Candidate API shape:

```rust
pub enum ForestOrTree<ASFOut, TreeOut> {
  Unambiguous(TreeOut),
  Ambiguous(ASFOut),
}
```

or a lower-level parser method that accepts two callbacks after
the recognizer reaches `R`.

Better internal shape:

1. `Parser::read(tokens)` reaches `R(Recognizer)`.
2. Move the recognizer out of the parser state once.
3. Build `Bocage` once.
4. Check `Bocage::ambiguity_metric()`.
5. For unambiguous input, build `Order` and `Tree` from that bocage.
6. For ambiguous input, build `ASF` from the already-created bocage
   instead of calling `ASF::new(recce)`.

**Concrete downstream wiring**: `latexml_math_parser::parse_marpa`
would route on the ambiguity result:
- unambiguous → existing legacy `TreeBuilder::actions_on_value`
  path (no convergence caps needed when only one tree exists)
- ambiguous → `MathTraverser` ASF path

This keeps two semantic paths alive — that's the cost. Risk
controls (parity tests, audit mode comparing both paths on
raw-unambiguous formulas) are essential.

Why this matters:

- Preserves ASF where it wins (5x to 100x on
  `asf_perf_compare`).
- Avoids ASF setup/factoring overhead for the common
  unambiguous formula path.
- Keeps the C libmarpa implementation untouched.
- The convergence caps in legacy are exactly what makes legacy
  fast on unambiguous input (no enumeration happens when there's
  only one tree to enumerate).

Risk:

- Two semantic paths in `latexml_math_parser`.
- Requires parity tests.
- Can change output if the ASF path and legacy tree path have
  subtle semantic differences even for raw-unambiguous parses.

Risk control:

- Collect counts first: how often is `ambiguity_metric() == 1`
  on the target corpus? `latexml_math_parser` should add a cheap
  counter behind an env var.
- For unambiguous formulas, run both paths in audit mode on a
  sample and assert identical `XM` output before enabling hybrid
  by default.
- Include semantically rejected raw-unambiguous formulas in the
  audit; failure behavior must match too.

### 2. Add a singleton fast path in `compute_symches`

Current hot area:

- `marpa/src/asf.rs::compute_symches`
- `marpa/src/asf.rs::collect_factorings`

The general path allocates and transforms:

- `source_data`
- `group_nids`
- `raw_factorings: Vec<Vec<Vec<i32>>>`
- `work_stack`
- `groups`
- final `factorings: Vec<Vec<usize>>`

For singleton OR-node/AND-node chains, this is avoidable. A direct
predecessor-chain walk can emit one factoring without building
grouped intermediate structures, cloning `work_stack`, or
reversing a cloned stack at every leaf.

Expected implementation:

1. Detect that a source nid's predecessor chain has exactly one
   AND node per OR node.
2. Collect cause nids into one scratch buffer.
3. Register child glades directly.
4. Emit a `Symch` with one factoring.
5. Fall back to the existing general algorithm when branching
   appears.

This is the most promising Rust-side ASF-only improvement for
unambiguous parses if hybrid routing is not acceptable. Should
still be measured *after* the hybrid experiment because a
branch-away strategy may make this optimization less important
for the downstream workload — but it's the right thing to land
even with hybrid, because the ambiguous path also visits many
singleton-chain glades inside an otherwise-ambiguous forest.

### 3. Cache bocage metadata in Rust

Current factoring and nid classification repeatedly cross FFI for:

- AND-node cause
- AND-node predecessor
- AND-node symbol
- OR-node IRL
- IRL to XRL
- XRL LHS

Add lazy caches inside `ASF`:

```rust
struct OrNodeInfo {
  irl_id: i32,
  xrl_id: i32,
  lhs_id: i32,
}

struct AndNodeInfo {
  cause: i32,
  predecessor: Option<i32>,
  symbol: i32,
}
```

Back them with `Vec<Option<_>>` indexed by node id. Use the
caches from `collect_factorings`, `nid_sort_ix`, `nid_rule_id`,
`nid_token_id`, and `nid_symbol_id`.

Expected benefit:

- Fewer libmarpa FFI calls in the inner factoring path.
- Centralized error handling for raw bocage metadata.
- Easier instrumentation of bocage shape.

### 4. Flatten factoring storage

Current `Symch` stores:

```rust
Vec<Vec<usize>>
```

Most RHS factorings are short, so this creates many small heap
allocations. Options:

1. Use `SmallVec<[usize; 4]>` for each factoring.
2. Use a flat arena:

```rust
struct Symch {
  rule_id: i32,
  factoring_ranges: Vec<Range<usize>>,
  rhs_glades: Vec<usize>,
  omitted: bool,
}
```

The flat arena is more invasive but likely better for cache
locality and allocation volume.

**Senior-engineering caution**: do not start here. This is the
kind of change that can make the implementation harder to reason
about while only moving the needle a few percent. Use allocation
profiles or the ASF stats counters to prove that small `Vec`
allocation is still material after singleton fast paths and
metadata caching.

### 5. Clean up traversal internals

Current traversal has several fixable costs and one defensive-code
bug:

- `ASF::traverse` is generic, but `traverse_glade_recursive`
  accepts `&mut dyn Traverser`, reintroducing dynamic dispatch
  internally.
- **Duplicate `cache_set` calls**: the parent loop writes child
  outputs into the cache (`cache_set(cache, child_id,
  child_output)`), and then the recursive call's tail *also*
  writes to the cache (`cache_set(cache, glade_id,
  output.clone())`). For shared children the second write is
  redundant; for first-time visits the explicit insert in the
  parent loop is redundant because the recursion sets it itself.
- `visited` is set but not checked, so the stated cycle
  protection is not effective.

Recommended changes:

1. Make `traverse_glade_recursive` generic over `TR: Traverser`.
2. Let recursive calls cache their own output; remove the
   parent-side duplicate `cache_set`.
3. Replace `visited: bool` with an explicit traversal mark:

```rust
enum VisitState {
  Unseen,
  Visiting,
  Done,
}
```

or move to an iterative post-order traversal.

These are not expected to close the downstream gap by themselves,
but they are low-risk cleanup before larger factoring refactors.

### 6. Avoid eager Cartesian product materialization downstream

In `latexml_math_parser/src/asf_traverser.rs`,
`MathTraverser::dispatch_action` still materializes all
combinations as:

```rust
Vec<Vec<Option<XM>>>
```

Replace this with an odometer iterator plus one reusable scratch
vector. Larger follow-up: change `Actions::action_on` from:

```rust
Vec<Option<XM>>
```

to:

```rust
&[Option<XM>]
```

and clone only in actions that need owned values.

This is downstream work, not wrapper work, but it is directly
exposed by the ASF API shape and should be benchmarked with
wrapper changes.

**Caveat from downstream session**: the cartesian fast path for
`total == 1` already exists and saves the Vec<Vec<>> chain in
the common case. The odometer is only material when ambiguous
parses with `total > 1` are common, which on
`latexml_math_parser`'s typical math workload they aren't.

## Instrumentation Plan

Before implementing broad refactors, add optional ASF stats under
an env var, for example `MARPA_ASF_AUDIT=1`.

Useful counters:

- OR-node count
- AND-node count
- glade count
- symch count
- factoring count
- omitted factoring count
- max source nids per glade
- max factorings per symch
- number of singleton fast-path hits
- number of general factoring fallback hits
- time in `ASF::new`
- time in `compute_symches`
- time in user traversal callbacks

This should be available to downstream callers so
`latexml_math_parser` can correlate formula-level slowdowns with
ASF shape.

**Downstream parallel counter**: `LATEXML_MATH_AMBIGUITY_AUDIT=1`
would tally per-formula `ambiguity_metric()` results across a
corpus. Output: "of N formulas, K had metric==1, (N-K) had
metric==2". This is the single most important measurement for
deciding whether hybrid routing is worth the complexity.

## Consensus Decision Tree

The implementation path should be driven by measured corpus
shape:

1. **If most formulas are raw-unambiguous** (best guess from
   manual inspection of latexml math: 80%+), prioritize hybrid
   routing. ROI is the highest of any single change.
2. **If many formulas are ambiguous but most ASF glades are
   singleton chains** within ambiguous forests, prioritize
   `compute_symches` fast paths.
3. **If profiling shows repeated FFI metadata calls** in hot
   stacks, prioritize bocage metadata caches.
4. **If allocation profiles still show many tiny factoring
   allocations**, then flatten `Symch` storage.

This avoids spending engineering time polishing the general ASF
representation if the actual workload mostly needs a cheap
unambiguous-path escape hatch.

## Proposed Sequencing

1. **Minimal audit counters** for ambiguity-metric distribution
   on a math-heavy downstream fixture (Article-2025.tex, 579
   formulas; the existing 1011.1955, etc. from the standing
   perf corpus). Run once, record the K/N ratio.
2. **Prototype the post-recognition hybrid API** without
   duplicating token scanning or bocage construction. Requires
   `ASF::from_bocage` constructor in marpa.
3. **Validate hybrid semantic parity** on raw-unambiguous
   formulas: assert ASF output == legacy output on a sample
   corpus before flipping the default.
4. **Implement singleton fast path** in `compute_symches` if
   ASF-only cost remains important after hybrid lands (matters
   for the ambiguous-path glades that are themselves singletons).
5. **Add bocage metadata caches** if FFI metadata calls show up
   in profiles.
6. **Clean traversal internals**: generic recursion, duplicate
   cache write removal, real cycle marking.
7. **Flatten factoring storage** if counters still show
   allocation pressure.
8. **Optimize downstream Cartesian product enumeration** —
   already partially landed in `dispatch_action`'s `total == 1`
   fast path; the odometer pattern is the remaining work.

## Validation

Wrapper validation:

```text
cargo test --release --test asf_perf_compare -- --nocapture
cargo test --workspace
```

Downstream validation:

```text
LATEXML_PARSE_AUDIT=1 cargo test ...
```

Use at least:

- a mostly unambiguous math-heavy paper or fixture (e.g.
  `Article-2025.tex` from the session, `1912.03329` from
  PERFORMANCE.md)
- known ambiguous formulas such as the `sin[XY]` family
- the existing latexml ASF parity tests (1301 tests, must
  remain 1301/0)
- a corpus-level benchmark after any hybrid routing change

Acceptance criteria should include both speed and semantics:

- No change in existing ASF correctness tests.
- No latexml parity regression on raw-unambiguous formulas.
- No duplicate token scan in the hybrid path.
- No duplicated bocage construction in the benchmarked hybrid
  path.
- Measured improvement on a mostly unambiguous math-heavy
  fixture (target: ASF ≤ 1.05x LEGACY wall on
  `Article-2025.tex`).
- No regression on known ambiguous cases where ASF is expected
  to win.

## Summary

The main optimization opportunity is to **avoid paying full ASF
factoring cost when the parse is unambiguous**. The downstream
measurements confirm this: the Rust-side allocation reductions
(`Rc<str>`, `Rc<Vec>`, `Vec<Option<>>` caches) shaved ~6% total
on a math-heavy paper, of which the Rc-based changes
contributed approximately 0% and the marpa HashMap→Vec swaps
contributed approximately 6%. To close the remaining ~30%
ASF→LEGACY gap on `Article-2025.tex` without a fundamental
algorithmic change, **hybrid routing is the only candidate with
expected single-digit-percent gap closure**.

If ASF-only remains a requirement, the next best wrapper-side
target is a singleton fast path inside `compute_symches`,
followed by bocage metadata caching. Both should be measured
on the downstream workload, not the synthetic wrapper benchmark.

Micro-optimizations are still worthwhile, but the already-landed
`Vec<Option<_>>`, `Rc<Vec<_>>`, and `Rc<str>` changes did NOT
move the bottleneck materially. The remaining work should focus
on algorithmic branching and fast paths for the common parse
shapes seen by `latexml_math_parser`.

## Disclaimers

This document is hypothesis-driven until measured. Specifically:

- We do not yet know the per-corpus distribution of
  `ambiguity_metric() == 1` vs `== 2`. Step 1 of the sequencing
  must establish this before the hybrid prototype.
- The "Rc<> changes were ~0%" measurement is based on
  `Article-2025.tex` alone. Other workloads (very ambiguous
  formulas with much sharing) could show different shape.
- The 12s legacy baseline benefits from convergence caps. If
  those caps were ever removed, legacy could lose its advantage
  on ambiguous inputs and the priority ordering might shift.

## First-Pass Implementation Review (2026-05-17)

Codex landed the consensus plan's hybrid routing as a first pass.
Files modified:

- `marpa/src/parser/mod.rs` — `Parser::parse_hybrid<T, U, TR>(...)`
  returning `HybridParseResult<PT, PS>` (`Unambiguous(Tree)` |
  `Ambiguous(PT, PS)`)
- `marpa/src/asf.rs` — `ASF::from_parts(recce, bocage)` constructor;
  existing `ASF::new(recce)` now delegates
- `marpa/tests/asf_traverse_parse.rs` — two new tests:
  `hybrid_parse_returns_tree_for_unambiguous_input` (asserts ASF
  branch is *not* taken on unambiguous input via a `PanicTraverser`)
  and `hybrid_parse_traverses_asf_for_ambiguous_input`
- `latexml_math_parser/src/parser.rs` — hybrid dispatch wired
  behind `LATEXML_MARPA_HYBRID=1` (or implicit under
  `LATEXML_MATH_AMBIGUITY_AUDIT=1`); reuses
  `Actions::get_tree` for unambiguous trees and `MathTraverser`
  for ambiguous forests

### What the first pass got right

- **One-pass design**. `parse_hybrid` reads tokens once, builds
  bocage once, branches on `ambiguity_metric()`. Avoids the
  "compose `ambiguity_metric()` + `parse_and_traverse_forest()`"
  anti-pattern explicitly called out in the consensus plan.
- **`ASF::from_parts(recce, bocage)`** matches the consensus
  recommendation exactly: it reuses an already-constructed bocage.
- **`PanicTraverser` test pattern** is a much stronger assertion
  than just checking the returned variant — it proves the user
  callback is never invoked on the unambiguous branch.
- **`HybridParseResult<PT, PS>` shape** is asymmetric in a way
  that reflects the underlying reality: the unambiguous branch
  returns a raw `Tree` iterator, the ambiguous branch returns an
  ASF-traversed `PT`. Callers have to handle both, but the names
  make that obvious.
- **Staged rollout downstream**: hybrid is opt-in, not default.
  Allows measurement under audit + soak before flipping.
- **Audit instrumentation** matches Step 1 of the sequencing —
  counts per-formula ambiguity metric distribution before any
  default change.
- **`drop(traverser)` before reusing `nodes`/`document`** in
  `ActionContext` correctly handles the borrow-conflict between
  `MathTraverser::document: &'a mut Document` and the downstream
  `ActionContext::document`.

### Issues that should land before flipping hybrid on by default

1. **No parity assertion**. The consensus plan required (verbatim):
   *"For unambiguous formulas, run both paths in audit mode on a
   sample and assert identical XM output before enabling hybrid
   by default."* This is missing from the first pass. Acceptable
   shapes:
   - A `LATEXML_MARPA_HYBRID_AUDIT_PARITY=1` mode that runs both
     the hybrid-unambiguous path and the legacy/ASF path on every
     raw-unambiguous formula and asserts identical `XM` output, OR
   - A test fixture in `latexml_oxide/tests/parse/` that asserts
     identical `.xml` output under `LATEXML_MARPA_LEGACY=1` and
     `LATEXML_MARPA_HYBRID=1`

2. **Asymmetric pruning semantics across branches**. The
   unambiguous branch counts `Err` from `get_tree` once. The
   ambiguous branch tallies `traverser.pruned_count` plus
   result-Vec dedup separately. Both feed into the same
   `pruned_trees` audit variable. This is fine as a count, but
   downstream tests that compare prune counts across paths will
   see drift. Either:
   - Document that hybrid `pruned_trees` is not directly
     comparable to legacy/ASF, OR
   - Normalize the counts (decide which kind of "prune" the
     audit counter represents and align both branches)

3. **`record_ambiguity_metric` atomics**. The function uses
   `Ordering::Relaxed` increments followed by `Ordering::Relaxed`
   reads — the read after the conditional fetch_add is not
   atomic with the increment, so a concurrent caller can print
   interleaved or stale counter values. Single-threaded today,
   but the cortex_worker path runs concurrent parses. Either:
   - Add a comment that audit output is best-effort and
     single-thread-only
   - Use a single fetch_add and load the running total once

4. **`parse_hybrid` state asymmetry**: the unambiguous branch
   sets `self.state = T(tree.clone())`; the ambiguous branch
   leaves `self.state = GReady` (because the `mem::replace` in
   the entry already put it there). The downstream caller is
   one-shot for math parsing, so neither is reused — but a
   future caller that *does* reuse the parser will see different
   resumption shapes. Either:
   - Make the ambiguous branch also set `self.state` to a
     defined post-traversal state, OR
   - Document explicitly that `parse_hybrid` consumes the
     parser's R-state regardless of branch

5. **No perf measurement landed yet**. Codex reported "testing"
   which is the 1301-test suite. The load-bearing measurement is
   the `Article-2025.tex` benchmark under `LATEXML_MARPA_HYBRID=1`.
   Target from the consensus plan: ASF wall ≤ 1.05x LEGACY wall.

### Acceptance gate before flipping hybrid default

Before changing `PARSE_VIA_HYBRID`'s default from "opt-in" to
"always-on-unless-LEGACY-flag":

1. **1301/0 test suite** under `LATEXML_MARPA_HYBRID=1`. Same
   bar as ASF default.
2. **Parity assertion** between hybrid-unambiguous and
   legacy/ASF on raw-unambiguous formulas — either as a test
   fixture or an audit-mode equality check.
3. **Article-2025.tex wall** ≤ 1.05× LEGACY (~12.6s) on the
   release build.
4. **Ambiguous fixture regression check**: known-ambiguous
   formulas (`sin[XY]` family) still produce expected results
   under hybrid (they exercise the ASF branch).
5. **Audit-counter sanity**: a corpus run with
   `LATEXML_MATH_AMBIGUITY_AUDIT=1` reports a sensible
   unambiguous:ambiguous ratio (manual inspection — if the
   unambiguous fraction is <50%, hybrid won't deliver the
   expected wall reduction and we should revisit whether the
   added complexity is worth it).
