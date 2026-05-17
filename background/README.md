# Background — Marpa reference materials

Reference docs for the ASF / evaluator port work. Not part of the
crate or build artifacts; this is a scratch dir for human reference.
Git-tracked so future sessions can find these without re-hunting them.

## Files

| File | What it covers | Use when |
|---|---|---|
| [`Kegler_2023_Marpa_Recognizer.pdf`](Kegler_2023_Marpa_Recognizer.pdf) | Kegler, *Marpa, a practical general parser: The Recognizer* (arXiv:1910.08129v3, Jan 2023). The formal recognizer paper. Earley + Leo + Aycock-Horspool integration, confluence/mainstem/tributary model, complexity proofs, generalized input model. | Reasoning about what's in the bocage (OR-nodes / AND-nodes), how Leo memoization affects derivation reconstruction, the meaning of the EIMT / EIM / dotted-rule machinery. |
| [`Kegler_2023_Marpa_2303.04093.pdf`](Kegler_2023_Marpa_2303.04093.pdf) | Kegler, *Marpa and Nullable Symbols* (arXiv:2303.04093v1, Mar 2023). The CHAF rewrite (Chomsky-Horspool-Aycock Form). | Understanding why the bocage talks about IRL (internal rule) IDs and the `xrl ↔ irl` mapping in `Grammar::source_xrl`. The ASF must expose XRL-level rules to users; CHAF "head / inner / tail" variants belong below the ASF API. |

## What's NOT in these files

The **evaluator / ASF** paper has not been published. The authoritative
reference for the ASF API is Marpa::R2's Perl source:

* [`Marpa::R2::ASF` docs](https://manpages.ubuntu.com/manpages/xenial/man3/Marpa::R2::ASF.3pm.html) — high-level interface.
* [`ASF.pod` on metacpan](https://metacpan.org/dist/Marpa-R2/view/pod/ASF.pod) — same content, may need crates.io account.
* `MARPA_R2/Marpa/R2/ASF.pm` — the implementation (~500 lines).
* `MARPA_R2/Marpa/R2/Choicepoint.pm` — factoring-stack helpers used by `ASF.pm`.

## Key concepts ported into our scaffolding (see `ASF_STATUS.md`)

* **OR-node** ↔ libmarpa `marpa_b_top_or_node` / `_marpa_b_or_node_*`. Multiple alternative reasons for a parse position. In our scaffolding this becomes a `Glade` (via `obtain_nidset`).
* **AND-node** ↔ libmarpa `_marpa_b_and_node_*`. One specific confluence (mainstem = predecessor OR-node, tributary = cause OR-node or token). In our scaffolding these populate the `or_nodes: Vec<Nidset>` during `ASF::new`.
* **Symch** (symbol choice) — Perl-level grouping of AND-nodes by their rule's LHS / token symbol. The inner `compute_symches` loop in `asf.rs` builds these; the body is currently commented Perl source (Step 2 in the audit).
* **Factoring** — one specific way to expand a symch's RHS. Comes from walking the AND-node predecessor chain. Not yet implemented in the Rust port.
