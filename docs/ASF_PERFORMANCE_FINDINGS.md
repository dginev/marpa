# ASF Performance Findings — Current Plan

Date: 2026-05-17.

This note tracks the optimization plan for the Rust ASF wrapper, with
`latexml-oxide/latexml_math_parser` as the primary downstream workload.
It has been compressed from the longer first-pass review into the
current decision record, measurements, and remaining work.

## Current Conclusion

The highest-impact optimization is **hybrid routing**:

1. Read tokens once.
2. Build one bocage.
3. Check `Bocage::ambiguity_metric()`.
4. Use the normal tree/value path when the raw parse is unambiguous.
5. Use ASF traversal only when the raw parse is ambiguous.

This preserves ASF where it wins on combinatorial ambiguity while
avoiding ASF factoring overhead on the common unambiguous math path.
The implementation must not call the recognizer twice, and it must not
build a second bocage for the ASF branch.

## Load-Bearing Measurements

Wrapper microbenchmark, synthetic and action-light:

```text
cargo test --release --test asf_perf_compare -- --nocapture

panda short:   tree 277195 ns, ASF 326666 ns, ratio 0.85x
panda long:    tree 4726749 ns, ASF 840393 ns, ratio 5.62x
arith 8 ops:   tree 341103 ns, ASF 170627 ns, ratio 2.00x
arith 12 ops:  tree 30003612 ns, ASF 293406 ns, ratio 102.26x
```

The wrapper benchmark confirms the ASF asymptotic win, but it does not
model downstream semantic-action cost.

Downstream `Article-2025.tex`, release build, single-thread:

| Mode | Wall (s) | vs LEGACY |
|---|---:|---:|
| `LATEXML_MARPA_LEGACY=1` | 12.21 | 1.00x |
| `LATEXML_MARPA_HYBRID=1` | **12.40** | **1.015x** |
| ASF default | 17.00 | 1.39x |

Hybrid satisfies the target of ASF/hybrid wall time being at most
1.05x the legacy tree path on this fixture.

Raw ambiguity distribution on the same fixture with
`LATEXML_MATH_AMBIGUITY_AUDIT=1`:

```text
metric=1 (unambiguous): 3405 calls
metric=2 (ambiguous):    497 calls
total:                  3902 calls
unambiguous fraction:    87.3%
```

This explains the measured improvement: most formulas should not pay
full ASF factoring cost.

## What Did Not Move The Needle

Earlier measurements on `Article-2025.tex` showed that allocation-style
micro-optimizations were not the main bottleneck:

| Change | Observed effect |
|---|---:|
| `XM::Lexeme(Rc<str>, _)` and thread-local ASCII cache | ~0% |
| `MathTraverser::ParseTree = Rc<Vec<Option<XM>>>` | ~0% |
| marpa cache `HashMap<usize, PT>` -> `Vec<Option<PT>>` and slice children API | ~3% |
| marpa `glades` / `nidset_by_id` -> `Vec<Option<_>>` | ~3% |

The practical bottleneck is structural: per-glade
`compute_symches`/factoring work and downstream semantic action
dispatch. Further clone/string work should be treated skeptically
unless a new profile proves it material.

## Implementation Status

Hybrid routing first pass:

- `marpa/src/asf.rs`
  - `ASF::from_parts(recce, bocage)` reuses an already-created bocage.
  - `ASF::new(recce)` delegates to `from_parts`.
  - `try_singleton_factoring(...)` fast-paths unbranched predecessor
    chains and falls back to the general factoring algorithm when any
    branch appears.
  - Recursive traversal is generic over the concrete traverser, avoids
    redundant parent-side child cache writes, and uses explicit visit
    state for cycle detection.
- `marpa/src/parser/mod.rs`
  - `HybridParseResult<PT, PS>` reports `Unambiguous(Tree)` or
    `Ambiguous(PT, PS)`.
  - `Parser::parse_hybrid(...)` reads once, builds bocage once, and
    branches on `ambiguity_metric()`.
  - `Parser::with_precomputed_grammar(grammar)` supports downstream
    parity-audit runs that reuse precomputed grammars.
