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
        // println!("{}", proc_value(b.clone(), v));
        parse_count += 1;
    }
    assert_eq!(parse_count, 3, "panda sentence should have three parses with run_recognizer.");
}

/// `Parser::ambiguity_metric` returns 2 (libmarpa's "ambiguous" sentinel)
/// without iterating the parse forest — useful as a pre-flight check.
///
/// Note: the value is 2 even though the grammar admits 3 parses;
/// libmarpa's metric distinguishes "unambiguous (=1)" from "ambiguous
/// (=2)" but does not return the actual parse count. Use
/// `run_recognizer` if you need the exact number.
#[test]
fn ambiguity_metric_oracle_reports_ambiguous() {
    let (mut parser, _b, _rule_names) = build_grammar().expect("grammar build should succeed");
    let metric = parser
        .ambiguity_metric(ByteScanner::new(Cursor::new(PANDA_INPUT)))
        .expect("ambiguity_metric should succeed");
    assert_eq!(metric, 2, "panda sentence is ambiguous → metric == 2");
}

/// `Parser::ambiguity_metric` returns 1 on an unambiguous grammar.
///
/// Constructs a one-rule grammar `S ::= a` and feeds it the single
/// character 'a' — exactly one parse, so metric == 1.
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
fn asf_traverse_parse() {
    let runner_result = runner_asf_traverse();
    assert!(runner_result.is_ok(), "failed to run asf traversal: {:?}", runner_result.err());
}

/// Pin-down test for the ASF peak glade after Step 2 landed.
///
/// `Glade` exposed to the traverser at the peak now reports:
///
/// * a non-trivial `id()` (set by `ASF::peak` → `obtain_nidset`),
/// * a `symbol_id()` corresponding to the grammar's start symbol,
/// * `symch_count() == 1` — the start rule has one user-visible
///   external rule (S → NP ws VP .),
/// * `is_factored() == false` — Perl-faithful glade unification:
///   the 3 panda parses share the same S rule with the same NP, ws,
///   VP, period RHS positions; the ambiguity lives **inside** the
///   VP child glade as 3 source or-nodes unified into one multi-nid
///   nidset.
/// * `factor_count() == 1` — single factoring at S level,
/// * `rh_length() == 4` — the 4 RHS positions of S.
///
/// Step 5 (recursive `ASF::traverse`) is not yet implemented, so
/// `traverse_glade` is still invoked exactly once on the peak.
#[test]
fn asf_peak_glade_scaffolding_pin_down() {
    use std::cell::Cell;
    use std::rc::Rc;

    let (mut parser, _b, _rule_names) = build_grammar().expect("grammar build should succeed");

    let invocation_count = Rc::new(Cell::new(0usize));
    let observed_symbol_id = Rc::new(Cell::new(-99i32));
    let observed_symch_count = Rc::new(Cell::new(usize::MAX));
    let observed_is_factored = Rc::new(Cell::new(true));
    let observed_id_nonzero = Rc::new(Cell::new(false));
    let observed_rh_length = Rc::new(Cell::new(usize::MAX));
    let observed_rule_id_nonneg = Rc::new(Cell::new(false));
    let observed_factor_count = Rc::new(Cell::new(usize::MAX));

    parser
        .parse_and_traverse_forest(
            ByteScanner::new(Cursor::new(PANDA_INPUT)),
            (),
            Box::new(PinDownTraverser {
                invocation_count: invocation_count.clone(),
                observed_symbol_id: observed_symbol_id.clone(),
                observed_symch_count: observed_symch_count.clone(),
                observed_is_factored: observed_is_factored.clone(),
                observed_id_nonzero: observed_id_nonzero.clone(),
                observed_rh_length: observed_rh_length.clone(),
                observed_rule_id_nonneg: observed_rule_id_nonneg.clone(),
                observed_factor_count: observed_factor_count.clone(),
            }),
        )
        .expect("traverse should succeed");

    assert_eq!(
        invocation_count.get(),
        1,
        "traverse_glade should be called exactly once on the peak (recursion is Step 5)"
    );
    assert!(observed_symbol_id.get() >= 0, "peak glade should have a valid (>= 0) symbol_id");
    assert!(observed_id_nonzero.get(), "peak glade id should be non-zero (a valid nidset_id)");
    assert_eq!(observed_symch_count.get(), 1, "S rule has exactly one symch (one external rule)");
    assert!(
        !observed_is_factored.get(),
        "S rule has a single factoring; ambiguity lives in the unified VP child glade"
    );
    assert_eq!(observed_factor_count.get(), 1, "single factoring at S level (Perl-faithful unification)");
    assert_eq!(observed_rh_length.get(), 4, "S → NP ws VP . has 4 RHS positions");
    assert!(observed_rule_id_nonneg.get(), "peak glade's current rule_id is a valid (>= 0) XRL");
}

struct PinDownTraverser {
    invocation_count: std::rc::Rc<std::cell::Cell<usize>>,
    observed_symbol_id: std::rc::Rc<std::cell::Cell<i32>>,
    observed_symch_count: std::rc::Rc<std::cell::Cell<usize>>,
    observed_is_factored: std::rc::Rc<std::cell::Cell<bool>>,
    observed_id_nonzero: std::rc::Rc<std::cell::Cell<bool>>,
    observed_rh_length: std::rc::Rc<std::cell::Cell<usize>>,
    observed_rule_id_nonneg: std::rc::Rc<std::cell::Cell<bool>>,
    observed_factor_count: std::rc::Rc<std::cell::Cell<usize>>,
}

