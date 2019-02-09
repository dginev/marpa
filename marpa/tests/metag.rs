use marpa::metag;
#[test]
fn meta_g_is_available() {
  let meta_g = metag::new();
  assert!(meta_g.is_ok());
}
