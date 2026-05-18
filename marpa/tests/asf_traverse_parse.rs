extern crate marpa;

use marpa::grammar::Grammar;
use marpa::lexer::byte_scanner::*;
use marpa::parser::*;
use marpa::result::Result;

use marpa::asf::{Glade, Traverser};
use marpa::tree_builder::*;

use std::io::Cursor;

use std::collections::HashMap;

const PANDA_INPUT: &str = "a panda eats shoots and leaves.";

#[test]
fn recce_parse_sanity() {
  // check that recce behaves as expected, for sanity's sake
  let (mut parser, _b, _rule_names) = build_grammar().expect("grammar build should succeed, not core part of test");

  let parsed_result_iterator = parser
    .run_recognizer(ByteScanner::new(Cursor::new(PANDA_INPUT)))
    .expect("recognizer suceeds");
  let mut parse_count = 0;
  for _v in parsed_result_iterator {
    parse_count += 1;
  }
  assert_eq!(parse_count, 3, "panda sentence should have three parses with run_recognizer.");
}

/// `Parser::ambiguity_metric` returns 2 (libmarpa's "ambiguous" sentinel)
/// without iterating the parse forest — useful as a pre-flight check.
#[test]
fn ambiguity_metric_oracle_reports_ambiguous() {
  let (mut parser, _b, _rule_names) = build_grammar().expect("grammar build should succeed");
  let metric = parser
    .ambiguity_metric(ByteScanner::new(Cursor::new(PANDA_INPUT)))
    .expect("ambiguity_metric should succeed");
  assert_eq!(metric, 2, "panda sentence is ambiguous → metric == 2");
}

/// `Parser::ambiguity_metric` returns 1 on an unambiguous grammar.
#[test]
fn ambiguity_metric_oracle_reports_unambiguous() {
  let mut g = Grammar::new().expect("grammar new");
  let a = g.literal_string(None, "a").expect("literal a");
  let s = g.rule(None, &[a]).expect("rule S ::= a");
  g.set_start(s).expect("set_start");
  let mut parser = Parser::with_grammar(g.unwrap());
  let metric = parser
    .ambiguity_metric(ByteScanner::new(Cursor::new("a")))
    .expect("ambiguity_metric should succeed");
  assert_eq!(metric, 1, "single-rule grammar with one parse → metric == 1");
}

#[test]
fn hybrid_parse_returns_tree_for_unambiguous_input() {
  let mut g = Grammar::new().expect("grammar new");
  let a = g.literal_string(None, "a").expect("literal a");
  let s = g.rule(None, &[a]).expect("rule S ::= a");
  g.set_start(s).expect("set_start");
  let mut parser = Parser::with_grammar(g.unwrap());
  let mut traverser = PanicTraverser;

  match parser
    .parse_hybrid(ByteScanner::new(Cursor::new("a")), (), &mut traverser)
    .expect("hybrid parse should succeed")
  {
    HybridParseResult::Unambiguous(tree) => {
      assert_eq!(tree.count(), 1, "unambiguous branch should return the single Tree iterator");
    },
    HybridParseResult::Ambiguous(_, _) => panic!("unambiguous grammar should not traverse ASF"),
    HybridParseResult::AmbiguousTree(_, _) => panic!("unambiguous grammar should not route through ambiguous fallback"),
  }
}

#[test]
fn hybrid_parse_traverses_asf_for_ambiguous_input() {
  let (mut parser, _b, rule_names) = build_grammar().expect("grammar build should succeed");
  let mut traverser = ExhaustiveTraverser { rule_names };

  match parser
    .parse_hybrid(ByteScanner::new(Cursor::new(PANDA_INPUT)), (), &mut traverser)
    .expect("hybrid parse should succeed")
  {
    HybridParseResult::Unambiguous(_) => panic!("panda grammar should route to ASF"),
    HybridParseResult::AmbiguousTree(_, _) => panic!("panda grammar should stay below the fallback limit"),
    HybridParseResult::Ambiguous(alts, ()) => {
      let mut unique = alts;
      unique.sort();
      unique.dedup();
      assert_eq!(unique.len(), 3, "ambiguous branch should expose the three panda parses");
    },
  }
}

