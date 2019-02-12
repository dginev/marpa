use std::collections::HashMap;
use crate::scanless;
use crate::scanless::R;
use crate::result::*;

/// An auto-generated rule, part of the SLIF-recognizing meta grammar
pub struct MetaRecceRule {
   pub action: &'static str,
   pub bless:  &'static str,
   pub lhs:  &'static str,
   pub mask: Vec<bool>,
   pub name:  &'static str,
   pub rhs: Vec<&'static str>,
   pub min: Option<usize>,
   pub proper: &'static str,
   pub separator: &'static str,
   pub description: &'static str,
   pub symbol_as_event: &'static str,
}
impl Default for MetaRecceRule {
  fn default() -> Self {
    MetaRecceRule {
      action: "",
      bless: "",
      lhs: "",
      mask: Vec::new(),
      name: "",
      rhs: Vec::new(),
      min: None,
      proper: "",
      separator: "",
      description: "",
      symbol_as_event: "",
    }
  }
}

/// An auto-generated symbol, part of the SLIF-recognizing meta grammar
pub struct MetaRecceSymbol { 
    pub description: &'static str,
    pub display_form: &'static str,
    pub dsl_form: &'static str,
}

/// An auto-generated struct representing the SLIF-recognizing meta grammar
pub struct MetaRecce { 
    pub character_classes: HashMap<&'static str, Vec<&'static str>>,
    pub discard_default_adverbs: bool,
    pub default_adverbs: HashMap<&'static str, HashMap<&'static str, &'static str>>,
    pub first_lhs: &'static str,
    pub start_lhs: &'static str,
    pub default_g1_start_action: &'static str,
    pub lexeme_default_adverbs: HashMap<&'static str, &'static str>,
    pub rules_g1: Vec<MetaRecceRule>,
    pub rules_l0: Vec<MetaRecceRule>,
    pub symbols_g1: HashMap<&'static str, MetaRecceSymbol>,
    pub symbols_l0: HashMap<&'static str, MetaRecceSymbol>,
}
impl Default for MetaRecce {
  fn default() -> Self {
    MetaRecce {
      character_classes: HashMap::new(),
      discard_default_adverbs: false,
      first_lhs: "",
      start_lhs: "",
      lexeme_default_adverbs: HashMap::new(),
      default_adverbs: HashMap::new(),
      rules_g1: Vec::new(),
      rules_l0: Vec::new(),
      symbols_g1: HashMap::new(),
      symbols_l0: HashMap::new(),
      default_g1_start_action: ""
    }
  }
}


pub struct MetaAST {
  meta_recce: R,
  top_node: bool
}

impl MetaAST {
  pub fn new(p_rules_source: &str) -> Result<Self> {
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

  pub fn start_rule_create(parse: &mut MetaRecce, symbol_name: &'static str) {
    let start_lhs = "[:start]";
    parse.default_g1_start_action =
        parse.default_adverbs.get("G1").unwrap().get("action").unwrap();
    parse.symbols_g1.insert("start_lhs", MetaRecceSymbol {
        dsl_form: "",
        display_form: ":start",
        description: "Internal G1 start symbol",
    });
    parse.rules_g1.push(MetaRecceRule {
        lhs   : start_lhs,
        rhs   : vec![symbol_name],
        action: "::first",
        ..MetaRecceRule::default()
    });
  }
}