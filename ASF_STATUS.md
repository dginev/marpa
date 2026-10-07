# ASF Traversal — Status & Completion Plan

> **Status 2026-05-17**: Steps 2-6 landed on branch
> `asf-step2-symches`. `compute_symches` is fully ported (with Perl-
> faithful predecessor-group unification), the `Traverser` trait was
> redesigned around post-order memoization, and the panda grammar's
> 3 distinct parses are validated via a substantive
> `ExhaustiveTraverser` test. **Step 7 (downstream latexml-oxide
> consumption) is the remaining open work.**
>
> **Downstream context**: the primary consumer of this ASF
> infrastructure is [latexml-oxide](https://github.com/dginev/latexml-oxide),
> whose math parser currently uses Tree-iteration with a 5000-tree
> cap as a defensive bandage against per-tree cost. The migration
> plan to ASF traversal — and the mapping of the three-stage
> "grammar / actions / pragmas" pipeline onto the ASF callback
> model — is documented at
> [latexml-oxide:docs/MATH_PARSER_AND_ASF.md](https://github.com/dginev/latexml-oxide/blob/master/docs/MATH_PARSER_AND_ASF.md).
> Read that doc for *why* this matters; read this one for *what
> needs to be built*.

## What's complete and tested

The state machine around the libmarpa primitives works end-to-end:

| Component | File | Status | Test |
|---|---|---|---|
| `Recognizer` → `Bocage` → `Order` → `Tree` step chain | `src/parser/mod.rs::adv_marpa` | ✅ | implicit via `recce_parse_sanity` |
| `Parser::read` (incremental token consumption) | `src/parser/mod.rs:87` | ✅ | implicit via `recce_parse_sanity` |
| `Parser::run_recognizer` (single-shot iteration that yields a `Tree`) | `src/parser/mod.rs:114` | ✅ | `tests/asf_traverse_parse.rs::recce_parse_sanity` — panda sentence with **3 valid parses** correctly enumerated |
| Grammar reuse across multiple parse calls (G → GReady cycle) | `src/parser/mod.rs::adv_marpa` `GReady → R` arm | ✅ | covered by the second `parse_and_traverse_forest` call in `asf_traverse_parse` not panicking |
| `TreeBuilder` (rollup_token / rollup_rule / discard logic) | `src/tree_builder/builder.rs` | ✅ | `tests/simple_expression_parse.rs` (1 test) + lib unit tests (9 tests in `src/lib.rs`) |
| `Bocage::and_node_cause`, `or_node_irl`, `and_node_symbol`, `and_node_predecessor` raw accessors | `src/thin/bocage.rs` | ✅ | used by `ASF::new` |
| `Order::or_node_and_node_ids` | `src/thin/order.rs` | ✅ | used by `ASF::new` |

## What's scaffolding (types and method signatures exist; bodies are stubs)

**Status as of branch `asf-step2-symches`** — most rows in the prior
audit table have been retired by Step 2 + Step 5.

| Component | File / Line | Status |
|---|---|---|
| `ASF::new` | `src/asf.rs` | ✅ |
| `ASF::peak` | `src/asf.rs` | ✅ |
| `ASF::obtain_glade` | `src/asf.rs` | ✅ |
| `ASF::compute_symches` | `src/asf.rs` | ✅ Step 2 — factoring loop ported from Perl `ASF.pm` with predecessor-group unification (`set_last_choice` semantics). Mirrors `glade_obtain` lines 838-965. |
| `ASF::traverse` | `src/asf.rs` | ✅ Step 5 — post-order recursive driver with `HashMap<glade_id, PT>` memoization. Each glade fires the user callback exactly once. |
| `Glade::rule_id` | `src/asf/glade.rs` | ✅ Step 3 — reads from current symch's `rule_id` field. |
| `Glade::symbol_id` | `src/asf/glade.rs` | ✅ (a byte read through a byte class: the class terminal, 0.4.0) |
| `Glade::token_value` | `src/asf/glade.rs` | ✅ 0.4.0 — the value a token glade's token was read with (a byte scanner's byte + 1); `None` for a rule glade or a nulling token. |
| `Glade::symch_count`, `factor_count`, `is_factored`, `rh_length`, `rh_glade_id`, `next`, `rewind`, `is_token`, `cursor`, `symches()` | `src/asf/glade.rs` | ✅ Step 4 |
| `Glade::literal` (token-glade input span) | — | **deferred** — needs SLR; latexml-oxide math parser doesn't need text spans (token-stream consumer). |
| `Traverser` trait | `src/asf.rs` | ✅ Step 5 — redesigned to `fn(&mut self, &mut Glade, &HashMap<usize, PT>, &mut PS) -> Result<PT>`. |

## What the panda tests prove (post-Steps 2-6)

* **`recce_parse_sanity`** — recognizer-only path admits 3 parses.
* **`ambiguity_metric_oracle_reports_ambiguous`** — pre-flight oracle returns 2.
* **`ambiguity_metric_oracle_reports_unambiguous`** — single-rule grammar returns 1.
* **`asf_three_parses_via_exhaustive_traverser`** — substantive: post-order recursion + memoized `Vec<String>` Cartesian product produces exactly **3 distinct Penn-tagged strings**. This is the load-bearing verification that Steps 2 + 5 work end-to-end.
* **`asf_peak_glade_scaffolding_pin_down`** — invariants:
  * each glade fires exactly once (memoization),
  * an S-shape glade exists (4 RHS positions, 1 symch),
  * a unified-VP-shape glade exists (3 symches, 1 factoring each).
* **`asf_traverse_parse`** — smoke test for both ExhaustiveTraverser and PruningTraverser.

## Completion plan

In priority order. Each step is testable against the panda grammar (3 parses, all distinct via NN/NNS/CC factoring).

### Step 1 — ✅ Pin-down test for scaffolding state — LANDED

(commit `79af103` on prior `asf-completion`).

### Step 2 — ✅ Port `compute_symches` factoring loop — LANDED

Branch `asf-step2-symches`, commit `4c84d7e`. Source: `MARPA_R2/
Marpa/R2/ASF.pm`, the `SYMCH:` loop body. Notable nuance: the Perl
`set_last_choice` + `and_nodes_to_cause_nids` extends contiguous
same-predecessor and-nodes into one nook covering FIRST..LAST and
unifies their causes into a single multi-nid nidset — i.e. multi-
source glade. This unification is faithfully ported and is what
keeps `S` at 1 factoring (with the panda ambiguity localized in
the unified-VP glade) instead of 3 factorings at the top.

### Step 3 — ✅ Fix `Glade::rule_id` — LANDED (piggy on Step 2)

`Glade::rule_id` now reads from `self.symches[cursor.symch_ix].rule_id`. The rule_id lives on the symch (where it conceptually belongs), not on the glade.

### Step 4 — ✅ Most of the `Glade` query API — LANDED (piggy on Step 2)

Implemented: `symch_count`, `factor_count`, `is_factored`, `rh_length`, `rh_glade_id`, `next`, `rewind`, `is_token`, `cursor`, `symches()`. Deferred: `literal()` (needs SLR span lookup; latexml-oxide math parser doesn't need it).

### Step 5 — ✅ Recursive `ASF::traverse` — LANDED

Branch `asf-step2-symches`, commit `e32619c`. The driver walks the
bocage in post-order; each glade fires the user callback exactly
once and child outputs are memoized in `HashMap<glade_id, PT>`.
Cycle-safe via the `visited` flag (defensive — honest bocages are
acyclic).

The `Traverser` trait was redesigned:

```rust
pub trait Traverser {
  type ParseTree;
  type ParseState;
  fn traverse_glade(
    &mut self,
    glade: &mut Glade,
    children: &HashMap<usize, Self::ParseTree>,
    state: &mut Self::ParseState,
  ) -> Result<Self::ParseTree>;
}
```

Single-threaded by design (no Send/Sync bounds, no Arc/Mutex) —
aligned with latexml-oxide's `#[thread_local]` state model.

### Step 6 — ✅ Substantive 3-parse test — LANDED

`asf_three_parses_via_exhaustive_traverser` in
`marpa/tests/asf_traverse_parse.rs` verifies that
`ExhaustiveTraverser` produces exactly **3 distinct Penn-tagged
strings** for the panda sentence. The traverser does a per-glade
Cartesian product across RHS children, reading already-memoized
child outputs from the `children` HashMap.

### Step 7 — ⏳ Apply to downstream latexml-oxide ambiguity reduction

Open. Tracked in
[latexml-oxide:docs/MATH_PARSER_AND_ASF.md](https://github.com/dginev/latexml-oxide/blob/master/docs/MATH_PARSER_AND_ASF.md)
under "Sequencing". Concretely:

1. Switch latexml-oxide's `Cargo.toml` marpa dep to the
   merged-master branch with this ASF work.
2. Refactor `latexml_math_parser::semantics::Actions::action_on`
   signature: `Vec<Option<XM>>` → `(alternatives, &cached_children, ...)`.
3. Rewrite `latexml_math_parser::parser::parse_string`'s tree-
   iteration loop as a `parse_and_traverse_forest` call.
4. Delete 5 of the 6 convergence caps (only `max_time` should
   remain — the rest are bandages against per-tree cost that
   memoization removes).

## Effort estimate

- Step 1 — ✅ 30 min — done (commit `79af103`).
- Steps 2-6 — ✅ ~1 session — done (commits `4c84d7e`, `e32619c`).
  The estimated 1-2 weeks was overcautious; the Rust port came
  together cleanly because we collapsed the Perl nook-stack
  iterator into a single eager DFS that materializes the full
  symch+factoring structure up-front. The trade-off is more memory
  per glade (we hold the whole symch list, not an iterator state),
  but that's bounded by `factoring_max = 42` per Perl's default.
- Step 7 — ⏳ open. Tracked in latexml-oxide.

## Target Rust API (sketch, derived from Marpa::R2::ASF docs)

The Perl interface ([metacpan ASF.pod](https://metacpan.org/dist/Marpa-R2/view/pod/ASF.pod)) is a mutable-iterator design — `glade.next()` advances the glade pointer through alternatives in-place, the user calls `rh_value(i)` to recursively pull child values, and a single mutable scratchpad threads state.

For Rust, the same capability is more idiomatically expressed as:

```rust
/// Result of walking one glade. Token glades have no rule_id and no
/// RHS; rule glades have both.
pub enum GladeKind {
    Token { symbol_id: i32, literal: Vec<u8>, span: Range<usize> },
    Rule  { rule_id: i32, symbol_id: i32, span: Range<usize>, rh: Vec<usize> /* child glade ids */ },
}

pub trait Traverser {
    type Output;

    /// Called at most once per glade (results are memoized by the ASF).
    /// `alternatives` enumerates all symch + factoring combinations at
    /// this glade; the user picks zero, one, or all to compute Output
    /// from. `children` provides already-computed Output values for
    /// each child glade referenced in `alternatives`.
    fn traverse_glade(
        &mut self,
        glade_id: usize,
        alternatives: &[GladeKind],
        children: &HashMap<usize, Self::Output>,
    ) -> Result<Self::Output>;
}

impl<R: Recognizer> ASF<R> {
    pub fn new(recce: R) -> Result<Self>;
    pub fn traverse<T: Traverser>(&mut self, traverser: &mut T) -> Result<T::Output>;
    pub fn ambiguity_metric(&self) -> i32;  // 1 or 2, mirrors Bocage
}
```

Differences from the Perl design and why:

| Perl behavior | Rust shape | Why |
|---|---|---|
| `glade.next()` mutates the glade in place | `alternatives: &[GladeKind]` is the full list | Avoids interior mutability + lifetime headaches. The memory cost is bounded by `Glade::symch_count * factor_count`, which is small for non-pathological grammars. |
| `glade.rh_value(i)` does on-demand recursion into child glades | `children: &HashMap<usize, Output>` is pre-populated | The ASF driver decides traversal order (post-order: children first, then parent) and memoizes results. Users don't trigger recursion; they consume already-computed values. |
| `$scratch_ref` mutable scratch | `&mut self` on the Traverser | Same effect, but typed and scoped to the traverser instance. |
| Token alternatives signalled by `rule_id == undef` | `GladeKind::Token { … }` vs `GladeKind::Rule { … }` | Sum-type makes the discriminant impossible to forget at the call site. |
| `glade.symbol_id()` returns the LHS or token symbol | Both variants of `GladeKind` carry `symbol_id` | Same information, exposed uniformly. |
| `traverse` callback may return any defined value (undef = fatal) | `traverse_glade -> Result<Output>` | `Result` is Rust's idiomatic way to signal "definedness"; the caller can fail the entire traversal by returning `Err`. |

**What's NOT carried over from Perl:**
- The `[:start]` symbol name convention. In our world, the peak glade is just the result `traverse(...)` returns. No magic string.
- The "alternatives within a symch are visited as a group" guarantee. In Rust, `alternatives: &[GladeKind]` is grouped naturally (sort by `rule_id` if the user cares); we don't need to encode that as a traversal-order invariant.
- The pruning style (returning early from `next()` loop). In Rust, the user just doesn't include certain alternatives in their `Output` — semantically equivalent, structurally cleaner.

## Background reading

See [`background/`](background/) for the Kegler 2023 papers (the
recognizer and the nullable-symbols rewrite) and the index pointing
at the Marpa::R2::ASF Perl docs. These are the load-bearing
references for the Step-2 port.
