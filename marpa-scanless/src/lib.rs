#[macro_export]
macro_rules! grammar {
  ($source:literal) => {{
    let macrog;
    grammar!(macrog, $source);
    macrog
  }};
  ($var:ident, $source:expr) => {{
    use marpa::scanless::G;
    use marpa::error::Error;
    #[derive(CompileGrammar)]
    #[source=$source]
    struct _DummyG;
    let tmp : Result<G,Error> = this_grammar!();
    $var = tmp;
  }};
}