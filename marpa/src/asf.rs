mod glade;
mod nidset;

use std::collections::HashMap;

use crate::result::Result;
use crate::thin::{Bocage, Order, Recognizer};

pub use self::glade::*;
pub use self::nidset::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum VisitState {
  Unseen,
  Visiting,
  Done,
}

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

/// Per-and-node bocage metadata. Cached so the inner factoring
/// loops don't re-cross FFI for the same bocage node. Populated
/// lazily on first access via `and_node_info(id)`.
#[derive(Debug, Clone, Copy)]
struct AndNodeInfo {
  /// Cause or-node id, or a negative sentinel for token and-nodes.
  cause: i32,
  /// Predecessor or-node id, or `None` at the chain root.
  predecessor: Option<i32>,
  /// Token symbol id (only meaningful when `cause < 0`). Lazy: we
  /// only fill this when `nid_sort_ix` / `nid_token_id` actually
  /// asks for it, since rule and-nodes never use it.
  symbol: Option<i32>,
}

/// Per-or-node bocage metadata. The `(irl_id → xrl_id → lhs_id)`
/// chain crosses FFI three times and the values are stable for
/// the lifetime of the bocage. Cache them per-or-node.
///
/// `irl_id` is computed alongside `xrl_id` and `lhs_id` (the
/// libmarpa traversal goes irl → xrl → lhs) but no current
/// consumer reads it; kept on the struct to document the
/// derivation chain and to avoid re-deriving if a future caller
/// needs it.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
struct OrNodeInfo {
  irl_id: i32,
  xrl_id: i32,
  lhs_id: i32,
}

// `Powerset` and `or_nodes` are scaffolding for the in-progress
// ASF traversal port; keep the fields populated so the eventual
// glade-traversal port can read them, but tell rustc not to warn yet.
#[allow(dead_code)]
pub struct ASF {
  next_inset_id: usize,
  factoring_max: usize,
  /// Indexed by glade id (sequential, dense). `None` slots are
  /// holes — register_glade/get widen the Vec as needed.
  nidset_by_id: Vec<Option<Nidset>>,
  /// Same indexing convention as `nidset_by_id`.
  glades: Vec<Option<Glade>>,
  intset_by_key: HashMap<Vec<i32>, usize>,
  or_nodes: Vec<Nidset>,
  /// Lazy per-and-node FFI metadata cache. Sized to grow on first
  /// query; entries are stable once filled.
  and_node_cache: std::cell::RefCell<Vec<Option<AndNodeInfo>>>,
  /// Lazy per-or-node FFI metadata cache. Same growth semantics
  /// as `and_node_cache`.
  or_node_cache: std::cell::RefCell<Vec<Option<OrNodeInfo>>>,
  recce: Recognizer,
  bocage: Bocage,
  ordering: Order,
}

