//! Performance comparison: `run_recognizer` (Tree iteration) vs
//! `parse_and_traverse_forest` (ASF traversal).
//!
//! Run with `cargo test --release --test asf_perf_compare -- --nocapture`
//! to see timings (release mode strips debug-assertion overhead).
//!
//! ## What this measures
//!
//! Both paths produce the same set of parses for a grammar; the
//! difference is **how the per-rule semantic logic scales** with
//! ambiguity:
//!
//! * **Tree iteration**: each `Value` from `run_recognizer` is a
//!   full parse tree. To compute output, you walk every node of
//!   every tree — cost is `O(trees × tree_size)`.
//! * **ASF traversal**: post-order `parse_and_traverse_forest`
//!   visits each glade **once** and memoizes its output; parents
//!   compose via Cartesian product over child alternatives. Cost is
//!   `O(glades) + Cartesian-product expansion at each glade`.
//!
//! For unambiguous parses, both paths do equivalent work. For
//! grammars where ambiguity is **structural** (different
//! factorings of the same rule LHS at the same span), ASF wins
//! by a polynomial-vs-factorial margin.
//!
//! The panda grammar (3 parses, 31 chars) doesn't show much
//! difference at this scale. The classic explosion grammar
//! `E → E + E | a` does — `Catalan(N-1)` parses on `N` operands.

extern crate marpa;

use marpa::grammar::Grammar;
use marpa::lexer::byte_scanner::*;
use marpa::parser::*;
use marpa::result::Result;

use marpa::asf::{Glade, Traverser};

use std::io::Cursor;
use std::time::Instant;

const ITERATIONS: usize = 10;

/// Trivial traverser that just walks the forest computing the
/// per-glade alternative-count. Mirrors the kind of light per-glade
/// work the math parser will do in the ASF migration; gives a
/// fair "ASF traversal cost" baseline.
struct CountTraverser;

impl Traverser for CountTraverser {
  type ParseTree = usize;
  type ParseState = ();
  fn traverse_glade(&mut self, glade: &mut Glade, children: &[Option<usize>], _state: &mut ()) -> Result<usize> {
    if glade.is_token() {
      return Ok(1);
    }
    let mut total: usize = 0;
    loop {
      let rh_len = glade.rh_length();
      let mut combo: usize = 1;
      for ix in 0..rh_len {
        let child_id = glade.rh_glade_id(ix).expect("rh");
        let n = children.get(child_id).and_then(|o| o.as_ref()).copied().unwrap_or(1);
        combo = combo.saturating_mul(n);
      }
      total = total.saturating_add(combo);
      if glade.next().is_none() {
        break;
      }
    }
    Ok(total)
  }
}

fn build_panda_grammar() -> Result<Parser> {
  let mut g = Grammar::new()?;
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
  Ok(Parser::with_grammar(g.unwrap()))
}

fn time_run_recognizer(input: &str, build: fn() -> Result<Parser>) -> (u128, usize) {
  let start = Instant::now();
  let mut count = 0usize;
  for _ in 0..ITERATIONS {
    let mut parser = build().unwrap();
    let iter = parser.run_recognizer(ByteScanner::new(Cursor::new(input))).unwrap();
    count = 0;
    for _ in iter {
      count += 1;
    }
  }
  let avg_ns = start.elapsed().as_nanos() / ITERATIONS as u128;
  (avg_ns, count)
}

fn time_asf_traverse(input: &str, build: fn() -> Result<Parser>) -> (u128, usize) {
  let start = Instant::now();
  let mut count = 0usize;
  for _ in 0..ITERATIONS {
    let mut parser = build().unwrap();
    let mut traverser = CountTraverser;
    let (n, _) = parser
      .parse_and_traverse_forest(ByteScanner::new(Cursor::new(input)), (), &mut traverser)
      .unwrap();
    count = n;
  }
  let avg_ns = start.elapsed().as_nanos() / ITERATIONS as u128;
  (avg_ns, count)
}

#[test]
fn perf_compare_panda_simple() {
  let input = "a panda eats shoots and leaves.";
  let (rr_ns, rr_count) = time_run_recognizer(input, build_panda_grammar);
  let (asf_ns, asf_count) = time_asf_traverse(input, build_panda_grammar);
  println!("\n=== panda short ({} chars, {} trees) ===", input.len(), rr_count);
  println!("run_recognizer:           {:>10} ns/run  ({} trees)", rr_ns, rr_count);
  println!("parse_and_traverse_forest:{:>10} ns/run  ({} alternative-count)", asf_ns, asf_count);
  println!(
    "ratio (run_recognizer / ASF): {:.2}×",
    rr_ns as f64 / asf_ns.max(1) as f64
  );
  // Correctness gate: `CountTraverser` enumerates the same Cartesian
  // product the tree iterator unfolds, so its saturating sum must match
  // the underlying parse-tree count. A regression in symch unification
  // or in the post-order memoization would change `asf_count`.
  assert_eq!(asf_count, rr_count, "ASF parse-count must equal tree-iterator count");
}

