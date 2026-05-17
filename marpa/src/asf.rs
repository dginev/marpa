mod glade;
mod nidset;

use std::collections::HashMap;

use crate::result::Result;
use crate::thin::{Bocage, Order, Recognizer};

pub use self::glade::*;
pub use self::nidset::*;

/// Write `val` into the sparse `cache` at `idx`, growing the Vec with
/// `None` placeholders as needed. The cache is allocated once per
/// `traverse(...)` call and is sparse-by-design: a glade id ≤ max
/// visited so far that wasn't reached on this traversal branch stays
/// `None`.
fn cache_set<T>(cache: &mut Vec<Option<T>>, idx: usize, val: T) {
  if cache.len() <= idx {
    cache.resize_with(idx + 1, || None);
  }
  cache[idx] = Some(val);
}

/// User callback for ASF traversal.
///
/// The driver walks the bocage in post-order: child glades are
/// evaluated first and their `ParseTree` outputs are memoized; then
/// the parent glade's `traverse_glade` is called with the children's
/// outputs already available in `children`. This makes the cost
/// **O(glades)** rather than **O(trees)** — the central reason this
/// crate exists.
///
/// The user iterates `(symch, factoring)` combinations inside the
/// callback by calling `glade.next()`; for each combination,
/// `glade.rh_length()` and `glade.rh_glade_id(ix)` describe the RHS
/// positions, and the corresponding child outputs are looked up in
/// `children` by glade id.
///
/// `state` is a shared mutable scratchpad threaded through the whole
/// traversal (e.g. for accumulators, deterministic counters, error
/// flags). Mirrors Perl `Marpa::R2::ASF`'s `$scratch_ref`.
pub trait Traverser {
  type ParseTree;
  type ParseState;

  /// `children` is a sparse slice indexed by glade id; a `None`
  /// entry means that glade hasn't been visited on this branch of
  /// the traversal (cycle-cousin scenarios). For a well-formed
  /// acyclic bocage, every `glade.rh_glade_id(ix)` lookup resolves
  /// to `Some(&PT)` by post-order invariant.
  fn traverse_glade(
    &mut self,
    glade: &mut Glade,
    children: &[Option<Self::ParseTree>],
    state: &mut Self::ParseState,
  ) -> Result<Self::ParseTree>;
}

// `Powerset` and `or_nodes` are scaffolding for the in-progress
// ASF traversal port; keep the fields populated so the eventual
// glade-traversal port can read them, but tell rustc not to warn yet.
#[allow(dead_code)]
pub struct ASF {
  next_inset_id: usize,
  factoring_max: usize,
  nidset_by_id: HashMap<usize, Nidset>,
  glades: HashMap<usize, Glade>,
  intset_by_key: HashMap<Vec<i32>, usize>,
  or_nodes: Vec<Nidset>,
  recce: Recognizer,
  bocage: Bocage,
  ordering: Order,
}

impl ASF {
  fn intset_id(&mut self, mut ids: Vec<i32>) -> (usize, Vec<i32>) {
    ids.sort();
    let intset_id = self.intset_by_key.entry(ids.clone()).or_insert(self.next_inset_id + 1);
    if *intset_id > self.next_inset_id {
      self.next_inset_id += 1;
    }
    (*intset_id, ids)
  }

  fn obtain_nidset_id(&mut self, nids: Vec<i32>) -> usize {
    let (id, nids) = self.intset_id(nids);
    self.nidset_by_id.entry(id).or_insert_with(|| Nidset { id, nids });
    id
  }

  /// Make sure a `Glade` exists in `self.glades` for `glade_id` and
  /// mark it registered. Computation of its symches is deferred until
  /// `obtain_glade(glade_id)` is called (lazy, matches Perl).
  fn register_glade(&mut self, glade_id: usize) {
    let glade = self.glades.entry(glade_id).or_default();
    glade.registered = true;
  }

