# ASF Traversal — Status & Completion Plan

> Audit performed 2026-05-17 across the 17 commits on
> `abstract_syntax_forests` that aren't on `master`. The work is
> **scaffolding-stage**, not feature-complete.
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

| Component | File / Line | What's there | What's missing |
|---|---|---|---|
| `ASF::new` | `src/asf.rs:67-105` | Walks the bocage, populates `or_nodes: Vec<Nidset>`. | nothing missing **for construction** itself — but `glades`, `powerset_by_id`, `intset_by_key` are intentionally left empty, populated lazily by `obtain_glade`. |
| `ASF::peak` | `src/asf.rs:113-124` | Discovers the start-rule glade by chasing `bocage.top_or_node()` → `and_node_cause()`. | Works. |
| `ASF::obtain_glade` | `src/asf.rs:126-137` | Returns cached glade if `symches` already populated, else calls `compute_symches`. | Works. |
| `ASF::compute_symches` | `src/asf.rs:139-211` | Sorts source nids by `nid_sort_ix`, partitions by sort_ix to build symch_ids, calls `obtain_powerset`. | **Inner symch loop body is commented-out Perl** (lines 165-210). `symches` always ends as `Vec::new()`. No factoring stack, no choicepoint navigation. |
| `ASF::traverse` | `src/asf.rs:107-111` | Computes peak glade, calls `traverser.traverse_glade(peak_glade, init_state)` **once**. | Recursion into child glades is missing. Iteration over symches is missing. |
| `Glade::rule_id` | `src/asf/glade.rs:25-27` | Returns `self.id`. | **BUG**: `id` is the glade-id (= nidset-id), not the rule-id. Should call `nid_rule_id(asf, nid0)` against the underlying nidset. |
| `Glade::symbol_id` | `src/asf/glade.rs:29-31` | Returns precomputed `symbol_id`. | Works (set in `compute_symches:201`). |
| `Glade` — other methods | `src/asf/glade.rs` | Only `rule_id` (buggy) + `symbol_id`. | Missing: `literal()`, `rh_length()`, `rh_value(ix)`, `next()`, `is_factored()`, `symch_count()`, `factor_count()`, `rh_glade_id(ix)`. |
| `Traverser` trait | `src/asf.rs:12-17` | Trait with `traverse_glade(glade, state) -> (ParseTree, ParseState)`. | The trait shape is fine; what's missing is the **recursive driver** in `ASF::traverse` that walks the glade tree and threads state through child traversals. |

## What the existing `asf_traverse_parse` test actually proves

```rust
let runner_result = runner_asf_traverse();
assert!(runner_result.is_ok(), "failed to run asf traversal: {:?}", ...);
```

The test only asserts the call doesn't panic. Inside, `runner_asf_traverse` calls `parse_and_traverse_forest` twice with two no-op traversers (`ExhaustiveTraverser` and `PruningTraverser` both return `Ok(((), ()))`). With `symches` always empty, the traverser is invoked **exactly once on the peak glade**, with no recursion. So the test currently proves only:

1. `Parser::read` consumes the panda input without error.
2. `ASF::new` completes for a 3-parse grammar.
3. `ASF::peak` finds a glade.
4. `Traverser::traverse_glade` is invoked exactly once with valid `rule_id` (currently == glade_id, not rule_id) and `symbol_id` arguments.

It does **not** prove that distinct parses are enumerated, that factorings are explored, or that pruning works.

## Completion plan

In priority order. Each step is testable against the panda grammar (3 parses, all distinct via NN/NNS/CC factoring).

### Step 1 — Lock in current behavior with a pin-down test

Before completing missing logic, capture exactly what the existing 13 tests + 1 new test prove, so future refactors can't silently regress the scaffolding. **Land first** (test added in this branch).

### Step 2 — Port `compute_symches` factoring loop from Perl `Marpa::R2::ASF`

Source: `MARPA_R2/Marpa/R2/ASF.pm`, the `SYMCH:` loop body and the
`Marpa::R2::Choicepoint` helpers (`first_factoring`,
`next_factoring`, `glade_id_factors`, `nid_rule_id`).

Outcome: `Glade::symches` gets populated with `Vec<usize>` of symch-ids; each symch carries a factoring list.

### Step 3 — Fix `Glade::rule_id`

The current `self.id` return is the **nidset-id**, not the **rule-id**. Replace with a call that, for a non-token nid, walks `nid → or_node_irl → source_xrl` to recover the XRL id (= rule id). For a token nid (`nid < NID_LEAF_BASE`), return a sentinel like `0` or `-1` per Perl convention.

### Step 4 — Add the rest of the `Glade` query API

| Method | Returns | Wraps |
|---|---|---|
| `literal(&self) -> &[u8]` | Token bytes for token-glades | Recognizer input span lookup |
| `rh_length(&self) -> usize` | RHS length of the chosen symch's chosen factoring | Factoring stack inspection |
| `rh_value(&self, ix: usize) -> Vec<Handle>` | Children at RHS position `ix` | Recursion into child glades |
| `rh_glade_id(&self, ix: usize) -> usize` | Glade id at RHS position `ix` | Factoring stack lookup |
| `next(&mut self) -> Option<()>` | Advance to next factoring within current symch | `next_factoring(choicepoint, nid)` |
| `symch_count(&self) -> usize` | How many symches | `self.symches.len()` |
| `factor_count(&self) -> usize` | How many factorings on current symch | Factoring stack height |
| `is_factored(&self) -> bool` | True if current symch has > 1 factoring | `factor_count() > 1` |

### Step 5 — Make `ASF::traverse` recursive

Current body invokes `traverse_glade` once on the peak. The real driver needs to:

1. For each symch in the glade, for each factoring of that symch, recurse into each child glade via `rh_value(ix)`.
2. Thread `ParseState` through the recursion per the trait's signature.
3. Honor a `Glade::visited` flag (the field exists at `src/asf/glade.rs:9` but is dead code today) to avoid re-traversing shared sub-glades.

### Step 6 — Replace the no-op test with a substantive test

Goal: ExhaustiveTraverser should produce exactly **3 distinct Penn-tagged strings** for the panda sentence, matching the Marpa::R2 reference output:

```
(S (NP a panda) (VP eats (NP shoots and leaves)) .)
(S (NP a panda) (VP (VP eats shoots) and (VP leaves)) .)
(S (NP a panda) (VP eats (NN shoots) and (NN leaves)) .)
```
(or whatever set Marpa::R2 yields against this grammar — needs to be regenerated as the reference.)

The PruningTraverser should pick one specific factoring (e.g. always the leftmost branch) and produce exactly 1 result.

### Step 7 — Apply to downstream latexml-oxide ambiguity reduction

Once Steps 2-6 land, the latexml-oxide math parser can use `parse_and_traverse_forest` with a custom pruning traverser to cut the post-parse tree-enumeration cost. Currently latexml-oxide's marpa wrapper iterates all parse trees up to a 5000-cap before applying pragma rules; with ASF pruning, semantic constraints can prune at each glade, eliminating combinatorial blowup before it materializes.

## Effort estimate

- Step 1 — 30 min (committed; pin-down test + Glade API cleanup +
  `Parser::ambiguity_metric` pre-flight oracle).
- Steps 2-5 — 1-2 weeks of focused porting + testing. The Perl source is ~500 lines in `ASF.pm` plus `~Choicepoint.pm`; needs Rust ownership/borrowing rewrite (factoring stack with shared glade references is the tricky bit).
- Step 6 — 1 day, gated on Steps 2-5.
- Step 7 — separate effort in the latexml-oxide repo.

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
