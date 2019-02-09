#[macro_use]
extern crate marpa_scanless;
#[macro_use]
extern crate marpa; // 2018 style macro import

use marpa::metag;

#[test]
fn meta_g_is_available() {
  let meta_g = metag::new();
  assert!(meta_g.is_ok());
}

#[test] 
fn test_simple_arith_grammar() {
let grammar_result = grammar!(r###"
:default ::= action => [name,values]
lexeme default = latm => 1
 
Calculator ::= Expression action => ::first
 
Factor ::= Number action => ::first
Term ::=
    Term '*' Factor action => do_multiply
    | Factor action => ::first
Expression ::=
    Expression '+' Term action => do_add
    | Term action => ::first
Number ~ digits
digits ~ [\d]+
:discard ~ whitespace
whitespace ~ [\s]+
"###);

  assert!(grammar_result.is_ok());
  // println!("---- RESULT IS : {:?}", grammar_result);
  // let input = "42 * 1 + 7";
  // let value_ref = grammar.parse( input, my_actions );
}