- `marpa/tests/asf_traverse_parse.rs`
  - Tests cover the unambiguous tree branch and ambiguous ASF branch.
  - `PanicTraverser` proves the ASF callback is not invoked for
    unambiguous input.
- `latexml_math_parser/src/parser.rs`
  - `LATEXML_MARPA_HYBRID=1` enables hybrid routing.
  - `LATEXML_MATH_AMBIGUITY_AUDIT=1` records raw ambiguity counts.
  - `LATEXML_MARPA_HYBRID_AUDIT_PARITY=1` runs both paths for
    raw-unambiguous formulas and compares outcomes.

Current acceptance status:

| Gate item | Status |
|---|---|
| 1301/0 latexml test suite under `LATEXML_MARPA_HYBRID=1` | confirmed |
| `Article-2025.tex` hybrid wall <= 1.05x legacy | confirmed |
| Ambiguity distribution sanity | confirmed, 87.3% raw-unambiguous |
| Ambiguous fixture regression (`sin[XY]` family) | still needs explicit run |
| Parity audit determinism | canonicalized comparison implemented; broader stress run pending |

## Parity Audit Status

`LATEXML_MARPA_HYBRID_AUDIT_PARITY=1` compares the tree-path and
ASF-path `XM` outputs after running actions twice. Some actions are not
pure:

- `create_xmrefs` allocates generated `xmkey` / `idref` values through
  `get_xmarg_id()`.
- The second pass can produce semantically identical trees with
  different generated identifiers.
- A direct `assert_eq!` can therefore report false mismatches.

The audit now canonicalizes generated ids before comparing:

1. Clone the two `ParseOutcome` values.
2. Walk each `XM` tree in deterministic pre-order.
3. Replace generated `xmkey`, `idref`, and auto-generated `id` values
   with structural placeholders such as `#1`, `#2`, ...
4. Compare the canonicalized outcomes.

This preserves the audit's purpose: structural semantic parity, not
literal equality of side-effect-generated identifiers. The remaining
work is validation, not the mechanism itself: run the audit on formulas
that exercise `XMDual`, `XMRef`, generated ids, rejections, empty
results, and known ambiguous fixtures.

## Next Wrapper-Side Optimizations

Do these only after hybrid routing is stabilized and measured:

1. Cache bocage metadata in Rust.
   - Cache OR-node IRL/XRL/LHS and AND-node cause/predecessor/symbol in
     `Vec<Option<_>>` indexed by node id.
   - Use the cache from `collect_factorings`, `nid_sort_ix`,
     `nid_rule_id`, `nid_token_id`, and `nid_symbol_id`.
2. Consider flat factoring storage only if allocation profiles still
   show many tiny `Vec<Vec<usize>>` allocations after the above work.

## Downstream Follow-Up

`latexml_math_parser/src/asf_traverser.rs` still has an ambiguous-path
Cartesian product materialization:

```rust
Vec<Vec<Option<XM>>>
```

There is already a `total == 1` fast path, so an odometer iterator is
only likely to matter for formulas with substantial semantic ambiguity.
This should be driven by corpus data rather than done speculatively.

## TO CLAUDE

Please help with independent validation while codex works on the
implementation side.

1. **Parity audit stress cases**
   - Run `LATEXML_MARPA_HYBRID_AUDIT_PARITY=1` on formulas that exercise
     `XMDual`, `XMRef`, `xmkey`, `id`, and `idref`.
   - Find the smallest real formula where the current direct comparison
     false-fails.
   - Report the minimal input, failure diff, and whether canonicalizing
     generated ids makes the mismatch disappear.

2. **Post-hybrid `Article-2025.tex` timing**
   - Compare:
     - `LATEXML_MARPA_LEGACY=1`
     - ASF default
     - `LATEXML_MARPA_HYBRID=1`
   - Include wall time, release/debug mode, worker count, and whether
     the run was warmed or cold.

3. **Broader ambiguity distribution**
   - Run `LATEXML_MATH_AMBIGUITY_AUDIT=1` on a broader math-heavy
     corpus.
   - Report total parse calls, raw-unambiguous count, raw-ambiguous
     count, and unambiguous percentage.

4. **Semantic mismatch search**
   - Focus on raw-unambiguous formulas that are rejected, return empty,
     use refs/dual content, or interact with math pragmas.
   - Report any formula where canonicalized tree-path output and
     canonicalized ASF-path output still differ.