#[test]
fn hybrid_parse_can_fallback_to_tree_for_large_ambiguous_bocage() {
  let (mut parser, _b, rule_names) = build_grammar().expect("grammar build should succeed");
  let mut traverser = ExhaustiveTraverser { rule_names };

  match parser
    .parse_hybrid_with_and_node_limit(ByteScanner::new(Cursor::new(PANDA_INPUT)), (), &mut traverser, Some(0))
    .expect("hybrid parse should succeed")
  {
    HybridParseResult::Unambiguous(_) => panic!("panda grammar should be ambiguous"),
    HybridParseResult::Ambiguous(_, _) => panic!("zero limit should force tree fallback"),
    HybridParseResult::AmbiguousTree(tree, stats) => {
      assert!(stats.or_node_count > 0);
      assert!(stats.and_node_count > 0);
      assert_eq!(tree.count(), 3, "fallback tree should expose the three panda parses");
    },
  }
}

#[test]
fn asf_traverse_parse() {
  let runner_result = runner_asf_traverse();
  assert!(runner_result.is_ok(), "failed to run asf traversal: {:?}", runner_result.err());
}

/// Pin-down test of the post-Step-5 recursive traversal state.
///
/// The recursive driver now invokes `Traverser::traverse_glade` once
/// per reachable glade in post-order. We probe two specific glades:
///
/// * **The peak (start symbol S)**: `symch_count == 1`, single
///   factoring at the top, `rh_length == 4` (S → NP ws VP .).
/// * **The unified VP glade**: `symch_count == 3` — the 3 panda VP
///   rules unified into one glade via multi-source nidset. Each
///   symch carries one factoring (single rule, single split).
///
/// Both observations come from the same traversal pass — the
/// `PinDownTraverser` records per-glade properties keyed by
/// `symch_count`, so we can extract the S frame (count=1) and the VP
/// frame (count=3) independently of glade-id order.
#[test]
fn asf_peak_glade_scaffolding_pin_down() {
  use std::cell::RefCell;
  use std::rc::Rc;

  let (mut parser, _b, _rule_names) = build_grammar().expect("grammar build should succeed");

  let log: Rc<RefCell<Vec<GladeFrame>>> = Rc::new(RefCell::new(Vec::new()));

  let mut traverser = PinDownTraverser { log: log.clone() };
  parser
    .parse_and_traverse_forest(ByteScanner::new(Cursor::new(PANDA_INPUT)), (), &mut traverser)
    .expect("traverse should succeed");

  let frames = log.borrow();
  assert!(!frames.is_empty(), "post-order traversal should produce at least one frame");
  // dbg!(&*frames);

  // Glade ids must be unique post-memoization. If the recursive
  // driver ever called `traverse_glade` twice for the same glade,
  // the memoization is broken.
  let mut ids: Vec<usize> = frames.iter().map(|f| f.glade_id).collect();
  ids.sort();
  let unique_len = {
    let mut copy = ids.clone();
    copy.dedup();
    copy.len()
  };
  assert_eq!(unique_len, ids.len(), "memoization should ensure each glade fires once");

  // S frame: the peak — only glade reached with the start XRL,
  // factor_count == 1, rh_length == 4.
  let s_frames: Vec<&GladeFrame> = frames.iter().filter(|f| f.rh_length == 4 && f.symch_count == 1).collect();
  assert!(
    !s_frames.is_empty(),
    "should find at least one S-shaped glade (NP ws VP .); got frames: {:#?}",
    *frames
  );
  let s = s_frames[0];
  assert_eq!(s.factor_count, 1, "S has a single factoring (Perl-faithful unification)");
  assert!(!s.is_factored, "S not factored");
  assert!(s.symbol_id >= 0, "S symbol_id well-formed");
  assert!(s.id_nonzero, "S glade id non-zero");

  // VP_unified frame: 3 symches (one per VP variant rule).
  let vp_frames: Vec<&GladeFrame> = frames.iter().filter(|f| f.symch_count == 3).collect();
  assert_eq!(vp_frames.len(), 1, "exactly one 3-symch glade — the unified VP");
  let vp = vp_frames[0];
  assert_eq!(vp.factor_count, 1, "first VP symch has one factoring");
  assert!(!vp.is_factored, "VP's first symch not factored");
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // fields are read via the Debug derive in failure messages
struct GladeFrame {
  glade_id: usize,
  rule_id: i32,
  symbol_id: i32,
  symch_count: usize,
  factor_count: usize,
  is_factored: bool,
  rh_length: usize,
  id_nonzero: bool,
}

struct PinDownTraverser {
  log: std::rc::Rc<std::cell::RefCell<Vec<GladeFrame>>>,
}

impl Traverser for PinDownTraverser {
  type ParseTree = ();
  type ParseState = ();
  fn traverse_glade(
    &mut self,
    glade: &mut Glade,
    _children: &[Option<Self::ParseTree>],
    _state: &mut Self::ParseState,
  ) -> Result<Self::ParseTree> {
    self.log.borrow_mut().push(GladeFrame {
      glade_id: glade.id(),
      rule_id: glade.rule_id(),
      symbol_id: glade.symbol_id(),
      symch_count: glade.symch_count(),
      factor_count: glade.factor_count(),
      is_factored: glade.is_factored(),
      rh_length: glade.rh_length(),
      id_nonzero: glade.id() != 0,
    });
    Ok(())
  }
}

/// Substantive 3-parse test: ExhaustiveTraverser enumerates every
/// (symch, factoring) combination as a Penn-tagged string. With
/// memoization, child glades produce their own alternative-list
/// once; the parent Cartesian-products across positions.
///
/// Expected result: exactly **3 distinct Penn-tagged strings** for
/// the panda sentence — one per parse the recognizer admits.
#[test]
fn asf_three_parses_via_exhaustive_traverser() {
  let (mut parser, _b, rule_names) = build_grammar().expect("grammar build should succeed");

  let mut traverser = ExhaustiveTraverser {
    rule_names: rule_names.clone(),
  };
  let (out, _state) = parser
    .parse_and_traverse_forest(ByteScanner::new(Cursor::new(PANDA_INPUT)), (), &mut traverser)
    .expect("traverse should succeed");

  // De-duplicate at the top to ignore order/repeats. The peak's
  // Vec<String> should contain 3 distinct Penn-tagged sentences.
  let mut distinct: Vec<String> = out.clone();
  distinct.sort();
  distinct.dedup();
  assert_eq!(distinct.len(), 3, "panda should parse to 3 distinct Penn-tagged sentences, got: {distinct:?}");
}

fn runner_asf_traverse() -> Result<Vec<String>> {
  let (mut parser, _b, rule_names) = build_grammar().expect("grammar build should succeed, not core part of test");
  let mut exhaustive = ExhaustiveTraverser {
    rule_names: rule_names.clone(),
  };
  let _ = parser.parse_and_traverse_forest(ByteScanner::new(Cursor::new(PANDA_INPUT)), (), &mut exhaustive)?;

  let mut pruning = PruningTraverser { rule_names };
  let _ = parser.parse_and_traverse_forest(ByteScanner::new(Cursor::new(PANDA_INPUT)), (), &mut pruning)?;

  Ok(Vec::new())
}

// Do a standalone build for each test, to avoid reentrance errors
fn build_grammar() -> Result<(Parser, TreeBuilder, HashMap<i32, &'static str>)> {
  let mut g = Grammar::new()?;
  let b = TreeBuilder::new();

  let ws = g.string_set(None, "\t\n\r ")?;

  let period = g.literal_string(None, ".")?;
  let cc = g.literal_string(None, "and")?;
  let det1 = g.literal_string(None, "a")?;
  let det2 = g.literal_string(None, "an")?;
  let dt = g.alternative(None, &[det1, det2])?;
  let panda = g.literal_string(None, "panda")?;
  let eats = g.literal_string(None, "eats")?;
  let shoots = g.literal_string(None, "shoots")?;
  let leaves = g.literal_string(None, "leaves")?;

  let nns = g.alternative(None, &[shoots, leaves])?;
  let vbz = g.alternative(None, &[eats, shoots, leaves])?;

  let nn = g.rule(None, &[panda])?;
  let np = g.rule(None, &[nn])?;
  let _np_simple_2 = g.rule(Some(np), &[nns])?;
  let _np_compound_1 = g.rule(Some(np), &[dt, ws, nn])?;
  let _np_compound_2 = g.rule(Some(np), &[nn, ws, nns])?;
  let _np_compound_3 = g.rule(Some(np), &[nns, ws, cc, ws, nns])?;

  let vp = g.rule(None, &[vbz])?;
  let _vp_1 = g.rule(Some(vp), &[vbz, ws, np])?;
  let _vp_2 = g.rule(Some(vp), &[vp, ws, vbz, ws, nns])?;
  let _vp_3 = g.rule(Some(vp), &[vp, ws, cc, ws, vp])?;
  let _vp_4 = g.rule(Some(vp), &[vp, ws, vp, ws, cc, ws, vp])?;

  let s = g.rule(None, &[np, ws, vp, period])?;
  g.set_start(s)?;

  let mut rule_names = HashMap::new();
  rule_names.insert(np.rule(), "NP");
  rule_names.insert(vp.rule(), "VP");
  rule_names.insert(s.rule(), "S");
  rule_names.insert(nn.rule(), "NN");
  rule_names.insert(nns.rule(), "NNS");
  rule_names.insert(vbz.rule(), "VBZ");
  rule_names.insert(dt.rule(), "DT");
  let parser = Parser::with_grammar(g.unwrap());
  Ok((parser, b, rule_names))
}

struct ExhaustiveTraverser {
  rule_names: HashMap<i32, &'static str>,
}
struct PruningTraverser {
  #[allow(dead_code)]
  rule_names: HashMap<i32, &'static str>,
}
struct PanicTraverser;

impl Traverser for PanicTraverser {
  type ParseTree = ();
  type ParseState = ();

  fn traverse_glade(
    &mut self,
    _glade: &mut Glade,
    _children: &[Option<Self::ParseTree>],
    _state: &mut Self::ParseState,
  ) -> Result<Self::ParseTree> {
    panic!("unambiguous hybrid parse should not invoke ASF traversal")
  }
}

impl Traverser for ExhaustiveTraverser {
  /// Each glade contributes a list of alternative strings — the
  /// Penn-tagged renderings reachable at this parse position. The
  /// parent Cartesian-products its RHS children to compose its own
  /// alternatives.
  type ParseTree = Vec<String>;
  type ParseState = ();

  fn traverse_glade(
    &mut self,
    glade: &mut Glade,
    children: &[Option<Self::ParseTree>],
    _state: &mut Self::ParseState,
  ) -> Result<Self::ParseTree> {
    // Token glade: the rendering is a Penn-tag wrapping the symbol id.
    // We don't yet have `glade.literal()` (deferred Step 4 piece), so
    // use the symbol id as a stand-in for the literal.
    if glade.is_token() {
      let tag = format!("T{}", glade.symbol_id());
      return Ok(vec![tag]);
    }

    let mut result_set: Vec<String> = Vec::new();

    loop {
      // For the current (symch, factoring) pair, build the Cartesian
      // product of child alternatives.
      let rule_id = glade.rule_id();
      let rh_len = glade.rh_length();
      let mut accum: Vec<String> = vec![String::new()];
      for ix in 0..rh_len {
        let child_id = glade.rh_glade_id(ix).expect("RHS position has a child glade");
        let child_alts = children.get(child_id).and_then(|o| o.as_ref()).expect("child precomputed in post-order");
        let mut next_accum = Vec::with_capacity(accum.len() * child_alts.len());
        for prefix in &accum {
          for alt in child_alts {
            if prefix.is_empty() {
              next_accum.push(alt.clone());
            } else {
              next_accum.push(format!("{prefix} {alt}"));
            }
          }
        }
        accum = next_accum;
      }

      // Wrap each combination in this rule's Penn-tag (or skip
      // tagging if it's the synthetic start rule).
      let tag = self.rule_names.get(&rule_id).copied();
      for combo in accum {
        match tag {
          Some(name) => result_set.push(format!("({name} {combo})")),
          // Untagged internal rule: pass children through.
          None => result_set.push(combo),
        }
      }

      if glade.next().is_none() {
        break;
      }
    }

    Ok(result_set)
  }
}

impl Traverser for PruningTraverser {
  /// PruningTraverser picks the **first** (symch, factoring) only,
  /// yielding exactly one rendering even on an ambiguous parse.
  type ParseTree = Vec<String>;
  type ParseState = ();

  fn traverse_glade(
    &mut self,
    glade: &mut Glade,
    children: &[Option<Self::ParseTree>],
    _state: &mut Self::ParseState,
  ) -> Result<Self::ParseTree> {
    if glade.is_token() {
      return Ok(vec![format!("T{}", glade.symbol_id())]);
    }

    // No iteration — single (symch=0, factoring=0) by default.
    let rh_len = glade.rh_length();
    let mut accum = String::new();
    for ix in 0..rh_len {
      let child_id = glade.rh_glade_id(ix).expect("rh position has a child glade");
      let child_alts = children.get(child_id).and_then(|o| o.as_ref()).expect("child precomputed");
      let first = child_alts.first().cloned().unwrap_or_default();
      if accum.is_empty() {
        accum = first;
      } else {
        accum.push(' ');
        accum.push_str(&first);
      }
    }
    Ok(vec![accum])
  }
}
