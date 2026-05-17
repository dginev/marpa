// `visited` is unused until the glade-traversal port lands; keep
// the field allocated so the eventual is-visited fast path doesn't
// need a schema change.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Glade {
    pub(crate) id: usize,
    pub(crate) symbol_id: i32,
    pub(crate) registered: bool,
    pub(crate) visited: bool,
    pub(crate) symches: Vec<usize>,
}

impl Default for Glade {
    fn default() -> Self {
        Glade {
            id: 0,
            symbol_id: -1,
            registered: false,
            visited: false,
            symches: Vec::new(),
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

    /// How many distinct symches (symbol-choices) this glade can take.
    ///
    /// For an unambiguous parse position this is 1; for an ambiguous
    /// position it is the number of competing rule choices.
    ///
    /// **Note**: until `ASF::compute_symches` is fleshed out (see
    /// `ASF_STATUS.md` Step 2), this returns 0 for every glade —
    /// the inner factoring loop that populates `symches` is still
    /// commented-out Perl source.
    pub fn symch_count(&self) -> usize {
        self.symches.len()
    }

    /// Whether this glade has more than one factoring at its
    /// currently-selected symch.
    ///
    /// **Note**: until factoring stack navigation is implemented
    /// (Step 2 in `ASF_STATUS.md`), this returns `false` for every
    /// glade.
    pub fn is_factored(&self) -> bool {
        false
    }
}
