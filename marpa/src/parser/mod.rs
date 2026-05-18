use std::mem;

use crate::asf::{ASF, Traverser};
use crate::lexer::token::Token;
use crate::lexer::token_source::TokenSource;
use crate::result::Result;
use crate::thin::{
    Bocage,
    Grammar,
    Order,
    Recognizer,
    Tree,
    // Value,
};

#[allow(dead_code)]
enum MarpaState {
    G,
    GReady,
    R(Recognizer),
    B(Bocage),
    O(Order),
    T(Tree),
}

use self::MarpaState::{B, G, GReady, O, R, T};

impl MarpaState {
    fn new() -> Self {
        G
    }
}

impl Default for MarpaState {
    fn default() -> Self {
        MarpaState::new()
    }
}

pub struct Parser {
    grammar: Grammar,
    state: MarpaState,
}

/// Result of a one-pass parse that routes by raw Marpa ambiguity.
///
/// `Unambiguous` contains the ordinary libmarpa tree iterator for
/// callers that want the cheaper Step/value path. `Ambiguous` contains
/// the result of ASF traversal. The ambiguity decision is grammar-level
/// only; semantic pruning remains the caller's responsibility.
pub enum HybridParseResult<PT, PS> {
    Unambiguous(Tree),
    Ambiguous(PT, PS),
    AmbiguousTree(Tree, BocageStats),
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BocageStats {
    pub or_node_count: usize,
    pub and_node_count: usize,
    pub max_and_nodes_per_or_node: usize,
}

impl Default for Parser {
    fn default() -> Self {
        Parser {
            state: MarpaState::default(),
            grammar: Grammar::new().unwrap(),
        }
    }
}

macro_rules! get_state {
    ($e:expr, $s:ident) => {{
        match $e.state {
            $s(ref mut g) => g,
            _ => return Err(format!("Marpa is not in the {} state", stringify!($s)).into()),
        }
    }};
}

impl Parser {
    pub fn new() -> Self {
        Parser::default()
    }

    pub fn with_grammar(grammar: Grammar) -> Self {
        Parser { state: G, grammar }
    }

    /// Construct a parser around a grammar that has already been
    /// precomputed.
    ///
    /// Most callers should use `with_grammar`; this is for owners that
    /// intentionally cache a precomputed grammar and need a fresh
    /// recognizer without running `Grammar::precompute()` again.
    pub fn with_precomputed_grammar(grammar: Grammar) -> Self {
        Parser { state: GReady, grammar }
    }

    fn adv_marpa(&mut self) -> Result<()> {
        let next_state = match self.state {
            G => {
                self.grammar.precompute()?;
                Recognizer::new(self.grammar.clone()).map(R)
            }
            GReady => Recognizer::new(self.grammar.clone()).map(R),
            R(ref r) => Bocage::new(r).map(B),
            B(ref b) => Order::new(b).map(O),
            O(ref o) => Tree::new(o.clone()).map(T),
            T(_) => Ok(GReady),
        };
        self.state = next_state?;
        Ok(())
    }

    pub fn read<T: TokenSource<U>, U: Token>(&mut self, mut tokens: T) -> Result<()> {
        loop {
            // just prep the recognizer, irrespective of initial state
            match self.state {
                R(_) => break,
                _ => self.adv_marpa()?,
            }
        }
        {
            // limit recognizer borrow
            let r = get_state!(self, R);
            r.start_input()?;
            loop {
                if r.is_exhausted() {
                    break;
                }
                let maybe_tok = tokens.next();
                match maybe_tok {
                    None => break,
                    Some(tok) => {
                        Parser::consume_tok(r, tok)?;
                    }
                }
            }
        }
        Ok(())
    }
    pub fn run_recognizer<T: TokenSource<U>, U: Token>(&mut self, tokens: T) -> Result<Tree> {
        self.read(tokens)?;
        loop {
            self.adv_marpa()?;
            if let T(ref tree) = self.state {
                return Ok(tree.clone());
            }
        }
    }

    fn consume_tok<U: Token>(r: &mut Recognizer, tok: U) -> Result<()> {
        r.alternative(tok.sym(), tok.value(), 1)?;
        r.earleme_complete()?;
        Ok(())
    }

    /// This is roughly equivalent to `$asf->traverse` in Marpa::R2,
    /// but the ASF details are hidden under the hood.
    ///
    /// Takes `&mut TR` rather than `Box<dyn Traverser>` so the
    /// traverser can borrow external state (e.g. the math parser's
    /// `&mut Document`, `&Actions`). The trait method is
    /// monomorphized at the call site — no dyn-dispatch overhead.
    ///
    /// `TR::ParseTree: Clone` because the recursive driver memoizes
    /// each glade's output and may hand the same value to multiple
    /// parents (shared sub-glades in the DAG). Wrap expensive types
    /// in `Rc` if cloning is costly.
    pub fn parse_and_traverse_forest<T, U, TR>(
        &mut self,
        tokens: T,
        init_state: TR::ParseState,
        traverser: &mut TR,
    ) -> Result<(TR::ParseTree, TR::ParseState)>
    where
        T: TokenSource<U>,
        U: Token,
        TR: Traverser,
        TR::ParseTree: Clone,
    {
        // we need to read the tokens before starting the ASF step
        self.read(tokens)?;
        if let R(recce) = mem::replace(&mut self.state, GReady) {
            let mut asf = ASF::new(recce)?;
            asf.traverse(init_state, traverser)
        } else {
            panic!("Parser::read must always terminate in the R state!");
        }
    }