#[test]
fn perf_compare_panda_long() {
  // Stress: "shoots and leaves" repeated 4× — VP-recursive ambiguity
  // explodes. Each VP can be parsed many ways, and the bocage
  // factors are exponentially more numerous than the visible parses.
  let input = "a panda eats shoots and leaves and shoots and leaves and shoots and leaves and shoots and leaves.";
  let (rr_ns, rr_count) = time_run_recognizer(input, build_panda_grammar);
  let (asf_ns, asf_count) = time_asf_traverse(input, build_panda_grammar);
  println!("\n=== panda long ({} chars, {} trees) ===", input.len(), rr_count);
  println!("run_recognizer:           {:>10} ns/run  ({} trees)", rr_ns, rr_count);
  println!("parse_and_traverse_forest:{:>10} ns/run  ({} alternative-count)", asf_ns, asf_count);
  println!(
    "ratio (run_recognizer / ASF): {:.2}×",
    rr_ns as f64 / asf_ns.max(1) as f64
  );
  assert_eq!(asf_count, rr_count, "ASF parse-count must equal tree-iterator count");
}

// Catalan-explosion arithmetic grammar: E → E op E | num
// With N `1+1+...+1`, there are Catalan(N-1) parses. This is the
// canonical "ambiguity explodes despite a simple grammar" case.
fn build_arith_grammar() -> Result<Parser> {
  let mut g = Grammar::new()?;
  let one = g.literal_string(None, "1")?;
  let plus = g.literal_string(None, "+")?;
  let e = g.rule(None, &[one])?;
  let _e_recursive = g.rule(Some(e), &[e, plus, e])?;
  g.set_start(e)?;
  Ok(Parser::with_grammar(g.unwrap()))
}

#[test]
fn perf_compare_arith_explosion() {
  // 8 operands → Catalan(7) = 429 distinct parses.
  let input = "1+1+1+1+1+1+1+1";
  let (rr_ns, rr_count) = time_run_recognizer(input, build_arith_grammar);
  let (asf_ns, asf_count) = time_asf_traverse(input, build_arith_grammar);
  println!("\n=== arithmetic Catalan ({}, {} operands) ===", input, input.matches('1').count());
  println!("run_recognizer:           {:>10} ns/run  ({} trees)", rr_ns, rr_count);
  println!("parse_and_traverse_forest:{:>10} ns/run  ({} alternative-count)", asf_ns, asf_count);
  println!(
    "ratio (run_recognizer / ASF): {:.2}×",
    rr_ns as f64 / asf_ns.max(1) as f64
  );
  assert_eq!(rr_count, 429, "Catalan(7) = 429 distinct parses (tree iterator)");
  assert_eq!(asf_count, 429, "ASF parse-count must match Catalan(7) = 429");
}

#[test]
fn perf_compare_arith_big_explosion() {
  // 12 operands → Catalan(11) = 58786 distinct parses.
  // Pushes Tree iteration well past anything realistic, demonstrating
  // the asymptotic advantage of ASF post-order memoization.
  let input = "1+1+1+1+1+1+1+1+1+1+1+1";
  let (rr_ns, rr_count) = time_run_recognizer(input, build_arith_grammar);
  let (asf_ns, asf_count) = time_asf_traverse(input, build_arith_grammar);
  println!("\n=== arithmetic Catalan ({}, {} operands) ===", input, input.matches('1').count());
  println!("run_recognizer:           {:>10} ns/run  ({} trees)", rr_ns, rr_count);
  println!("parse_and_traverse_forest:{:>10} ns/run  ({} alternative-count)", asf_ns, asf_count);
  println!(
    "ratio (run_recognizer / ASF): {:.2}×",
    rr_ns as f64 / asf_ns.max(1) as f64
  );
  assert_eq!(rr_count, 58786, "Catalan(11) = 58786 distinct parses (tree iterator)");
  assert_eq!(asf_count, 58786, "ASF parse-count must match Catalan(11) = 58786");
}
