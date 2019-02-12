use crate::grammar::Grammar;
use std::collections::HashMap;

pub struct G {
  c: bool,
  thick_lex_grammars: Grammar,
  thick_g1_grammar: Grammar,
  character_class_tables: HashMap<char,String>,
  discard_event_by_lexer_rule: bool,
  mask_by_rule_id: Vec<bool>,
  default_g1_start_action: String,
  completion_event_by_id: Option<bool>,
  nulled_event_by_id: Option<bool>,
  prediction_event_by_id: Option<bool>,
  lexeme_event_by_id: Option<bool>,
  symbol_ids_by_event_name_and_type: HashMap<String, String>,
  cache_ruleids_by_lhs_name: HashMap<String, String>,
  trace_file_handle: bool,
  trace_terminals: bool,
}

impl Default for G {
  fn default() -> Self {
    G {
      c: false,
      thick_lex_grammars: Grammar::new().unwrap(),
      thick_g1_grammar: Grammar::new().unwrap(),
      character_class_tables: HashMap::new(),
      discard_event_by_lexer_rule: false,
      mask_by_rule_id: Vec::new(),
      default_g1_start_action: String::new(),
      completion_event_by_id: None,
      nulled_event_by_id: None,
      prediction_event_by_id: None,
      lexeme_event_by_id: None,
      symbol_ids_by_event_name_and_type: HashMap::new(),
      cache_ruleids_by_lhs_name: HashMap::new(),
      trace_file_handle: false,
      trace_terminals: false,
    }
  }
}

impl G {
  pub fn meta_grammar() -> Self {
    // let mut meta_slg = G::new();
    // meta_slg.trace_terminals = false;
    // // TODO: should we inline G::hash_to_runtime ? Is that a high price?
    // meta_slg.hash_to_runtime(hashed_metag);

    // let mut thick_g1_grammar = meta_slg.thick_g1_grammar;
    // let mut mask_by_rule_id = Vec::new();
    // for id in thick_g1_grammar.get_rule_ids() {
    //   mask_by_rule_id[id] = thick_g1_grammar._rule_mask(id)
    // }
    
    // meta_slg.MASK_BY_RULE_ID = mask_by_rule_id;
    // meta_slg.trace_terminals = false;

    G::default() // TODO
  }


  // sub Marpa::R2::Scanless::G::new {
  //   my ( $class, @hash_ref_args ) = @_;

  //   my $slg = [];
  //   bless $slg, $class;

  //   my ($dsl, $g1_args) = Marpa::R2::Internal::Scanless::G::set ( $slg, 'new', @hash_ref_args );
  //   my $ast = Marpa::R2::Internal::MetaAST->new( $dsl );
  //   my $hashed_ast = $ast->ast_to_hash();
  //   Marpa::R2::Internal::Scanless::G::hash_to_runtime($slg, $hashed_ast, $g1_args);
  //   return $slg;
  // } ## end sub Marpa::R2::Scanless::G::new

}

