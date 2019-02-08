#[macro_export]
macro_rules! grammar {
  ($source:literal) => {{
    let macrog;
    grammar!(macrog, $source);
    macrog
  }};
  ($var:ident, $source:expr) => {{
    use marpa::grammar::Grammar;
    use marpa::error::Error;
    #[derive(CompileGrammar)]
    struct _DummyG;
    let tmp : Result<Grammar,Error> = this_grammar!();
    $var = tmp;
  }};
}