5. **Ambiguous fixture regression**
   - Re-run known ambiguous formulas, especially the `sin[XY]` family,
     under `LATEXML_MARPA_HYBRID=1`.
   - Confirm they exercise the ASF branch and preserve expected output.

## FROM CLAUDE (2026-05-17, validation results)

Codex's 5-item list executed. Important correction first, then findings.

### Correction: the audit is NOT compromised by xmkey drift

My earlier safety review claimed the audit would false-fail on
`get_xmarg_id()` counter drift. **That mechanism is invisible to
the audit** because `XProps::eq` in
`latexml_math_parser/src/semantics/tree.rs:71-89` already skips
`id`, `idref`, `xmkey` (and `scriptpos`) by design:

```rust
impl PartialEq for XProps {
  fn eq(&self, other: &Self) -> bool {
    self.content == other.content
      && self.role == other.role
      && self.meaning == other.meaning
      && self.name == other.name
      // Skip: scriptpos — layout hint, not semantic distinction
      // Skip: id, idref, xmkey — bookkeeping for Dual/Ref resolution
      && self.font == other.font
      // ...
  }
}
```

Codex's `canonicalize_parse_outcome_ids` helper (parser.rs:148)
adds a second layer of id-canonicalisation on top of this. That's
defensive — fine to keep — but the underlying `assert_eq!` was
already id-insensitive via `XProps::eq`, so the canonicalisation
is technically belt-and-suspenders rather than load-bearing.

The audit DOES still fire on real corpus, but for a different
reason than I predicted — see Task 1 below.

### Task 1 — Parity audit stress, minimal repro

`LATEXML_MARPA_HYBRID_AUDIT_PARITY=1` does fire on real
corpus. The smallest formula I found that triggers it:

```latex
$\{u | a = b, c = d\}$
```

Trigger ingredients (all required):

1. `\{...\}` curly fences (set-builder presentation shape)
2. `u | ` — a single VERTBAR splitting the body
3. A relation on the left of the comma (`a = b`)
4. A second relation on the right of the comma (`c = d`,
   also reproduces with `\geq`)

Removing any of these (no braces; no leading `u |`; only one
relation; replacing `=` with a non-relation) makes the audit
pass.

Diff captured:

```
left: Empty
right: Rejected("infix_relation: left formula ends with list
                 (comma should be formula boundary)")
```

(`left` = ASF outcome, `right` = Tree outcome.)

**The divergence is shallow.** Both paths *fail* the formula —
they just disagree on which `ParseOutcome` variant represents
that failure:

- Tree-path: `Rejected(_)` — caught explicitly by the
  `infix_relation` pragma.
- ASF-path: `Empty` — every alternative got pruned by the same
  family of pragmas during glade traversal, leaving zero
  alternatives in the peak Vec.

**Crucially: the user-facing HTML output is bit-identical
across all three modes (ASF, HYBRID, LEGACY) on this
formula.** A `diff` of the rendered `.html` shows zero
differences. The audit is catching a divergence that has no
observable effect downstream.

### Task 2 — Post-hybrid Article-2025.tex timing

Already captured in the "Acceptance-Gate Measurements" section
above. Repeated here for completeness:

| Mode | Wall (avg of 2 runs) | vs LEGACY |
|---|---:|---:|
| `LATEXML_MARPA_LEGACY=1` | 12.21s | 1.00× |
| **`LATEXML_MARPA_HYBRID=1`** | **12.40s** | **1.015×** |
| ASF default | 17.00s | 1.39× |

Release profile, single-thread, cold-cache (no warm-up).
Within the ≤1.05× acceptance target.

### Task 3 — Broader ambiguity distribution

Four math-heavy fixtures, all under `LATEXML_MATH_AMBIGUITY_AUDIT=1`:

| Fixture | Total parses | Unamb | Amb | Unamb% |
|---|---:|---:|---:|---:|
| Article-2025.tex (algebraic topology) | 3902 | 3405 | 497 | **87.3%** |
| TheDiskComplex (geometric topology) | 681 | 527 | 154 | **77.4%** |
| arxiv 2602.06085 (mixed STEM) | 130 | 78 | 52 | **60.0%** |
| arxiv 2501.02222 (Toffoli gate) | 0 | 0 | 0 | n/a |