  pub fn new(recce: Recognizer) -> Result<Self> {
    // Initialize all usual thin:: structs here, we'll need them
    let bocage = Bocage::new(&recce)?;
    let mut ordering = bocage.get_ordering().expect(
      "An attempt was make to create an ASF for a null parse\n
          A null parse is a successful parse of a zero-length string\n
          ASF's are not defined for null parses",
    );
    let mut or_nodes = Vec::new();
    let mut or_node_id = 0;
    loop {
      let and_node_ids = ordering.or_node_and_node_ids(or_node_id);
      if and_node_ids.is_empty() {
        break;
      }
      or_nodes.insert(
        or_node_id,
        Nidset {
          id: or_node_id,
          nids: and_node_ids,
        },
      );
      or_node_id += 1;
    }

    Ok(ASF {
      next_inset_id: 0,
      nidset_by_id: HashMap::new(),
      glades: HashMap::new(),
      intset_by_key: HashMap::new(),
      factoring_max: 42,
      or_nodes,
      recce,
      bocage,
      ordering,
    })
  }

  /// Run a `Traverser` over the parse forest in post-order with
  /// per-glade memoization. The user callback is invoked exactly
  /// once per reachable glade; child outputs are already in
  /// `children` by the time the parent fires.
  ///
  /// Generic over the concrete traverser type (no `Box<dyn>` so the
  /// traverser may borrow externally — important for the latexml-
  /// oxide math parser, whose traverser needs `&mut Document` and
  /// `&Actions`).
  ///
  /// Returns `(peak_output, final_state)` once the peak glade has
  /// been evaluated.
  pub fn traverse<TR>(&mut self, mut init_state: TR::ParseState, traverser: &mut TR) -> Result<(TR::ParseTree, TR::ParseState)>
  where
    TR: Traverser,
    TR::ParseTree: Clone,
  {
    let peak = self.peak()?;
    // Glade IDs are assigned sequentially from 0 by `register_glade`
    // / `obtain_nidset_id`, so a `Vec<Option<PT>>` indexed by id
    // replaces the prior `HashMap<usize, PT>` cache with O(1)
    // array-index loads and no hash probes.
    let mut cache: Vec<Option<TR::ParseTree>> = Vec::new();
    let output = self.traverse_glade_recursive(peak, &mut cache, traverser, &mut init_state)?;
    Ok((output, init_state))
  }

  /// Post-order recursive driver. Visits each child glade once
  /// (memoized in `cache`); cycle-safe via the `visited` flag.
  fn traverse_glade_recursive<PT, PS>(
    &mut self,
    glade_id: usize,
    cache: &mut Vec<Option<PT>>,
    traverser: &mut dyn Traverser<ParseTree = PT, ParseState = PS>,
    state: &mut PS,
  ) -> Result<PT>
  where
    PT: Clone,
  {
    if let Some(Some(cached)) = cache.get(glade_id) {
      return Ok(cached.clone());
    }

    // Ensure the glade's symches are populated, then enumerate the
    // distinct child glade ids reachable from any (symch, factoring,
    // RHS position). We grab them up-front so the recursion doesn't
    // hold a borrow into `self.glades`.
    self.obtain_glade(glade_id)?;
    let child_ids: Vec<usize> = {
      let glade = self.glades.get(&glade_id).unwrap();
      let mut seen: Vec<usize> = Vec::new();
      for symch in &glade.symches {
        for factoring in &symch.factorings {
          for &cid in factoring {
            // Skip the self-referential factoring of a token glade.
            if cid == glade_id {
              continue;
            }
            if !seen.contains(&cid) {
              seen.push(cid);
            }
          }
        }
      }
      seen
    };

    // Mark the parent visited up-front so a cycle (cousin pointing
    // back through us) doesn't recurse infinitely. Honest acyclic
    // bocages won't hit this, but defensive.
    if let Some(g) = self.glades.get_mut(&glade_id) {
      g.visited = true;
    }

    // Recurse into each child (post-order).
    for child_id in child_ids {
      if matches!(cache.get(child_id), Some(Some(_))) {
        continue;
      }
      let child_output = self.traverse_glade_recursive(child_id, cache, traverser, state)?;
      cache_set(cache, child_id, child_output);
    }

    // Now the parent's children are all in `cache`. Hand the parent
    // glade to the user callback. Rewind the cursor so the user can
    // iterate (symch, factoring) from the start.
    let glade = self
      .glades
      .get_mut(&glade_id)
      .expect("glade entry must exist after obtain_glade");
    glade.rewind();
    let output = traverser.traverse_glade(glade, cache.as_slice(), state)?;
    cache_set(cache, glade_id, output.clone());
    Ok(output)
  }

