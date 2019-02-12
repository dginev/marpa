extern crate proc_macro;
use std::collections::HashMap;
use proc_macro2::TokenStream;
use quote::ToTokens;
use quote::quote;
use crate::metag;
use crate::meta_ast::MetaRecce;
use crate::grammar::Grammar;
use crate::meta_ast::MetaAST;
use crate::result::*;

#[derive(Debug)]
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
    let mut meta_slg = G::default();
    let mut hashed_metag = metag::hashed_grammar();
    meta_slg.trace_terminals = false;
    // TODO: should we inline G::hash_to_runtime ? Is that a high price?
    meta_slg.hash_to_runtime(&mut hashed_metag); // , map!("bless_package"=>"MetaAST_Nodes" ???

    // let mut thick_g1_grammar = meta_slg.thick_g1_grammar;
    // let mut mask_by_rule_id = Vec::new();
    // for id in thick_g1_grammar.get_rule_ids() {
    //   mask_by_rule_id[id] = thick_g1_grammar._rule_mask(id)
    // }
    
    // meta_slg.MASK_BY_RULE_ID = mask_by_rule_id;
    // meta_slg.trace_terminals = false;
    meta_slg  
  }


  // sub Marpa::R2::Scanless::G::new {
  pub fn new(source: &str) -> Self {
    G::default() // TODO
  }
  //   my ( $class, @hash_ref_args ) = @_;

  //   my $slg = [];
  //   bless $slg, $class;

  //   my ($dsl, $g1_args) = Marpa::R2::Internal::Scanless::G::set ( $slg, 'new', @hash_ref_args );
  //   my $ast = Marpa::R2::Internal::MetaAST->new( $dsl );
  //   my $hashed_ast = $ast->ast_to_hash();
  //   Marpa::R2::Internal::Scanless::G::hash_to_runtime($slg, $hashed_ast, $g1_args);
  //   return $slg;
  // } ## end sub Marpa::R2::Scanless::G::new



