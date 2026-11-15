//! libmarpa-asf-sys builds `marpa_obs.c` with a per-thread cache of freed
//! default-size obstack chunks (`libmarpa-sys/c/marpa_obs.c`). Recycled chunks
//! hold stale bytes, and each thread's cache is freed by a pthread key
//! destructor at thread exit: parse the same input many times on several
//! threads (each thread exiting with a non-empty cache) and require every
//! result to equal the first.
extern crate marpa_asf as marpa;

use marpa::grammar::Grammar;
use marpa::lexer::byte_scanner::*;
use marpa::parser::*;
use marpa::result::Result;
use marpa::stack::*;
use marpa::tree_builder::*;

use std::io::Cursor;

fn parse_once(input: &str) -> Result<String> {
    let mut g = Grammar::new()?;
    let ws_char = g.string_set(None, "\t\n\r ")?;
    let ws = g.star(None, ws_char)?;
    let sep = g.literal_string(None, "::=")?;
    let term = g.literal_string(None, ";")?;
    let dquote = g.literal_string(None, "\"")?;
    let not_dquote = g.inverse_string_set(None, "\"")?;
    let str_chars_star = g.star(None, not_dquote)?;
    let string = g.rule(None, &[dquote, str_chars_star, dquote])?;
    let lower = g.char_range(None, 'a', 'z')?;
    let ident = g.plus(None, lower)?;
    let rule = g.rule(None, &[ident, ws, sep, ws, string, ws, term])?;
    let rules = g.sequence(None, rule, ws_char, false, false)?;
    g.set_start(rules)?;

    let mut b = TreeBuilder::new();
    for r in [rules, rule].iter().map(|x| x.rule()) {
        b.rule(r);
    }
    for t in [ident, string, sep, term].iter().map(|x| x.rule()) {
        b.token(t);
    }
    for d in [ws_char].iter().map(|x| x.rule()) {
        b.discard(d);
    }
    let mut p = Parser::with_grammar(g.unwrap());
    let mut t = p.run_recognizer(ByteScanner::new(Cursor::new(input.to_owned())))?;
    let v = t.next().unwrap();
    Ok(format!("{}", proc_value(b, v)))
}

#[test]
fn recycled_chunks_parse_identically_across_threads() {
    // Long enough that the recognizer's obstack spans many chunks.
    let input: String = (0..300).map(|i| format!("r{} ::= \"v{i}\";\n", "abc".repeat(i % 7 + 1))).collect();
    let input = input.replace(|c: char| c.is_ascii_digit(), "q");
    let expected = parse_once(&input).expect("first parse");
    let handles: Vec<_> = (0..4)
        .map(|_| {
            let input = input.clone();
            std::thread::spawn(move || (0..20).map(|_| parse_once(&input).expect("parse")).collect::<Vec<_>>())
        })
        .collect();
    for h in handles {
        for got in h.join().expect("thread") {
            assert_eq!(got, expected);
        }
    }
}