  /// Cheap pre-flight check: 1 = unambiguous, 2 = ambiguous.
  /// Mirrors `Bocage::ambiguity_metric` without forcing symch
  /// computation. Returns the libmarpa sentinel.
  pub fn ambiguity_metric(&self) -> Result<i32> {
    self.bocage.ambiguity_metric()
  }

  fn peak(&mut self) -> Result<usize> {
    let augment_or_node_id = self.bocage.top_or_node()?;
    // The augment or-node's and-nodes correspond to **distinct
    // top-level parses** — for an ambiguous grammar (the
    // latexml-oxide math parser is the motivating example), this
    // is the only place where top-level alternatives are
    // exposed by libmarpa. Aggregate **all** of their causes into
    // the peak glade's nidset, so `compute_symches` groups them by
    // their underlying XRL and the user's `Traverser` sees every
    // alternative top-rule reduction.
    //
    // Perl `Marpa::R2::ASF::peak` only takes `[0]`; on the panda
    // grammar (single start rule) it doesn't matter, but the math
    // parser's top-level rule has multiple alternatives that the
    // single-pick model loses.
    let augment_and_node_ids = self.or_nodes[augment_or_node_id as usize].nids.clone();
    let mut cause_nids: Vec<i32> = Vec::with_capacity(augment_and_node_ids.len());
    for and_id in augment_and_node_ids {
      let cause = self.bocage.and_node_cause(and_id)?;
      if !cause_nids.contains(&cause) {
        cause_nids.push(cause);
      }
    }
    let glade_id = self.obtain_nidset_id(cause_nids);
    self.register_glade(glade_id);
    self.obtain_glade(glade_id)?;
    Ok(glade_id)
  }

  fn obtain_glade(&mut self, glade_id: usize) -> Result<&mut Glade> {
    let glade = self.glades.get(&glade_id).expect("Attempt to use an invalid glade");
    if !glade.registered {
      panic!("attempt to use an unregistered glade with ID: {glade_id}");
    }
    // Return the glade if it is already set up
    if !glade.symches.is_empty() {
      Ok(self.glades.get_mut(&glade_id).unwrap())
    } else {
      self.compute_symches(glade_id)
    }
  }