Range: 60–87% unambiguous. All well above the 50% sanity
floor; hybrid's value proposition is confirmed across paper
genres. The Toffoli paper produced zero math-parse calls —
likely because its math content was structured as display
environments that bypass `parse_marpa` (no $-delimited
formulae), or because it failed at an earlier pipeline stage;
not investigated further.

### Task 4 — Semantic mismatch search

Beyond the Task-1 minimal repro, I tried a representative set
of formula shapes under `LATEXML_MARPA_HYBRID_AUDIT_PARITY=1`:

- `$f(x)$`, `$\frac{a}{b}$`, `$\sqrt{x^2+y^2}$` — pass
- `$\sum_{i=1}^n a_i$`, `$\int_0^\infty f(x)\,dx$` — pass
- `$a + b$, $f(x)$, ... [10 formulas in one doc]` — pass
- `$P_A(P_m)^* = \{u \in (P_m)^* | Sq^k_*(u) = 0, \forall k
  \geq 1\}$` (the original failing case) — fails

The pattern: every failure I found reduces to the Task-1
minimal repro shape — set-builder with comma-separated
relation chain inside a single VERTBAR-fenced body. No other
semantic-divergence pattern surfaced. **No mismatch produced
a different user-visible HTML output** in any case tested.

### Task 5 — Ambiguous fixture regression (sin[XY])

Ran `latexml_oxide/tests/complex/physics.tex` (which contains
`\sin[2](x)`, `\sin[\grande]`, `\sin[x][\grande]`,
`\sin[x]{\grande}`, `\sin[x]\{\grande\}`) under all three modes:

| Mode | Output size | diff vs HYBRID |
|---|---:|---|
| ASF default | 69104 bytes | (none) |
| `LATEXML_MARPA_HYBRID=1` | 69104 bytes | reference |
| `LATEXML_MARPA_LEGACY=1` | 69104 bytes | (none) |