impl Traverser for PinDownTraverser {
    type ParseTree = ();
    type ParseState = ();
    fn traverse_glade(&self, glade: &mut Glade, _state: Self::ParseState) -> Result<(Self::ParseTree, Self::ParseState)> {
        self.invocation_count.set(self.invocation_count.get() + 1);
        self.observed_symbol_id.set(glade.symbol_id());
        self.observed_symch_count.set(glade.symch_count());
        self.observed_is_factored.set(glade.is_factored());
        self.observed_id_nonzero.set(glade.id() != 0);
        self.observed_rh_length.set(glade.rh_length());
        self.observed_rule_id_nonneg.set(glade.rule_id() >= 0);
        self.observed_factor_count.set(glade.factor_count());
        Ok(((), ()))
    }
}

fn runner_asf_traverse() -> Result<Vec<String>> {
    let (mut parser, _b, rule_names) = build_grammar().expect("grammar build should succeed, not core part of test");
    // Now that we have validated the panda grammar is correctly ambiguous,
    // reparse it via the ASFs
    let _parse_forest_iterator = parser.parse_and_traverse_forest(
        ByteScanner::new(Cursor::new(PANDA_INPUT)),
        (), //init state
        Box::new(ExhaustiveTraverser {
            rule_names: rule_names.clone(),
        }),
    )?;

    let _parse_forest_iterator = parser.parse_and_traverse_forest(
        ByteScanner::new(Cursor::new(PANDA_INPUT)),
        (), //init state
        Box::new(PruningTraverser { rule_names }),
    )?;

    Ok(Vec::new())
}

// Do a standalone build for each test, to avoid reentrance errors
fn build_grammar() -> Result<(Parser, TreeBuilder, HashMap<i32, &'static str>)> {
    let mut g = Grammar::new()?;
    let b = TreeBuilder::new();

    let ws = g.string_set(None, "\t\n\r ")?;
    //b.discard(ws.rule());

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

    // for t_rule in &[cc, det1, det2, panda, eats, shoots, leaves] {
    //   b.token(t_rule.rule());
    // }
    // for r in &[nn, dt, nns, vbz, np, vp, s] {
    //   b.rule(r.rule());
    // }

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
    #[allow(dead_code)]
    rule_names: HashMap<i32, &'static str>,
}
struct PruningTraverser {
    #[allow(dead_code)]
    rule_names: HashMap<i32, &'static str>,
}

impl Traverser for ExhaustiveTraverser {
    type ParseTree = ();
    type ParseState = ();
    fn traverse_glade(&self, glade: &mut Glade, _state: Self::ParseState) -> Result<(Self::ParseTree, Self::ParseState)> {
        // This routine converts the glade into a list of Penn-tagged elements.
        // It is called recursively.
        let glade_id = dbg!(glade.id());
        let _symbol_id = dbg!(glade.symbol_id());

        // A token is a single choice, and we know enough to fully Penn-tag it
        if glade_id == 0 {
            //   let literal  = glade.literal();
            //   let penn_tag = penn_tag.get(symbol_id);
            //   return Ok(vec![format!("({} {})",penn_tag, literal)]);
        }

        // let mut return_value = Vec::new();

        // loop {
        //   // The results at each position are a list of choices, so
        //   // to produce a new result list, we need to take a Cartesian
        //   // product of all the choices
        //   let mut results = vec![Vec::new()];
        //   for rh_ix in 0 .. glade.rh_length() {
        //     let mut new_results = Vec::new();
        //     for prev_result in results.drain(..) {
        //       let child_value = glade.rh_value(rh_ix);
        //       for new_value in child_value.into_iter() {
        //         let prev_update = prev_result.clone();
        //         prev_update.push(new_value);
        //         new_results.push(prev_update);
        //       }
        //     }
        //     results = new_results;
        //   }

        //   // Special case for the start rule
        //   // if ( $symbol_name eq '[:start]' ) {
        //   //   return [ map { join q{}, @{$_} } @results ];
        //   // }

        //   // Now we have a list of choices, as a list of lists.  Each sub list
        //   // is a list of Penn-tagged elements, which we need to join into
        //   // a single Penn-tagged element.  The result will be to collapse
        //   // one level of lists, and leave us with a list of Penn-tagged
        //   // elements

        //   return_value.push(results.into_iter().map(|result|
        //      format!("({} {})", penn_tag.get(symbol_id), result.join(" "))
        //   ));

        //   // Look at the next alternative in this glade, or end the
        //   // loop if there is none
        //   if glade.next().is_none() {
        //     break;
        //   }
        // }

        Ok(((), ()))
    }
}

impl Traverser for PruningTraverser {
    type ParseTree = ();
    type ParseState = ();
    fn traverse_glade(&self, _glade: &mut Glade, _state: Self::ParseState) -> Result<(Self::ParseTree, Self::ParseState)> {
        Ok(((), ()))
    }
}