  fn compute_symches(&mut self, glade_id: usize) -> Result<&mut Glade> {
    // --- Phase 1: gather source nids and sort by sort_ix (== XRL or
    // token-id, depending on the nid sign). Same as the original
    // scaffolding, but cleaned up.
    let source_nids: Vec<i32> = self
      .nidset_by_id
      .get(&glade_id)
      .unwrap_or_else(|| panic!("No nidset registered for glade ID {glade_id}"))
      .nids
      .clone();

    let mut source_data: Vec<(i32, i32)> = Vec::with_capacity(source_nids.len());
    for nid in &source_nids {
      let sort_ix = self.nid_sort_ix(*nid)?;
      source_data.push((sort_ix, *nid));
    }
    source_data.sort_by_key(|k| k.0);

    // --- Phase 2: for each contiguous run of source_data sharing
    // the same sort_ix, build one symch. Within a symch, every
    // source nid contributes its full set of factorings.
    let factoring_max = self.factoring_max;
    let mut symches: Vec<Symch> = Vec::new();
    let mut group_start = 0usize;
    let is_token_glade = source_data.first().map(|(_, n)| *n < 0).unwrap_or(false);

    while group_start < source_data.len() {
      let current_sort_ix = source_data[group_start].0;
      let mut group_end = group_start + 1;
      while group_end < source_data.len() && source_data[group_end].0 == current_sort_ix {
        group_end += 1;
      }
      let group_nids: Vec<i32> = source_data[group_start..group_end].iter().map(|(_, n)| *n).collect();
      group_start = group_end;

      let first_nid = group_nids[0];
      let rule_id = self.nid_rule_id(first_nid)?;

      if rule_id < 0 {
        // ---- Token symch ----
        // Mirrors Perl ASF.pm: `push @factorings, [$glade_id]; push
        // @symches, \@factorings;` — a self-referential factoring
        // sentinel meaning "this glade IS a token leaf".
        // The token's own glade-id is the singleton nidset wrapping
        // the same negative nid we're already standing on.
        let token_glade_id = self.obtain_nidset_id(vec![first_nid]);
        self.register_glade(token_glade_id);
        symches.push(Symch {
          rule_id: -1,
          factorings: vec![vec![token_glade_id]],
          omitted: false,
        });
        continue;
      }

      // ---- Rule symch ----
      // Each factoring is a sequence of RHS positions, left-to-right.
      // Each RHS position is a SET of child-nids (possibly multiple
      // ids unified into one glade — Perl `glade_id_factors` does this
      // grouping for and-nodes that share a predecessor; here we do
      // the equivalent by grouping contiguous same-predecessor
      // and-nodes within each or-node we visit).
      let mut raw_factorings: Vec<Vec<Vec<i32>>> = Vec::new();
      let mut omitted = false;
      for &nid in &group_nids {
        if raw_factorings.len() >= factoring_max {
          omitted = true;
          break;
        }
        let mut work_stack: Vec<Vec<i32>> = Vec::new();
        self.collect_factorings(nid, &mut work_stack, &mut raw_factorings, factoring_max, &mut omitted);
        if omitted {
          break;
        }
      }

      // Translate each per-position cause-nid set into a registered
      // child glade id (a multi-nid nidset becomes a multi-source
      // glade — this is where parallel alternatives unify).
      let mut factorings: Vec<Vec<usize>> = Vec::with_capacity(raw_factorings.len());
      for position_sets in raw_factorings {
        let mut child_glade_ids: Vec<usize> = Vec::with_capacity(position_sets.len());
        for cause_nids in position_sets {
          let child_glade_id = self.obtain_nidset_id(cause_nids);
          self.register_glade(child_glade_id);
          child_glade_ids.push(child_glade_id);
        }
        factorings.push(child_glade_ids);
      }

      symches.push(Symch {
        rule_id,
        factorings,
        omitted,
      });
    }

    // --- Phase 3: precompute symbol_id and write back. The symbol_id
    // is derived from the first source nid; all source nids in a
    // glade share the same LHS symbol (or the same token id).
    let symbol_id = self.nid_symbol_id(source_data[0].1)?;

    let glade = self.glades.get_mut(&glade_id).expect("Attempt to use an invalid glade");
    glade.symches = symches;
    glade.id = glade_id;
    glade.symbol_id = symbol_id;
    glade.is_token = is_token_glade;
    glade.cursor = (0, 0);

    Ok(glade)
  }