    /// Read `tokens` once, build one bocage, and route by raw Marpa
    /// ambiguity.
    ///
    /// For unambiguous parses this returns the ordinary `Tree` iterator,
    /// avoiding ASF symch/factoring construction. For ambiguous parses it
    /// traverses the ASF using the supplied traverser. This deliberately
    /// does not compose `ambiguity_metric()` with
    /// `parse_and_traverse_forest`, because that would scan the token
    /// stream and build recognizer state twice.
    ///
    /// Parser state after return follows the branch: `Unambiguous` leaves
    /// the parser in `T`, matching `run_recognizer`; `Ambiguous` consumes
    /// the recognizer and leaves the parser ready to build a fresh
    /// recognizer on the next parse, matching `parse_and_traverse_forest`.
    pub fn parse_hybrid<T, U, TR>(
        &mut self,
        tokens: T,
        init_state: TR::ParseState,
        traverser: &mut TR,
    ) -> Result<HybridParseResult<TR::ParseTree, TR::ParseState>>
    where
        T: TokenSource<U>,
        U: Token,
        TR: Traverser,
        TR::ParseTree: Clone,
    {
        self.parse_hybrid_with_and_node_limit(tokens, init_state, traverser, None)
    }

    /// Like `parse_hybrid`, but ambiguous bocages whose total
    /// and-node count exceeds `max_and_nodes` are routed back to the
    /// ordinary `Tree` iterator instead of constructing the ASF.
    ///
    /// This is a pressure valve for high-cardinality ambiguous
    /// forests: `Tree` iteration streams alternatives and lets the
    /// caller's existing caps stop early, while ASF construction must
    /// allocate the Rust-side glade/factoring view up front.
    pub fn parse_hybrid_with_and_node_limit<T, U, TR>(
        &mut self,
        tokens: T,
        init_state: TR::ParseState,
        traverser: &mut TR,
        max_and_nodes: Option<usize>,
    ) -> Result<HybridParseResult<TR::ParseTree, TR::ParseState>>
    where
        T: TokenSource<U>,
        U: Token,
        TR: Traverser,
        TR::ParseTree: Clone,
    {
        self.read(tokens)?;
        if let R(recce) = mem::replace(&mut self.state, GReady) {
            let bocage = Bocage::new(&recce)?;
            if bocage.ambiguity_metric()? == 1 {
                let order = Order::new(&bocage)?;
                let tree = Tree::new(order)?;
                self.state = T(tree.clone());
                Ok(HybridParseResult::Unambiguous(tree))
            } else {
                if let Some(max_and_nodes) = max_and_nodes {
                    let mut order = Order::new(&bocage)?;
                    let stats = bocage_stats(&mut order)?;
                    if stats.and_node_count > max_and_nodes {
                        let tree = Tree::new(order)?;
                        self.state = T(tree.clone());
                        return Ok(HybridParseResult::AmbiguousTree(tree, stats));
                    }
                }
                let mut asf = ASF::from_parts(recce, bocage)?;
                let (output, state) = asf.traverse(init_state, traverser)?;
                Ok(HybridParseResult::Ambiguous(output, state))
            }
        } else {
            panic!("Parser::read must always terminate in the R state!");
        }
    }

    /// Cheap ambiguity oracle: scan `tokens`, build the bocage, return its
    /// ambiguity metric, and reset the parser to `GReady` for reuse.
    ///
    /// The returned value follows libmarpa's `marpa_b_ambiguity_metric`:
    ///
    /// * `1` — unambiguous (exactly one parse tree).
    /// * `2` — ambiguous (two or more parse trees). The exact count is
    ///   only obtainable by iterating `run_recognizer` to exhaustion or
    ///   walking the ASF.
    ///
    /// Use this as a fast pre-flight check before deciding whether to
    /// commit to full tree enumeration. Avoids the cost of building the
    /// Order + Tree iterator when the caller only needs to know "is this
    /// ambiguous at all?".
    pub fn ambiguity_metric<T: TokenSource<U>, U: Token>(&mut self, tokens: T) -> Result<i32> {
        self.read(tokens)?;
        let metric = {
            let r = get_state!(self, R);
            let bocage = Bocage::new(r)?;
            bocage.ambiguity_metric()?
        };
        // Restore the parser to a clean state for the next parse call.
        self.state = GReady;
        Ok(metric)
    }
}

fn bocage_stats(order: &mut Order) -> Result<BocageStats> {
    let mut stats = BocageStats::default();
    loop {
        let Some(and_node_count) = order.or_node_and_node_count_opt(stats.or_node_count)? else {
            break;
        };
        stats.and_node_count += and_node_count;
        stats.max_and_nodes_per_or_node = stats.max_and_nodes_per_or_node.max(and_node_count);
        stats.or_node_count += 1;
    }
    Ok(stats)
}
