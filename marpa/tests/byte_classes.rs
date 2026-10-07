//! Byte classes (`Grammar::byte_set`, `byte_range`, `inverse_byte_set`) are one terminal each,
//! read through the scanner offering a byte as every class holding it: the language, the token
//! bytes the tree builder and the ASF see, the errors, and the forest size a hybrid cap reads are
//! those of the alternative rules over the member bytes that classes used to be. Each grammar here
//! is built both ways and compared.

extern crate marpa_asf as marpa;

use marpa::asf::{Glade, Traverser};
use marpa::grammar::{Grammar, Item};
use marpa::lexer::byte_scanner::*;
use marpa::parser::*;
use marpa::result::Result;
use marpa::stack::*;
use marpa::tree_builder::*;

use std::io::Cursor;

/// The bytes `from..=to` as a class, or as the alternative rules over them classes used to be.
fn byte_range(g: &mut Grammar, from: u8, to: u8, as_class: bool) -> Result<Item> {
    if as_class {
        g.byte_range(None, from, to)
    } else {
        let bytes: Vec<Item> = (from..=to).map(|b| Item::Symbol(i32::from(b))).collect();
        g.alternative(None, &bytes)
    }
}

/// Every byte but `excluded`, as a class or as the alternative rules over them.
fn inverse_byte_set(g: &mut Grammar, excluded: &[u8], as_class: bool) -> Result<Item> {
    if as_class {
        g.inverse_byte_set(None, excluded)
    } else {
        let bytes: Vec<Item> = (0..=255u8)
            .filter(|b| !excluded.contains(b))
            .map(|b| Item::Symbol(i32::from(b)))
            .collect();
        g.alternative(None, &bytes)
    }
}

/// `word ::= "id:" content+ ":" digit+ " "`, content any byte but whitespace: the shape of the
/// latexml math lexemes (`ROLE:content:index `), where `:` and the digits are also content bytes.
fn build_word(as_class: bool) -> Result<(Parser, TreeBuilder, Item)> {
    let mut g = Grammar::new()?;
    let prefix = g.literal_string(None, "id:")?;
    let sep = g.literal_string(None, ":")?;
    let content_char = inverse_byte_set(&mut g, b"\t\n\r ", as_class)?;
    let content = g.plus(None, content_char)?;
    let digit = byte_range(&mut g, b'0', b'9', as_class)?;
    let digits = g.plus(None, digit)?;
    let space = g.literal_string(None, " ")?;
    let word = g.rule(None, &[prefix, content, sep, digits, space])?;
    g.set_start(word)?;
    let mut b = TreeBuilder::new();
    b.token(word.rule());
    Ok((Parser::with_grammar(g.unwrap()), b, word))
}

const WORD: &str = "id:\u{393}x:12 ";

#[test]
fn class_bytes_roll_up_into_their_token() {
    let (mut parser, builder, word) = build_word(true).unwrap();
    let mut trees = parser.run_recognizer(ByteScanner::new(Cursor::new(WORD))).unwrap();
    let tree = proc_value(builder, trees.next().expect("one parse"));
    assert_eq!(format!("{tree}"), format!("Token({}, \"{WORD}\")", word.rule()));
    assert!(trees.next().is_none(), "the word has one parse");
}

/// Concatenates the bytes of the first factoring under each glade.
struct ByteCollector;

impl Traverser for ByteCollector {
    type ParseTree = Vec<u8>;
    type ParseState = ();

    fn traverse_glade(&mut self, glade: &mut Glade, children: &[Option<Vec<u8>>], _state: &mut ()) -> Result<Vec<u8>> {
        if glade.is_token() {
            let value = glade.token_value().expect("a token glade has a value");
            return Ok(vec![u8::try_from(value - 1).expect("a byte value")]);
        }
        let mut bytes = Vec::new();
        for ix in 0..glade.rh_length() {
            let child = glade.rh_glade_id(ix).expect("RHS position has a child glade");
            bytes.extend(children[child].as_ref().expect("child visited first"));
        }
        Ok(bytes)
    }
}

#[test]
fn asf_token_glades_carry_their_byte() {
    let (mut parser, _builder, _word) = build_word(true).unwrap();
    let (bytes, _state) = parser
        .parse_and_traverse_forest(ByteScanner::new(Cursor::new(WORD)), (), &mut ByteCollector)
        .unwrap();
    assert_eq!(String::from_utf8(bytes).unwrap(), WORD);
}

fn word_error_code(input: &str, as_class: bool) -> u32 {
    let (mut parser, _builder, _word) = build_word(as_class).unwrap();
    parser
        .run_recognizer(ByteScanner::new(Cursor::new(input)))
        .err()
        .expect("rejected")
        .get_code()
}

#[test]
fn a_byte_no_class_expects_fails_as_through_the_byte_rules() {
    // 'x' is on no rule but through the content class, which `id` does not expect yet.
    assert_eq!(word_error_code("idx:1 ", true), libmarpa_sys::MARPA_ERR_UNEXPECTED_TOKEN_ID);
    assert_eq!(word_error_code("idx:1 ", false), libmarpa_sys::MARPA_ERR_UNEXPECTED_TOKEN_ID);
}

#[test]
fn a_byte_in_no_class_fails_as_before() {
    // A tab is in no class and on no rule: its own terminal is inaccessible, both ways.
    assert_eq!(word_error_code("id:\tx:1 ", true), libmarpa_sys::MARPA_ERR_INACCESSIBLE_TOKEN);
    assert_eq!(word_error_code("id:\tx:1 ", false), libmarpa_sys::MARPA_ERR_INACCESSIBLE_TOKEN);
}