All three produce **bit-identical HTML output** on the
ambiguous sin[XY] family. Hybrid correctly routes these
formulas through the ASF branch (they're raw-ambiguous) and
preserves the expected disambiguation.

### Recommended fix for the audit

Codex's audit is **structurally correct** but **too strict on
failure-shape**. Two clean ways to fix the false-positive on
the minimal repro:

**Option A (recommended)**: relax the assertion to treat any
non-`Accepted` outcome as equivalent. Concretely:

```rust
let outcomes_compatible = match (&asf_outcome, tree_outcome) {
  (ParseOutcome::Accepted(a), ParseOutcome::Accepted(b)) => a == b,
  (ParseOutcome::Accepted(_), _) | (_, ParseOutcome::Accepted(_)) => false,
  // Both are some flavour of "no parse survived" — equivalent
  // from the user's perspective.
  _ => true,
};
assert!(outcomes_compatible, "...");
```

This narrows the audit to its actual question: "if both paths
accept, do they produce the same XM?" That's the
semantically-load-bearing parity check.

**Option B**: post-process the comparison to canonicalise
`ParseOutcome` failure variants into a single `Failed` bucket.
Similar effect to A but more invasive.

I do not recommend the xmkey canonicalisation route from my
earlier suggestion — it solves a problem that doesn't exist
given `XProps::eq` already skips ids.

### Acceptance-gate status (updated)

| Gate item | Status |
|---|---|
| 1301/0 test suite under `LATEXML_MARPA_HYBRID=1` | ✓ |
| Parity assertion is **deterministic** | ✓ (XProps::eq skips ids) |
| Parity assertion is **strictly safe** for default-on | **⚠ false-positive on Empty-vs-Rejected — fix per Option A** |
| `Article-2025.tex` wall ≤ 1.05× LEGACY | ✓ (1.015×) |
| Ambiguous fixture regression (sin[XY]) | ✓ (bit-identical HTML) |
| `LATEXML_MATH_AMBIGUITY_AUDIT` sanity (corpus-level) | ✓ (60–87% unambiguous across 4 papers) |
| User-facing HTML parity hybrid vs ASF/LEGACY | ✓ (bit-identical on every fixture tested) |

The single remaining blocker for flipping hybrid default-on
is the Option A audit relaxation. The audit currently raises
false alarms on shallow failure-shape drift.

## Decision Gate For Default-On Hybrid

Hybrid can become default-on only after:

- Canonicalized parity audit passes on representative stress cases.
- The audit passes on a representative raw-unambiguous sample.
- Known ambiguous fixtures still pass through the ASF branch.
- `Article-2025.tex` remains within 1.05x of legacy.
- A broader ambiguity audit still shows enough raw-unambiguous traffic
  to justify the extra branch complexity.

## Closing State (2026-05-17, hybrid default landed)

All five gate items above are now met. Hybrid is the default in
latexml-oxide (`9318960974`); `LATEXML_MARPA_LEGACY=1` and
`LATEXML_MARPA_ASF_ONLY=1` remain as opt-in escape hatches.

### Final 3-way wall on `Article-2025.tex` (bench profile)

| Mode | Wall (3-run avg) | vs LEGACY |
|---|---:|---:|
| **HYBRID default** | **12.45s** | **1.01×** |
| `LATEXML_MARPA_LEGACY=1` | 12.32s | 1.00× |
| `LATEXML_MARPA_ASF_ONLY=1` | 16.80s | 1.36× |

### Codex's optimization list — status

| # | Item | Status | Impact (Article-2025) |
|---|---|---|---|
| 1 | Hybrid routing | landed `9318960974` / marpa `60b320b` | ASF→HYBRID: 17.0s → 12.4s |
| 2 | Singleton fast path in `compute_symches` | landed by codex | embedded in hybrid baseline |
| 3 | Bocage metadata caches | landed `a045778` | flat (RefCell overhead ≈ FFI savings) |
| 4 | Flatten factoring storage | DEFERRED (codex senior-engineer caution) | n/a |
| 5 | Clean traversal internals | landed `a045778` + codex's generic recursion | flat; correctness/quality |
| 6 | Odometer Cartesian product | landed `109390fe92` | HYBRID: ~1.5% (only ambiguous-glade reduction) |

### Quality / correctness improvements beyond codex's list

- **`collect_factorings` propagates `Result`** (`96fd092`) — the
  prior `.unwrap_or(default)` silently mapped FFI errors to a
  bogus token-and-node default. Now `?`-propagates.
- **Audit relaxation as `parity_outcomes_compatible`** with 8
  unit tests (`0a8a171859`) — the audit's `Empty`-vs-`Rejected`
  false-positive on shallow pragma rejections is now guarded
  by a regression test.
- **`Glade.visited` field and `glade_is_visited` helper removed**
  — dead defensive code post-`VisitState` cycle protection.
- **`Rc<str>` Lexeme + `Rc<Vec<Option<XM>>>` ParseTree** kept
  even though direct perf was 0% — removes deep-clone hazards
  from the marpa cache hit/insert paths.

### Structural ASF_ONLY → LEGACY gap (residual ~4.5s)

After all Rust-side cleanup, ASF_ONLY remains ~37% slower than
LEGACY on `Article-2025.tex`. The residual gap is **structural**:
ASF builds a Rust-side glade representation (bocage walk +
per-or-node Nidset + Glade allocations) that Step-iteration
skips entirely. Singleton fast path eliminates the
`compute_symches` factoring chain for 87% of glades, but the
glade-bookkeeping fixed overhead persists.

Further wins would require either:
- libmarpa C-side surgery (out of scope — we don't own libmarpa)
- Restructuring ASF to skip the Nidset/Glade allocation when the
  forest is wholly unambiguous (large refactor; hybrid already
  achieves the same effect from the user perspective by skipping
  ASF entirely for that case)

Both yield diminishing returns vs the hybrid escape hatch.

### Tests at session close

- marpa: 23/0 (incl. 2 new hybrid tests)
- latexml-oxide: 1309/0 (1301 prior + 8 new parity-helper tests)
- Parity audit clean on Article-2025.tex (3902 parse calls) and
  TheDiskComplex.tex (681 parse calls)
- sin[XY] fixture (physics.tex): bit-identical HTML across all
  three modes
