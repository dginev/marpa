use crate::scanless;
use crate::scanless::R;
use crate::result::*;

struct MetaAST {
  meta_recce: R,
  top_node: bool
}

impl MetaAST {
  fn new(p_rules_source: &str) -> Result<Self> {
    let mut meta_recce = scanless::R::meta_recce();
    match meta_recce.read(p_rules_source) {
      Ok(()) => {},
      Err(e) => return err(&format!("Parse of BNF/Scanless source failed\n {:?}", e))
    };
    if let Some(ambiguity_status) = meta_recce.ambiguous() {
        return err(&format!("Parse of BNF/Scanless source failed:\n {:?}",ambiguity_status));
    }
    if let Some(value_ref) = meta_recce.value() {
      Ok(MetaAST {
        meta_recce, 
        top_node: value_ref
      })
    } else {
      err("Parse of BNF/Scanless source failed")
    }
  }
}