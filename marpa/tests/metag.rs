use marpa::metag;
#[test]
fn meta_g_is_available() {
  let meta_g = metag::hashed_grammar();
  assert_eq!(meta_g.character_classes.len(), 58, "current number of character classes of MetaG expected to be 58");
  assert_eq!(meta_g.lexeme_default_adverbs.len(), 3, "current number of default adverbs of MetaG expected to be 3");
}