  /// DFS over the predecessor chain of `or_node_id` collecting full
  /// factorings as a list of RHS positions. Each position is a
  /// **set** of cause-nids (possibly multi-element when the bocage
  /// has parallel alternatives sharing the same predecessor — those
  /// get unified into a single multi-source child glade, mirroring
  /// Perl `glade_id_factors`).
  ///
  /// `work_stack` accumulates positions rightmost-first; on each
  /// leaf (no predecessor) the stack is reversed and pushed into
  /// `out`. `omitted` short-circuits once `factoring_max` is hit.
  fn collect_factorings(
    &self,
    or_node_id: i32,
    work_stack: &mut Vec<Vec<i32>>,
    out: &mut Vec<Vec<Vec<i32>>>,
    factoring_max: usize,
    omitted: &mut bool,
  ) {
    if *omitted || out.len() >= factoring_max {
      *omitted = true;
      return;
    }
    let and_node_ids: Vec<i32> = match self.or_nodes.get(or_node_id as usize) {
      Some(ns) => ns.nids.clone(),
      None => return,
    };

    // Group contiguous same-predecessor and-nodes. Each group
    // becomes one RHS-position step (with the cause-nids unified
    // into a multi-nid set). Different groups become different
    // factorings via the predecessor recursion.
    //
    // Mirrors Perl `set_last_choice`: extend the range while
    // predecessors match; when they differ, start a new group.
    let mut groups: Vec<(Option<i32>, Vec<i32>)> = Vec::new();
    for &and_id in &and_node_ids {
      let cause = self.bocage.and_node_cause(and_id).unwrap_or(-1);
      let pred_raw = self.bocage.and_node_predecessor(and_id);
      let pred: Option<i32> = match pred_raw {
        Some(p) if p >= 0 => Some(p),
        _ => None,
      };
      let cause_nid: i32 = if cause < 0 {
        // Token and-node: encode the and-node as a negative nid.
        and_node_to_nid(and_id)
      } else {
        // Rule and-node: cause is the child or-node id.
        cause
      };
      // Extend the previous group iff the predecessor matches;
      // otherwise start a new group.
      if let Some(last) = groups.last_mut()
        && last.0 == pred
      {
        if !last.1.contains(&cause_nid) {
          last.1.push(cause_nid);
        }
        continue;
      }
      groups.push((pred, vec![cause_nid]));
    }

    for (pred, cause_nids) in groups {
      if out.len() >= factoring_max {
        *omitted = true;
        return;
      }
      work_stack.push(cause_nids);
      match pred {
        None => {
          let mut factoring = work_stack.clone();
          factoring.reverse();
          out.push(factoring);
        }
        Some(pred_or) => {
          self.collect_factorings(pred_or, work_stack, out, factoring_max, omitted);
        }
      }
      work_stack.pop();
      if *omitted {
        return;
      }
    }
  }

  #[allow(dead_code)]
  fn glade_is_visited(&self, glade_id: usize) -> bool {
    match self.glades.get(&glade_id) {
      None => false,
      Some(glade) => glade.visited,
    }
  }

  fn nid_sort_ix(&self, nid: i32) -> Result<i32> {
    let grammar = self.recce.grammar();
    let bocage = &self.bocage;
    if nid >= 0 {
      let irl_id = bocage.or_node_irl(nid)?;
      return grammar.source_xrl(irl_id);
    }
    let and_node_id = nid_to_and_node(nid);
    let token_nsy_id = bocage.and_node_symbol(and_node_id)?;
    let token_id = grammar.source_xsy(token_nsy_id)?;

    // -2 is reserved for 'end of data'
    Ok(-token_id - 3)
  }

  /// External rule id for a (positive) rule nid, or `-1` for a
  /// token nid. Mirrors Perl `nid_rule_id`.
  fn nid_rule_id(&self, nid: i32) -> Result<i32> {
    if nid < 0 {
      return Ok(-1);
    }
    let irl_id = self.bocage.or_node_irl(nid)?;
    self.recce.grammar().source_xrl(irl_id)
  }

  pub(crate) fn nid_token_id(&self, nid: i32) -> Result<Option<i32>> {
    if nid > NID_LEAF_BASE {
      return Ok(None);
    }
    let and_node_id = nid_to_and_node(nid);
    let grammar_c = &self.recce.grammar;
    let bocage = &self.bocage;
    let token_nsy_id = bocage.and_node_symbol(and_node_id)?;
    let token_id = grammar_c.source_xsy(token_nsy_id)?;
    Ok(Some(token_id))
  }

  pub(crate) fn nid_symbol_id(&self, nid: i32) -> Result<i32> {
    if let Some(token_id) = self.nid_token_id(nid)? {
      return Ok(token_id);
    } else if nid < 0 {
      return Err(format!("No symbol ID for node ID: {nid}").into());
    }

    // Not a token, so return the LHS of the rule
    let grammar_c = &self.recce.grammar;
    let bocage = &self.bocage;
    let irl_id = bocage.or_node_irl(nid)?;
    let xrl_id = grammar_c.source_xrl(irl_id)?;
    let lhs_id = grammar_c.rule_lhs(xrl_id)?;
    Ok(lhs_id)
  }
}

const NID_LEAF_BASE: i32 = -43;
/// Range from -1 to -42 reserved for special values
fn and_node_to_nid(offset: i32) -> i32 {
  NID_LEAF_BASE - offset
}
/// Range from -1 to -42 reserved for special values
fn nid_to_and_node(offset: i32) -> i32 {
  NID_LEAF_BASE - offset
}