pub fn hash_to_runtime(&mut self, mut hashed_source: &mut MetaRecce) -> Result<()> { 
  let trace_terminals = self.trace_terminals;
  //     # Pre-lexer G1 processing
  let start_lhs = hashed_source.start_lhs;
  if start_lhs.is_empty() {
    return err("No rules in SLIF grammar")
  }
  MetaAST::start_rule_create(&mut hashed_source, start_lhs);

//     self.cache_ruleids_by_lhs_name = {};
//     self.default_g1_start_action =
//         $hashed_source->{'default_g1_start_action'};

  // let trace_fh =
//         self.trace_file_handle =
//         $g1_args->{trace_file_handle} // \*STDERR;

  // let if_inaccessible_default =
//         $hashed_source->{defaults}->{if_inaccessible} // 'warn';

//     # Prepare the arguments for the G1 grammar
//     $g1_args->{rules}   = $hashed_source->{rules}->{G1};
//     $g1_args->{symbols} = $hashed_source->{symbols}->{G1};
//     state $g1_target_symbol = '[:start]';
//     $g1_args->{start} = $g1_target_symbol;
//     $g1_args->{'_internal_'} =
//         { 'if_inaccessible' => $if_inaccessible_default };

  // let thick_g1_grammar = Marpa::R2::Grammar->new($g1_args);
  // let g1_tracer        = $thick_g1_grammar->tracer();
  // let g1_thin          = $g1_tracer->grammar();

  // let symbol_ids_by_event_name_and_type = {};
//     self.[
//         Marpa::R2::Internal::Scanless::G::SYMBOL_IDS_BY_EVENT_NAME_AND_TYPE]
//         = $symbol_ids_by_event_name_and_type;

  // let completion_events_by_name = $hashed_source->{completion_events};
  // let completion_events_by_id =
//         self.completion_event_by_id = [];
//     for my $symbol_name ( keys %{$completion_events_by_name} ) {
//         my ( $event_name, $is_active ) =
//             @{ $completion_events_by_name->{$symbol_name} };
//         my $symbol_id = $g1_tracer->symbol_by_name($symbol_name);
//         if ( not defined $symbol_id ) {
//             Marpa::R2::exception(
//                 "Completion event defined for non-existent symbol: $symbol_name\n"
//             );
//         }

//         # Must be done before precomputation
//         $g1_thin->symbol_is_completion_event_set( $symbol_id, 1 );
//         $g1_thin->completion_symbol_activate( $symbol_id, 0 )
//             if not $is_active;
//         self.completion_event_by_id
//             ->[$symbol_id] = $event_name;
//         push
//             @{ $symbol_ids_by_event_name_and_type->{$event_name}->{completion}
//             }, $symbol_id;
//     } ## end for my $symbol_name ( keys %{$completion_events_by_name...})

  // let nulled_events_by_name = $hashed_source->{nulled_events};
  // let nulled_events_by_id =
//         self.nulled_event_by_id = [];
//     for my $symbol_name ( keys %{$nulled_events_by_name} ) {
//         my ( $event_name, $is_active ) =
//             @{ $nulled_events_by_name->{$symbol_name} };
//         my $symbol_id = $g1_tracer->symbol_by_name($symbol_name);
//         if ( not defined $symbol_id ) {
//             Marpa::R2::exception(
//                 "nulled event defined for non-existent symbol: $symbol_name\n"
//             );
//         }

//         # Must be done before precomputation
//         $g1_thin->symbol_is_nulled_event_set( $symbol_id, 1 );
//         $g1_thin->nulled_symbol_activate( $symbol_id, 0 ) if not $is_active;
//         self.nulled_event_by_id
//             ->[$symbol_id] = $event_name;
//         push @{ $symbol_ids_by_event_name_and_type->{$event_name}->{nulled} },
//             $symbol_id;
//     } ## end for my $symbol_name ( keys %{$nulled_events_by_name} )

  // let prediction_events_by_name = $hashed_source->{prediction_events};
  // let prediction_events_by_id =
//         self.prediction_event_by_id = [];
//     for my $symbol_name ( keys %{$prediction_events_by_name} ) {
//         my ( $event_name, $is_active ) =
//             @{ $prediction_events_by_name->{$symbol_name} };
//         my $symbol_id = $g1_tracer->symbol_by_name($symbol_name);
//         if ( not defined $symbol_id ) {
//             Marpa::R2::exception(
//                 "prediction event defined for non-existent symbol: $symbol_name\n"
//             );
//         }

//         # Must be done before precomputation
//         $g1_thin->symbol_is_prediction_event_set( $symbol_id, 1 );
//         $g1_thin->prediction_symbol_activate( $symbol_id, 0 )
//             if not $is_active;
//         self.prediction_event_by_id
//             ->[$symbol_id] = $event_name;
//         push
//             @{ $symbol_ids_by_event_name_and_type->{$event_name}->{prediction}
//             }, $symbol_id;
//     } ## end for my $symbol_name ( keys %{$prediction_events_by_name...})

  // let lexeme_events_by_id =
//         self.lexeme_event_by_id = [];

//     if (defined(
//             my $precompute_error =
//                 Marpa::R2::Internal::Grammar::slif_precompute(
//                 $thick_g1_grammar)
//         )
//         )
//     {
//         if ( $precompute_error == $Marpa::R2::Error::UNPRODUCTIVE_START ) {

//             # Maybe someday improve this by finding the start rule and showing
//             # its RHS -- for now it is clear enough
//             Marpa::R2::exception(qq{Unproductive start symbol});
//         } ## end if ( $precompute_error == ...)
//         Marpa::R2::exception(
//             'Internal errror: unnkown precompute error code ',
//             $precompute_error );
//     } ## end if ( defined( my $precompute_error = ...))

//     # Find out the list of lexemes according to G1
//     my %g1_id_by_lexeme_name = ();
//     SYMBOL: for my $symbol_id ( 0 .. $g1_thin->highest_symbol_id() ) {

//         # Not a lexeme, according to G1
//         next SYMBOL if not $g1_thin->symbol_is_terminal($symbol_id);

//         my $symbol_name = $g1_tracer->symbol_name($symbol_id);
//         $g1_id_by_lexeme_name{$symbol_name} = $symbol_id;

//     } ## end SYMBOL: for my $symbol_id ( 0 .. $g1_thin->highest_symbol_id(...))

//     # A first phase of applying defaults
  // let discard_default_adverbs = $hashed_source->{discard_default_adverbs};
  // let lexeme_declarations     = $hashed_source->{lexeme_declarations};
  // let lexeme_default_adverbs  = $hashed_source->{lexeme_default_adverbs};
  // let latm_default_value      = $lexeme_default_adverbs->{latm} // 0;

//     # Current lexeme data is spread out in many places.
//     # Change so that it all resides in this hash, indexed by
//     # name
//     my %lexeme_data = ();

//     # Determine "latm" status
//     LEXEME: for my $lexeme_name ( keys %g1_id_by_lexeme_name ) {
//         my $declarations = $lexeme_declarations->{$lexeme_name};
//         my $latm_value = $declarations->{latm} // $latm_default_value;
//         $lexeme_data{$lexeme_name}{latm} = $latm_value;
//     }

//     # Lexers

  // let lexer_id   = 0;
  // let lexer_name = 'L0';

//     my %lexer_id_by_name                    = ();
//     my %thick_grammar_by_lexer_name         = ();
//     my @discard_event_by_lexer_rule_id      = ();
//     my %lexer_and_rule_to_g1_lexeme         = ();
//     my %character_class_table_by_lexer_name = ();
//     state $lex_start_symbol_name = '[:start_lex]';
//     state $discard_symbol_name   = '[:discard]';

  // let lexer_rules = $hashed_source->{rules}->{$lexer_name};
  // let character_class_hash = $hashed_source->{character_classes};
  // let lexer_symbols = $hashed_source->{symbols}->{'L'};

//     # If no lexer rules, fake a lexer
//     # Fake a lexer -- it discards symbols in character classes which
//     # never matches
//     if ( not $lexer_rules ) {
//         $character_class_hash = { '[[^\\d\\D]]' => [ '[^\\d\\D]', '' ] };
//         $lexer_rules = [
//             {   'rhs'         => [ '[[^\\d\\D]]' ],
//                 'lhs'         => '[:discard]',
//                 'symbol_as_event' => '[^\\d\\D]',
//                 'description' => 'Discard rule for <[[^\\d\\D]]>'
//             },
//         ];
//         $lexer_symbols = {
//             '[:discard]' => {
//                 'display_form' => ':discard',
//                 'description'  => 'Internal LHS for lexer "L0" discard'
//             },
//             '[[^\\d\\D]]' => {
//                 'dsl_form'     => '[^\\d\\D]',
//                 'display_form' => '[^\\d\\D]',
//                 'description'  => 'Character class: [^\\d\\D]'
//             }
//         };
//     } ## end if ( not $lexer_rules )

//     my %lex_lhs           = ();
//     my %lex_rhs           = ();
//     my %lex_separator     = ();
//     my %lexer_rule_by_tag = ();

  // let rule_tag = 'rule0';
//     for my $lex_rule ( @{$lexer_rules} ) {
//         $lex_rule->{tag} = ++$rule_tag;
//         my %lex_rule_copy = %{$lex_rule};
//         $lexer_rule_by_tag{$rule_tag} = \%lex_rule_copy;
//         delete $lex_rule->{event};
//         delete $lex_rule->{symbol_as_event};
//         $lex_lhs{ $lex_rule->{lhs} } = 1;
//         $lex_rhs{$_} = 1 for @{ $lex_rule->{rhs} };
//         if ( defined( my $separator = $lex_rule->{separator} ) ) {
//             $lex_separator{$separator} = 1;
//         }
//     } ## end for my $lex_rule ( @{$lexer_rules} )

//     my %this_lexer_symbols = ();
//     SYMBOL:
//     for my $symbol_name ( ( keys %lex_lhs ), ( keys %lex_rhs ),
//         ( keys %lex_separator ) )
//     {
//         my $symbol_data = $lexer_symbols->{$symbol_name};
//         $this_lexer_symbols{$symbol_name} = $symbol_data
//             if defined $symbol_data;
//     } ## end SYMBOL: for my $symbol_name ( ( keys %lex_lhs ), ( keys %lex_rhs...))

//     my %is_lexeme_in_this_lexer = map { $_ => 1 }
//         grep { not $lex_rhs{$_} and not $lex_separator{$_} }
//         keys %lex_lhs;

//     my @lex_lexeme_names = keys %is_lexeme_in_this_lexer;

//     Marpa::R2::exception( "No lexemes in lexer: $lexer_name\n",
//         "  An SLIF grammar must have at least one lexeme\n" )
//         if not scalar @lex_lexeme_names;

//     # Do I need this?
//     my @unproductive =
//         map {"<$_>"}
//         grep { not $lex_lhs{$_} and not $_ =~ /\A \[\[ /xms }
//         ( keys %lex_rhs, keys %lex_separator );
//     if (@unproductive) {
//         Marpa::R2::exception( 'Unproductive lexical symbols: ',
//             join q{ }, @unproductive );
//     }

//     $this_lexer_symbols{$lex_start_symbol_name}->{display_form} =
//         ':start_lex';
//     $this_lexer_symbols{$lex_start_symbol_name}->{description} =
//         'Internal L0 (lexical) start symbol';
//     push @{$lexer_rules}, map {
//         ;
//         {   description => "Internal lexical start rule for <$_>",
//             lhs         => $lex_start_symbol_name,
//             rhs         => [$_]
//         }
//     } sort keys %is_lexeme_in_this_lexer;

//     # Prepare the arguments for the lex grammar
//     my %lex_args = ();
//     $lex_args{trace_file_handle} = $trace_fh;
//     $lex_args{start}             = $lex_start_symbol_name;
//     $lex_args{'_internal_'} =
//         { 'if_inaccessible' => $if_inaccessible_default };
//     $lex_args{rules}   = $lexer_rules;
//     $lex_args{symbols} = \%this_lexer_symbols;

//     # Create the thick lex grammar
  // let lex_grammar = Marpa::R2::Grammar->new( \%lex_args );
//     $thick_grammar_by_lexer_name{$lexer_name} = $lex_grammar;
  // let lex_tracer = $lex_grammar->tracer();
  // let lex_thin   = $lex_tracer->grammar();

  // let lex_discard_symbol_id =
//         $lex_tracer->symbol_by_name($discard_symbol_name) // -1;
//     my @lex_lexeme_to_g1_symbol;
//     $lex_lexeme_to_g1_symbol[$_] = -1 for 0 .. $g1_thin->highest_symbol_id();

//     LEXEME_NAME: for my $lexeme_name (@lex_lexeme_names) {
//         next LEXEME_NAME if $lexeme_name eq $discard_symbol_name;
//         next LEXEME_NAME if $lexeme_name eq $lex_start_symbol_name;
//         my $g1_symbol_id = $g1_id_by_lexeme_name{$lexeme_name};
//         if ( not defined $g1_symbol_id ) {
//             Marpa::R2::exception(
//                 qq{<$lexeme_name> is a lexeme but it is not a legal lexeme in G1:\n},
//                 qq{   Lexemes must be G1 symbols that do not appear on a G1 LHS.\n}
//             );
//         }
//         if ( not $g1_thin->symbol_is_accessible($g1_symbol_id) ) {
//             my $message =
//                 "A lexeme in lexer $lexer_name is not accessible from the G1 start symbol: $lexeme_name";
//             say {$trace_fh} $message
//                 if $if_inaccessible_default eq 'warn';
//             Marpa::R2::exception($message)
//                 if $if_inaccessible_default eq 'fatal';
//         } ## end if ( not $g1_thin->symbol_is_accessible($g1_symbol_id...))
//         my $lex_symbol_id = $lex_tracer->symbol_by_name($lexeme_name);
//         $lexeme_data{$lexeme_name}{lexers}{$lexer_name}{'id'} =
//             $lex_symbol_id;
//         $lex_lexeme_to_g1_symbol[$lex_symbol_id] = $g1_symbol_id;
//     } ## end LEXEME_NAME: for my $lexeme_name (@lex_lexeme_names)

//     my @lex_rule_to_g1_lexeme;
  // let lex_start_symbol_id =
//         $lex_tracer->symbol_by_name($lex_start_symbol_name);
//     RULE_ID: for my $rule_id ( 0 .. $lex_thin->highest_rule_id() ) {
//         my $lhs_id = $lex_thin->rule_lhs($rule_id);
//         if ( $lhs_id == $lex_discard_symbol_id ) {
//             $lex_rule_to_g1_lexeme[$rule_id] = -2;
//             next RULE_ID;
//         }
//         if ( $lhs_id != $lex_start_symbol_id ) {
//             $lex_rule_to_g1_lexeme[$rule_id] = -1;
//             next RULE_ID;
//         }
//         my $lexer_lexeme_id = $lex_thin->rule_rhs( $rule_id, 0 );
//         if ( $lexer_lexeme_id == $lex_discard_symbol_id ) {
//             $lex_rule_to_g1_lexeme[$rule_id] = -1;
//             next RULE_ID;
//         }
//         my $lexeme_id = $lex_lexeme_to_g1_symbol[$lexer_lexeme_id] // -1;
//         $lex_rule_to_g1_lexeme[$rule_id] = $lexeme_id;
//         next RULE_ID if $lexeme_id < 0;
//         my $lexeme_name = $g1_tracer->symbol_name($lexeme_id);

//         # If 1 is the default, we don't need an assertion
//         next RULE_ID if not $lexeme_data{$lexeme_name}{latm};

//         my $assertion_id =
//             $lexeme_data{$lexeme_name}{lexers}{$lexer_name}{'assertion'};
//         if ( not defined $assertion_id ) {
//             $assertion_id = $lex_thin->zwa_new(0);

//             if ( $trace_terminals >= 2 ) {
//                 say {$trace_fh} "Assertion $assertion_id defaults to 0";
//             }

//             $lexeme_data{$lexeme_name}{lexers}{$lexer_name}{'assertion'} =
//                 $assertion_id;
//         } ## end if ( not defined $assertion_id )
//         $lex_thin->zwa_place( $assertion_id, $rule_id, 0 );
//         if ( $trace_terminals >= 2 ) {
//             say {$trace_fh}
//                 "Assertion $assertion_id applied to $lexer_name rule ",
//                 slg_rule_show( $slg, $rule_id, $lex_grammar );
//         }
//     } ## end RULE_ID: for my $rule_id ( 0 .. $lex_thin->highest_rule_id() )

//     Marpa::R2::Internal::Grammar::slif_precompute($lex_grammar);

//     my @class_table          = ();

//     CLASS_SYMBOL:
//     for my $class_symbol ( sort keys %{$character_class_hash} ) {
//         my $symbol_id = $lex_tracer->symbol_by_name($class_symbol);
//         next CLASS_SYMBOL if not defined $symbol_id;
//         my $cc_components = $character_class_hash->{$class_symbol};
//         my ( $compiled_re, $error ) =
//             Marpa::R2::Internal::MetaAST::char_class_to_re($cc_components);
//         if ( not $compiled_re ) {
//             $error =~ s/^/  /gxms;    #indent all lines
//             Marpa::R2::exception(
//                 "Failed belatedly to evaluate character class\n", $error );
//         }
//         push @class_table, [ $symbol_id, $compiled_re ];
//     } ## end CLASS_SYMBOL: for my $class_symbol ( sort keys %{...})
//     $character_class_table_by_lexer_name{$lexer_name} = \@class_table;

//     $lexer_and_rule_to_g1_lexeme{$lexer_name} = \@lex_rule_to_g1_lexeme;

//     # Apply defaults to determine the discard event for every
//     # rule id of the lexer.

  // let default_discard_event = $discard_default_adverbs->{event};
//     RULE_ID: for my $rule_id ( 0 .. $lex_thin->highest_rule_id() ) {
//         my $tag = $lex_grammar->tag($rule_id);
//         next RULE_ID if not defined $tag;
//         my $event;
//         FIND_EVENT: {
//             $event = $lexer_rule_by_tag{$tag}->{event};
//             last FIND_EVENT if defined $event;
//             my $lhs_id = $lex_thin->rule_lhs($rule_id);
//             last FIND_EVENT if $lhs_id != $lex_discard_symbol_id;
//             $event = $default_discard_event;
//         } ## end FIND_EVENT:
//         next RULE_ID if not defined $event;

//         my ( $event_name, $event_starts_active ) = @{$event};
//         if ( $event_name eq q{'symbol} ) {
//             my @event = (
//                 $lexer_rule_by_tag{$tag}->{symbol_as_event},
//                 $event_starts_active
//             );
//             $discard_event_by_lexer_rule_id[$rule_id] = \@event;
//             next RULE_ID;
//         } ## end if ( $event_name eq q{'symbol} )
//         if ( ( substr $event_name, 0, 1 ) ne q{'} ) {
//             $discard_event_by_lexer_rule_id[$rule_id] = $event;
//             next RULE_ID;
//         }
//         Marpa::R2::exception(
//             qq{Discard event has unknown name: "$event_name"}
//         );

//     } ## end RULE_ID: for my $rule_id ( 0 .. $lex_thin->highest_rule_id() )

//     # Post-lexer G1 processing

  // let thick_L0 = $thick_grammar_by_lexer_name{'L0'};
  // let thin_L0  = $thick_L0->[Marpa::R2::Internal::Grammar::C];
  // let thin_slg = self.c =
//         Marpa::R2::Thin::SLG->new( $thin_L0, $g1_tracer->grammar() );

//     # Relies on default lexer being given number zero
//     $lexer_id_by_name{'L0'} = 0;

//     LEXEME: for my $lexeme_name ( keys %g1_id_by_lexeme_name ) {
//         Marpa::R2::exception(
//             "A lexeme in G1 is not a lexeme in any of the lexers: $lexeme_name"
//         ) if not defined $lexeme_data{$lexeme_name}{'lexers'};
//     }

//     # At this point we know which symbols are lexemes.
//     # So now let's check for inconsistencies

//     # Check for lexeme declarations for things which are not lexemes
//     for my $lexeme_name ( keys %{$lexeme_declarations} ) {
//         Marpa::R2::exception(
//             "Symbol <$lexeme_name> is declared as a lexeme, but it is not used as one.\n"
//         ) if not defined $g1_id_by_lexeme_name{$lexeme_name};
//     }

//     # Now that we know the lexemes, check attempts to defined a
//     # completion or a nulled event for one
//     for my $symbol_name ( keys %{$completion_events_by_name} ) {
//         Marpa::R2::exception(
//             "A completion event is declared for <$symbol_name>, but it is a lexeme.\n",
//             "  Completion events are only valid for symbols on the LHS of G1 rules.\n"
//         ) if defined $g1_id_by_lexeme_name{$symbol_name};
//     } ## end for my $symbol_name ( keys %{$completion_events_by_name...})

//     for my $symbol_name ( keys %{$nulled_events_by_name} ) {
//         Marpa::R2::exception(
//             "A nulled event is declared for <$symbol_name>, but it is a G1 lexeme.\n",
//             "  nulled events are only valid for symbols on the LHS of G1 rules.\n"
//         ) if defined $g1_id_by_lexeme_name{$symbol_name};
//     } ## end for my $symbol_name ( keys %{$nulled_events_by_name} )

//     # Mark the lexemes, and set their data
//     # Now that we have created the SLG, we can set the latm value,
//     # already determined above.
//     LEXEME: for my $lexeme_name ( keys %g1_id_by_lexeme_name ) {
//         my $g1_lexeme_id = $g1_id_by_lexeme_name{$lexeme_name};
//         my $declarations = $lexeme_declarations->{$lexeme_name};
//         my $priority     = $declarations->{priority} // 0;
//         $thin_slg->g1_lexeme_set( $g1_lexeme_id, $priority );
//         my $latm_value = $lexeme_data{$lexeme_name}{latm} // 0;
//         $thin_slg->g1_lexeme_latm_set( $g1_lexeme_id, $latm_value );
//         my $pause_value = $declarations->{pause};
//         if ( defined $pause_value ) {
//             $thin_slg->g1_lexeme_pause_set( $g1_lexeme_id, $pause_value );
//             my $is_active = 1;

//             if ( defined( my $event_data = $declarations->{'event'} ) ) {
//                 my $event_name;
//                 ( $event_name, $is_active ) = @{$event_data};
//                 $lexeme_events_by_id->[$g1_lexeme_id] = $event_name;
//                 push @{ $symbol_ids_by_event_name_and_type->{$event_name}
//                         ->{lexeme} }, $g1_lexeme_id;
//             } ## end if ( defined( my $event_data = $declarations->{'event'...}))

//             $thin_slg->g1_lexeme_pause_activate( $g1_lexeme_id, $is_active );
//         } ## end if ( defined $pause_value )

//     } ## end LEXEME: for my $lexeme_name ( keys %g1_id_by_lexeme_name )

//     # Second phase of lexer processing
  // let lexer_rule_to_g1_lexeme = $lexer_and_rule_to_g1_lexeme{$lexer_name};

//     RULE_ID: for my $lexer_rule_id ( 0 .. $#{$lexer_rule_to_g1_lexeme} ) {
//         my $g1_lexeme_id = $lexer_rule_to_g1_lexeme->[$lexer_rule_id];
//         my $lexeme_name  = $g1_tracer->symbol_name($g1_lexeme_id);
//         my $assertion_id =
//             $lexeme_data{$lexeme_name}{lexers}{$lexer_name}{'assertion'}
//             // -1;
//         $thin_slg->lexer_rule_to_g1_lexeme_set( $lexer_rule_id,
//             $g1_lexeme_id, $assertion_id );
//         my $discard_event = $discard_event_by_lexer_rule_id[$lexer_rule_id];
//         if ( defined $discard_event ) {
//             my ( $event_name, $is_active ) = @{$discard_event};
//             self.[
//                 Marpa::R2::Internal::Scanless::G::DISCARD_EVENT_BY_LEXER_RULE
//             ]->[$lexer_rule_id] = $event_name;
//             push @{ $symbol_ids_by_event_name_and_type->{$event_name}
//                     ->{discard} }, $lexer_rule_id;
//             $thin_slg->discard_event_set( $lexer_rule_id, 1 );
//             $thin_slg->discard_event_activate( $lexer_rule_id, 1 )
//                 if $is_active;
//         } ## end if ( defined $discard_event )
//     } ## end RULE_ID: for my $lexer_rule_id ( 0 .. $#{$lexer_rule_to_g1_lexeme...})

//     # Second phase of G1 processing

//     $thin_slg->precompute();
//     self.thick_g1_grammar =
//         $thick_g1_grammar;

//     # More lexer processing
//     # Determine events by lexer rule, applying the defaults

//     {
//         my $character_class_table =
//             $character_class_table_by_lexer_name{$lexer_name};
//         self.character_class_tables
//             ->[$lexer_id] = $character_class_table;
//         self.thick_lex_grammars
//             ->[$lexer_id] = $thick_grammar_by_lexer_name{$lexer_name};
//     }

//     # This section violates the NAIF interface, directly changing some
//     # of its internal structures.
//     #
//     # Some lexeme default adverbs are applied in earlier phases.
//     #
//     APPLY_DEFAULT_LEXEME_ADVERBS: {
//         last APPLY_DEFAULT_LEXEME_ADVERBS if not $lexeme_default_adverbs;

//         my $action = $lexeme_default_adverbs->{action};
//         my $g1_symbols =
//             $thick_g1_grammar->[Marpa::R2::Internal::Grammar::SYMBOLS];
//         LEXEME:
//         for my $lexeme_name ( keys %g1_id_by_lexeme_name ) {
//             my $g1_lexeme_id = $g1_id_by_lexeme_name{$lexeme_name};
//             my $g1_symbol    = $g1_symbols->[$g1_lexeme_id];
//             next LEXEME if $lexeme_name =~ m/ \] \z/xms;
//             $g1_symbol->[Marpa::R2::Internal::Symbol::LEXEME_SEMANTICS] //=
//                 $action;
//         } ## end LEXEME: for my $lexeme_name ( keys %g1_id_by_lexeme_name )

//         my $blessing = $lexeme_default_adverbs->{bless};
//         last APPLY_DEFAULT_LEXEME_ADVERBS if not $blessing;
//         last APPLY_DEFAULT_LEXEME_ADVERBS if $blessing eq '::undef';

//         LEXEME:
//         for my $lexeme_name ( keys %g1_id_by_lexeme_name ) {
//             my $g1_lexeme_id = $g1_id_by_lexeme_name{$lexeme_name};
//             my $g1_symbol    = $g1_symbols->[$g1_lexeme_id];
//             next LEXEME if $lexeme_name =~ m/ \] \z/xms;
//             if ( $blessing eq '::name' ) {
//                 if ( $lexeme_name =~ / [^ [:alnum:]] /xms ) {
//                     Marpa::R2::exception(
//                         qq{Lexeme blessing by '::name' only allowed if lexeme name is whitespace and alphanumerics\n},
//                         qq{   Problematic lexeme was <$lexeme_name>\n}
//                     );
//                 } ## end if ( $lexeme_name =~ / [^ [:alnum:]] /xms )
//                 my $blessing_by_name = $lexeme_name;
//                 $blessing_by_name =~ s/[ ]/_/gxms;
//                 $g1_symbol->[Marpa::R2::Internal::Symbol::BLESSING] //=
//                     $blessing_by_name;
//                 next LEXEME;
//             } ## end if ( $blessing eq '::name' )
//             if ( $blessing =~ / [\W] /xms ) {
//                 Marpa::R2::exception(
//                     qq{Blessing lexeme as '$blessing' is not allowed\n},
//                     qq{   Problematic lexeme was <$lexeme_name>\n}
//                 );
//             } ## end if ( $blessing =~ / [\W] /xms )
//             $g1_symbol->[Marpa::R2::Internal::Symbol::BLESSING] //= $blessing;
//         } ## end LEXEME: for my $lexeme_name ( keys %g1_id_by_lexeme_name )

//     } ## end APPLY_DEFAULT_LEXEME_ADVERBS:
  Ok(())
} // G::hash_to_runtime


}

// Crucial for SLIF precompilation
impl ToTokens for G {
  fn to_tokens(&self, stream: &mut TokenStream) {
    stream.extend(quote!(marpa::scanless::G::default()))
  }
}