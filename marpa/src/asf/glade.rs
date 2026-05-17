/// One symbol-choice for a glade.
///
/// A glade represents a parse position. When that position is
/// ambiguous over multiple external rules, each rule contributes one
/// `Symch`. Within a symch, the same external rule may have multiple
/// **factorings** — distinct ways of splitting the input among the
/// rule's RHS positions. Token-only symches use the sentinel
/// `rule_id == -1` and carry a single self-referential factoring
/// `[[self.id]]`, mirroring Perl `Marpa::R2::ASF`.
#[derive(Debug, Clone)]
pub struct Symch {
  pub(crate) rule_id: i32,
  pub(crate) factorings: Vec<Vec<usize>>,
  pub(crate) omitted: bool,
}

impl Symch {
  /// External rule id, or `-1` for a token symch.
  pub fn rule_id(&self) -> i32 {
    self.rule_id
  }

  /// Number of factorings recorded for this symch (after the
  /// `factoring_max` ceiling).
  pub fn factor_count(&self) -> usize {
    self.factorings.len()
  }

  /// `true` if the inner factoring enumeration hit `factoring_max`
  /// and dropped further factorings.
  pub fn factorings_omitted(&self) -> bool {
    self.omitted
  }

  /// The `ix`-th factoring as a slice of child-glade ids
  /// (left-to-right RHS positions).
  pub fn factoring(&self, ix: usize) -> &[usize] {
    &self.factorings[ix]
  }
}

#[derive(Debug, Clone)]
pub struct Glade {
  pub(crate) id: usize,
  pub(crate) symbol_id: i32,
  pub(crate) registered: bool,
  pub(crate) visited: bool,
  pub(crate) symches: Vec<Symch>,
  /// `true` iff this glade wraps a single token-and-node. Token
  /// glades have `symches.len() == 1` with `rule_id == -1` and a
  /// single self-referential factoring.
  pub(crate) is_token: bool,
  /// Iterator state for `Glade::next` — `(symch_ix, factoring_ix)`.
  /// Initialized to `(0, 0)` and advanced by `next()`.
  pub(crate) cursor: (usize, usize),
}

impl Default for Glade {
  fn default() -> Self {
    Glade {
      id: 0,
      symbol_id: -1,
      registered: false,
      visited: false,
      symches: Vec::new(),
      is_token: false,
      cursor: (0, 0),
    }
  }
}

impl Glade {
  /// The glade's identity. Equal to its underlying `nidset_id` in
  /// `ASF`; useful as a HashMap key and for cycle detection.
  pub fn id(&self) -> usize {
    self.id
  }

  /// The grammar symbol this glade derives. For a non-token glade
  /// this is the LHS of the chosen rule; for a token glade it is
  /// the token's external symbol id. Set by `ASF::compute_symches`.
  pub fn symbol_id(&self) -> i32 {
    self.symbol_id
  }

  /// How many distinct symches (symbol-choices) this glade exposes.
  ///
  /// For an unambiguous parse position this is 1; for an ambiguous
  /// position it is the number of competing rule choices.
  pub fn symch_count(&self) -> usize {
    self.symches.len()
  }

  /// True iff the glade wraps a single token-and-node. Token glades
  /// have no real RHS — their sole "factoring" is the glade itself.
  pub fn is_token(&self) -> bool {
    self.is_token
  }

  /// External rule id of the **currently selected** symch.
  /// Returns `-1` for a token glade.
  pub fn rule_id(&self) -> i32 {
    let (symch_ix, _) = self.cursor;
    self
      .symches
      .get(symch_ix)
      .map_or(-1, |s| s.rule_id)
  }

  /// All symches exposed by this glade. Each entry carries a
  /// rule id and its set of factorings.
  pub fn symches(&self) -> &[Symch] {
    &self.symches
  }

  /// Whether the currently-selected symch has more than one
  /// factoring. The cursor is initialized to symch 0 / factoring 0.
  pub fn is_factored(&self) -> bool {
    let (symch_ix, _) = self.cursor;
    self
      .symches
      .get(symch_ix)
      .map(|s| s.factorings.len() > 1)
      .unwrap_or(false)
  }

  /// How many factorings the currently-selected symch has.
  pub fn factor_count(&self) -> usize {
    let (symch_ix, _) = self.cursor;
    self
      .symches
      .get(symch_ix)
      .map(|s| s.factorings.len())
      .unwrap_or(0)
  }

  /// Number of RHS positions in the **currently selected**
  /// (symch, factoring) pair. Returns 1 for a token glade
  /// (the sole self-referential position).
  pub fn rh_length(&self) -> usize {
    let (symch_ix, fact_ix) = self.cursor;
    self
      .symches
      .get(symch_ix)
      .and_then(|s| s.factorings.get(fact_ix))
      .map(|f| f.len())
      .unwrap_or(0)
  }

  /// Glade id at RHS position `ix` of the current factoring.
  pub fn rh_glade_id(&self, ix: usize) -> Option<usize> {
    let (symch_ix, fact_ix) = self.cursor;
    self
      .symches
      .get(symch_ix)
      .and_then(|s| s.factorings.get(fact_ix))
      .and_then(|f| f.get(ix).copied())
  }

  /// `(symch_ix, factoring_ix)` of the current cursor.
  pub fn cursor(&self) -> (usize, usize) {
    self.cursor
  }

  /// Advance to the next factoring within the current symch, or to
  /// the first factoring of the next symch. Returns `Some(())` if
  /// the cursor advanced, `None` if the glade is exhausted.
  ///
  /// **Iteration model**: traversal-time consumers call `next()`
  /// after fully processing the current `(symch, factoring)` pair.
  /// `factor_count`, `rh_length`, `rh_glade_id`, and `rule_id` all
  /// read the cursor — so they see the new position on the next
  /// query.
  #[allow(clippy::should_implement_trait)]
  pub fn next(&mut self) -> Option<()> {
    let (mut s_ix, mut f_ix) = self.cursor;
    if s_ix >= self.symches.len() {
      return None;
    }
    f_ix += 1;
    if f_ix >= self.symches[s_ix].factorings.len() {
      s_ix += 1;
      f_ix = 0;
    }
    if s_ix >= self.symches.len() {
      // Park the cursor past the end so further `next()` calls
      // return `None` without advancing.
      self.cursor = (s_ix, 0);
      return None;
    }
    self.cursor = (s_ix, f_ix);
    Some(())
  }

  /// Reset the cursor to `(0, 0)`. Useful between full traversals
  /// of the same glade.
  pub fn rewind(&mut self) {
    self.cursor = (0, 0);
  }
}