#[test]
fn a_byte_in_two_classes_keeps_both_readings() {
    let mut g = Grammar::new().unwrap();
    let digit = g.char_range(None, '0', '9').unwrap();
    let hex = g.string_set(None, "0123456789abcdef").unwrap();
    let start = g.alternative(None, &[digit, hex]).unwrap();
    g.set_start(start).unwrap();
    let mut parser = Parser::with_grammar(g.unwrap());
    let mut trees = parser.run_recognizer(ByteScanner::new(Cursor::new("7"))).unwrap();
    let mut count = 0;
    while trees.next().is_some() {
        count += 1;
    }
    assert_eq!(count, 2, "'7' is a digit and a hex digit");
}

/// "x:1" three ways: the literal "x:" then a digit; "x" then content (`:` and `1` read through
/// the content class); the literal "x:1" (every byte its own terminal). At one position a byte is
/// read as its own terminal and through one or two classes, as in the latexml lexemes.
fn build_mixed(as_class: bool) -> Result<Parser> {
    let mut g = Grammar::new()?;
    let x = g.literal_string(None, "x")?;
    let x_colon = g.literal_string(None, "x:")?;
    let x_colon_one = g.literal_string(None, "x:1")?;
    let content_char = inverse_byte_set(&mut g, b"\t\n\r ", as_class)?;
    let content = g.plus(None, content_char)?;
    let digit = byte_range(&mut g, b'0', b'9', as_class)?;
    let digits = g.plus(None, digit)?;
    let indexed = g.rule(None, &[x_colon, digits])?;
    let with_content = g.rule(None, &[x, content])?;
    let literal = g.rule(None, &[x_colon_one])?;
    let start = g.alternative(None, &[indexed, with_content, literal])?;
    g.set_start(start)?;
    Ok(Parser::with_grammar(g.unwrap()))
}

fn mixed_tree_count(as_class: bool) -> usize {
    let mut parser = build_mixed(as_class).unwrap();
    let mut trees = parser.run_recognizer(ByteScanner::new(Cursor::new("x:1"))).unwrap();
    let mut count = 0;
    while trees.next().is_some() {
        count += 1;
    }
    count
}

#[test]
fn a_mixed_forest_parses_as_through_the_byte_rules() {
    assert_eq!(mixed_tree_count(true), 3);
    assert_eq!(mixed_tree_count(false), 3);
}

/// The forest's size as the hybrid cap reads it: a cap of 0 sends any ambiguous forest down the
/// tree route, which reports it.
fn capped_stats(mut parser: Parser, input: &str) -> BocageStats {
    match parser
        .parse_hybrid_with_and_node_limit(ByteScanner::new(Cursor::new(input)), (), &mut ByteCollector, Some(0))
        .unwrap()
    {
        HybridParseResult::AmbiguousTree(_, stats) => stats,
        _ => panic!("an ambiguous forest over the cap takes the tree route"),
    }
}

#[test]
fn the_hybrid_cap_reads_the_mixed_forest_of_the_byte_rules() {
    assert_eq!(
        capped_stats(build_mixed(true).unwrap(), "x:1"),
        capped_stats(build_mixed(false).unwrap(), "x:1")
    );
}

/// `start ::= word word`, `word ::= c+`: "aab" splits two ways.
fn build_split(as_class: bool) -> Result<Parser> {
    let mut g = Grammar::new()?;
    let c = byte_range(&mut g, b'a', b'b', as_class)?;
    let word = g.plus(None, c)?;
    let start = g.rule(None, &[word, word])?;
    g.set_start(start)?;
    Ok(Parser::with_grammar(g.unwrap()))
}

#[test]
fn the_hybrid_cap_reads_a_split_forest_of_the_byte_rules() {
    assert_eq!(
        capped_stats(build_split(true).unwrap(), "aab"),
        capped_stats(build_split(false).unwrap(), "aab")
    );
}

/// Records each token glade's value, in order.
struct TokenValues;

impl Traverser for TokenValues {
    type ParseTree = Vec<Option<i32>>;
    type ParseState = ();

    fn traverse_glade(&mut self, glade: &mut Glade, children: &[Option<Vec<Option<i32>>>], _state: &mut ()) -> Result<Vec<Option<i32>>> {
        if glade.is_token() {
            return Ok(vec![glade.token_value()]);
        }
        let mut values = Vec::new();
        for ix in 0..glade.rh_length() {
            let child = glade.rh_glade_id(ix).expect("RHS position has a child glade");
            values.extend(children[child].as_ref().expect("child visited first"));
        }
        Ok(values)
    }
}

#[test]
fn a_nulling_token_glade_has_no_value() {
    // `start ::= "x" digit*`, read as "x": the digits are a nulling token glade.
    let mut g = Grammar::new().unwrap();
    let x = g.literal_string(None, "x").unwrap();
    let digit = g.char_range(None, '0', '9').unwrap();
    let digits = g.star(None, digit).unwrap();
    let start = g.rule(None, &[x, digits]).unwrap();
    g.set_start(start).unwrap();
    let mut parser = Parser::with_grammar(g.unwrap());
    let (values, _state) = parser
        .parse_and_traverse_forest(ByteScanner::new(Cursor::new("x")), (), &mut TokenValues)
        .unwrap();
    assert_eq!(values, vec![Some(i32::from(b'x') + 1), None]);
}