/// Get a mutable ref to the slot at `idx`, growing as needed and
/// initializing with `Default::default()`. Mirrors the prior
/// `HashMap::entry(idx).or_default()` pattern.
fn vec_slot_or_default<T: Default>(slot: &mut Vec<Option<T>>, idx: usize) -> &mut T {
  if slot.len() <= idx {
    slot.resize_with(idx + 1, || None);
  }
  if slot[idx].is_none() {
    slot[idx] = Some(T::default());
  }
  slot[idx].as_mut().unwrap()
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
    if self.nidset_by_id.len() <= id {
      self.nidset_by_id.resize_with(id + 1, || None);
    }
    if self.nidset_by_id[id].is_none() {
      self.nidset_by_id[id] = Some(Nidset { id, nids });
    }
    id
  }

  /// Make sure a `Glade` exists in `self.glades` for `glade_id` and
  /// mark it registered. Computation of its symches is deferred until
  /// `obtain_glade(glade_id)` is called (lazy, matches Perl).
  fn register_glade(&mut self, glade_id: usize) {
    let glade = vec_slot_or_default(&mut self.glades, glade_id);
    glade.registered = true;
  }

  pub fn new(recce: Recognizer) -> Result<Self> {
    let bocage = Bocage::new(&recce)?;
    Self::from_parts(recce, bocage)
  }

  /// Build an ASF from an already-created recognizer/bocage pair.
  ///
  /// This is useful for one-pass hybrid callers that need to inspect
  /// `Bocage::ambiguity_metric()` before deciding whether to traverse
  /// the ASF. Passing the bocage through avoids constructing it twice.
  pub fn from_parts(recce: Recognizer, bocage: Bocage) -> Result<Self> {
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

    let or_node_count = or_nodes.len();
    Ok(ASF {
      next_inset_id: 0,
      nidset_by_id: Vec::new(),
      glades: Vec::new(),
      intset_by_key: HashMap::new(),
      factoring_max: 42,
      or_nodes,
      // OR-node id range is bounded by `or_nodes.len()` (we enumerated
      // them upfront). Pre-size to avoid growth in the hot path.
      // AND-node ids are unbounded from our side; grow on demand.
      and_node_cache: std::cell::RefCell::new(Vec::new()),
      or_node_cache: std::cell::RefCell::new(vec![None; or_node_count]),
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
    let mut visit_state: Vec<VisitState> = Vec::new();
    let output =
      self.traverse_glade_recursive(peak, &mut cache, &mut visit_state, traverser, &mut init_state)?;
    Ok((output, init_state))
  }

  /// Post-order recursive driver. Visits each child glade once
  /// (memoized in `cache`); cycle-safe via `visit_state`.
  fn traverse_glade_recursive<TR>(
    &mut self,
    glade_id: usize,
    cache: &mut Vec<Option<TR::ParseTree>>,
    visit_state: &mut Vec<VisitState>,
    traverser: &mut TR,
    state: &mut TR::ParseState,
  ) -> Result<TR::ParseTree>
  where
    TR: Traverser,
    TR::ParseTree: Clone,
  {
    if let Some(Some(cached)) = cache.get(glade_id) {
      return Ok(cached.clone());
    }
    if visit_state.len() <= glade_id {
      visit_state.resize(glade_id + 1, VisitState::Unseen);
    }
    match visit_state[glade_id] {
      VisitState::Unseen => {},
      VisitState::Visiting => {
        return Err(format!("cycle detected while traversing ASF glade {glade_id}").into());
      },
      VisitState::Done => {
        return Err(format!("ASF glade {glade_id} marked done without cached output").into());
      },
    }
    visit_state[glade_id] = VisitState::Visiting;

    // Ensure the glade's symches are populated, then enumerate the
    // distinct child glade ids reachable from any (symch, factoring,
    // RHS position). We grab them up-front so the recursion doesn't
    // hold a borrow into `self.glades`.
    self.obtain_glade(glade_id)?;
    let child_ids: Vec<usize> = {
      let glade = self.glades.get(glade_id).and_then(|o| o.as_ref()).unwrap();
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

    // Recurse into each child (post-order). Cycle protection is
    // handled at function entry via `visit_state[glade_id]` —
    // setting it to `Visiting` above already shields recursion
    // from cousin-pointer cycles.

    // Recurse into each child (post-order).
    for child_id in child_ids {
      if matches!(cache.get(child_id), Some(Some(_))) {
        continue;
      }
      self.traverse_glade_recursive(child_id, cache, visit_state, traverser, state)?;
    }

    // Now the parent's children are all in `cache`. Hand the parent
    // glade to the user callback. Rewind the cursor so the user can
    // iterate (symch, factoring) from the start.
    let glade = self
      .glades
      .get_mut(glade_id)
      .and_then(|o| o.as_mut())
      .expect("glade entry must exist after obtain_glade");
    glade.rewind();
    let output = traverser.traverse_glade(glade, cache.as_slice(), state)?;
    cache_set(cache, glade_id, output.clone());
    visit_state[glade_id] = VisitState::Done;
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
      let cause = self.and_node_info(and_id)?.cause;
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
    let glade = self.glades.get(glade_id).and_then(|o| o.as_ref())
      .expect("Attempt to use an invalid glade");
    if !glade.registered {
      panic!("attempt to use an unregistered glade with ID: {glade_id}");
    }
    // Return the glade if it is already set up
    if !glade.symches.is_empty() {
      Ok(self.glades.get_mut(glade_id).and_then(|o| o.as_mut()).unwrap())
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
      .get(glade_id)
      .and_then(|o| o.as_ref())
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
      let mut omitted = false;
      let factorings = if let Some(singleton_factoring) = self.try_singleton_factoring(&group_nids)? {
        vec![singleton_factoring]
      } else {
        let mut raw_factorings: Vec<Vec<Vec<i32>>> = Vec::new();
        for &nid in &group_nids {
          if raw_factorings.len() >= factoring_max {
            omitted = true;
            break;
          }
          let mut work_stack: Vec<Vec<i32>> = Vec::new();
          self.collect_factorings(nid, &mut work_stack, &mut raw_factorings, factoring_max, &mut omitted)?;
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
        factorings
      };

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

    let glade = self.glades.get_mut(glade_id).and_then(|o| o.as_mut())
      .expect("Attempt to use an invalid glade");
    glade.symches = symches;
    glade.id = glade_id;
    glade.symbol_id = symbol_id;
    glade.is_token = is_token_glade;
    glade.cursor = (0, 0);

    Ok(glade)
  }

  /// Fast path for the common unbranched predecessor-chain shape:
  /// exactly one source nid for the symch and exactly one and-node at
  /// every OR node along the chain. This emits the single factoring
  /// directly and avoids the general grouping/work-stack machinery.
  fn try_singleton_factoring(&mut self, group_nids: &[i32]) -> Result<Option<Vec<usize>>> {
    if group_nids.len() != 1 {
      return Ok(None);
    }

    let mut or_node_id = group_nids[0];
    let mut child_nids: Vec<i32> = Vec::new();

    loop {
      let and_node_id = match self.or_nodes.get(or_node_id as usize) {
        Some(ns) if ns.nids.len() == 1 => ns.nids[0],
        _ => return Ok(None),
      };

      let info = self.and_node_info(and_node_id)?;
      let cause_nid = if info.cause < 0 {
        and_node_to_nid(and_node_id)
      } else {
        info.cause
      };
      child_nids.push(cause_nid);

      match info.predecessor {
        Some(pred) => or_node_id = pred,
        None => {
          child_nids.reverse();
          let mut child_glade_ids = Vec::with_capacity(child_nids.len());
          for cause_nid in child_nids {
            let child_glade_id = self.obtain_nidset_id(vec![cause_nid]);
            self.register_glade(child_glade_id);
            child_glade_ids.push(child_glade_id);
          }
          return Ok(Some(child_glade_ids));
        },
      }
    }
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
  ) -> Result<()> {
    if *omitted || out.len() >= factoring_max {
      *omitted = true;
      return Ok(());
    }

    // Group contiguous same-predecessor and-nodes. Each group
    // becomes one RHS-position step (with the cause-nids unified
    // into a multi-nid set). Different groups become different
    // factorings via the predecessor recursion.
    //
    // Mirrors Perl `set_last_choice`: extend the range while
    // predecessors match; when they differ, start a new group.
    //
    // Hold a borrow into `self.or_nodes` for the iteration only;
    // the borrow is released before the recursion in `groups`
    // below. `self.and_node_info(_)` uses RefCell internally so
    // it composes with the shared `self.or_nodes` borrow.
    let mut groups: Vec<(Option<i32>, Vec<i32>)> = Vec::new();
    {
      let and_node_ids: &[i32] = match self.or_nodes.get(or_node_id as usize) {
        Some(ns) => ns.nids.as_slice(),
        None => return Ok(()),
      };
      for &and_id in and_node_ids {
        // Use the cached metadata for cause/predecessor — these
        // are the inner FFI calls collect_factorings is built
        // around, so caching them turns the per-and-node cost
        // into a Vec index. A genuine FFI error here is
        // propagated as the function's `Result` rather than
        // silently mapped to a token-and-node default; the prior
        // `.unwrap_or(default)` masked real bugs.
        let info = self.and_node_info(and_id)?;
        let pred: Option<i32> = info.predecessor;
        let cause_nid: i32 = if info.cause < 0 {
          // Token and-node: encode the and-node as a negative nid.
          and_node_to_nid(and_id)
        } else {
          // Rule and-node: cause is the child or-node id.
          info.cause
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
    }

    for (pred, cause_nids) in groups {
      if out.len() >= factoring_max {
        *omitted = true;
        return Ok(());
      }
      work_stack.push(cause_nids);
      match pred {
        None => {
          let mut factoring = work_stack.clone();
          factoring.reverse();
          out.push(factoring);
        }
        Some(pred_or) => {
          self.collect_factorings(pred_or, work_stack, out, factoring_max, omitted)?;
        }
      }
      work_stack.pop();
      if *omitted {
        return Ok(());
      }
    }
    Ok(())
  }

  // ---- Bocage metadata caches ----
  //
  // FFI calls to libmarpa for `and_node_cause/predecessor/symbol` and
  // `or_node_irl/source_xrl/rule_lhs` were repeated per-glade in
  // `collect_factorings`, `try_singleton_factoring`, `nid_sort_ix`,
  // `nid_rule_id`, `nid_token_id`, and `nid_symbol_id`. The bocage
  // is immutable for the lifetime of the ASF, so each `(id, field)`
  // pair has a fixed answer. Cache them on first read.
  //
  // We use `RefCell<Vec<Option<_>>>` so the caches are populated from
  // `&self` accessors (the hot paths that read this metadata only
  // need `&self`). Single-threaded by design — `Recognizer` /
  // `Bocage` aren't `Send` anyway.

  /// Read (or populate then read) the cached AndNodeInfo for an
  /// and-node id. The bocage may not have a `symbol` field for rule
  /// and-nodes, so `cause < 0 ⟹ symbol = Some(...)` is the
  /// invariant; for `cause >= 0` (rule and-nodes) we leave it None
  /// and the caller falls back to or-node metadata.
  fn and_node_info(&self, and_node_id: i32) -> Result<AndNodeInfo> {
    let id = and_node_id as usize;
    if let Some(Some(info)) = self.and_node_cache.borrow().get(id).copied() {
      return Ok(info);
    }
    let cause = self.bocage.and_node_cause(and_node_id)?;
    let pred_raw = self.bocage.and_node_predecessor(and_node_id);
    let predecessor = match pred_raw {
      Some(p) if p >= 0 => Some(p),
      _ => None,
    };
    let symbol = if cause < 0 {
      Some(self.bocage.and_node_symbol(and_node_id)?)
    } else {
      None
    };
    let info = AndNodeInfo { cause, predecessor, symbol };
    let mut cache = self.and_node_cache.borrow_mut();
    if cache.len() <= id {
      cache.resize(id + 1, None);
    }
    cache[id] = Some(info);
    Ok(info)
  }

  /// Read (or populate then read) the cached OrNodeInfo for an
  /// or-node id. Crosses three FFI calls on a cache miss: irl_id,
  /// xrl_id, lhs_id.
  fn or_node_info(&self, or_node_id: i32) -> Result<OrNodeInfo> {
    let id = or_node_id as usize;
    if let Some(Some(info)) = self.or_node_cache.borrow().get(id).copied() {
      return Ok(info);
    }
    let irl_id = self.bocage.or_node_irl(or_node_id)?;
    let grammar = self.recce.grammar();
    let xrl_id = grammar.source_xrl(irl_id)?;
    let lhs_id = grammar.rule_lhs(xrl_id)?;
    let info = OrNodeInfo { irl_id, xrl_id, lhs_id };
    let mut cache = self.or_node_cache.borrow_mut();
    if cache.len() <= id {
      cache.resize(id + 1, None);
    }
    cache[id] = Some(info);
    Ok(info)
  }

  fn nid_sort_ix(&self, nid: i32) -> Result<i32> {
    if nid >= 0 {
      // Rule nid → external rule id, served from the OrNode cache.
      return Ok(self.or_node_info(nid)?.xrl_id);
    }
    let and_node_id = nid_to_and_node(nid);
    let token_nsy_id = match self.and_node_info(and_node_id)?.symbol {
      Some(s) => s,
      None => self.bocage.and_node_symbol(and_node_id)?,
    };
    let token_id = self.recce.grammar().source_xsy(token_nsy_id)?;
    // -2 is reserved for 'end of data'
    Ok(-token_id - 3)
  }

  /// External rule id for a (positive) rule nid, or `-1` for a
  /// token nid. Mirrors Perl `nid_rule_id`.
  fn nid_rule_id(&self, nid: i32) -> Result<i32> {
    if nid < 0 {
      return Ok(-1);
    }
    Ok(self.or_node_info(nid)?.xrl_id)
  }

  pub(crate) fn nid_token_id(&self, nid: i32) -> Result<Option<i32>> {
    if nid > NID_LEAF_BASE {
      return Ok(None);
    }
    let and_node_id = nid_to_and_node(nid);
    let token_nsy_id = match self.and_node_info(and_node_id)?.symbol {
      Some(s) => s,
      None => self.bocage.and_node_symbol(and_node_id)?,
    };
    let token_id = self.recce.grammar().source_xsy(token_nsy_id)?;
    Ok(Some(token_id))
  }

  pub(crate) fn nid_symbol_id(&self, nid: i32) -> Result<i32> {
    if let Some(token_id) = self.nid_token_id(nid)? {
      return Ok(token_id);
    } else if nid < 0 {
      return Err(format!("No symbol ID for node ID: {nid}").into());
    }
    // Not a token, so return the LHS of the rule — served from the
    // OrNode cache (which computes irl → xrl → lhs once).
    Ok(self.or_node_info(nid)?.lhs_id)
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
