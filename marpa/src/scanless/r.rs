use crate::metag::MetaRecce;
pub struct R {
  c: bool,
  grammar: bool,
  thick_g1_recce: bool,
  p_input_string: bool,
  exhaustion_action: String,
  rejection_action: String,
  trace_file_handle: bool,
  trace_lexers: bool,
  trace_terminals: bool,
  read_string_error: bool,
  events: bool,
}

impl Default for R {
  fn default() -> Self {
    R {
        c: false,
        grammar: false,
        thick_g1_recce: false,
        p_input_string: false,
        exhaustion_action: String::new(),
        rejection_action: String::new(),
        trace_file_handle: false,
        trace_lexers: false,
        trace_terminals: false,
        read_string_error: false,
        events: false,
    }
  }
}

impl R {
  pub fn meta_recce() -> Self {
    let meta_grammar = super::G::meta_grammar();
    R::new(meta_grammar)
  }


  // sub Marpa::R2::Scanless::R::new {
  pub fn new(grammar: super::G) -> Self {
    let mut slr = R::default();
    
    // Set SLIF (not NAIF) recognizer args to default
    slr.exhaustion_action =  String::from("fatal");
    slr.rejection_action = String::from("fatal");
    slr.trace_lexers = false;
    slr.trace_terminals = false;

    // my ($g1_recce_args, $flat_args) =
    //     Marpa::R2::Internal::Scanless::R::set( $slr, "new", @args );
    // my $too_many_earley_items = $g1_recce_args->{too_many_earley_items};

    // my $slg = $slr->[Marpa::R2::Internal::Scanless::R::GRAMMAR];

    // Marpa::R2::exception(
    //     qq{Marpa::R2::Scanless::R::new() called without a "grammar" argument}
    // ) if not defined $slg;

    // my $slg_class = 'Marpa::R2::Scanless::G';
    // if ( not blessed $slg or not $slg->isa($slg_class) ) {
    //     my $ref_type = ref $slg;
    //     my $desc = $ref_type ? "a ref to $ref_type" : 'not a ref';
    //     Marpa::R2::exception(
    //         qq{'grammar' named argument to new() is $desc\n},
    //         "  It should be a ref to $slg_class\n" );
    // } //// end if ( not blessed $slg or not $slg->isa($slg_class) )

    // my $thick_g1_grammar =
    //     $slg->[Marpa::R2::Internal::Scanless::G::THICK_G1_GRAMMAR];

    // my $trace_file_handle = $g1_recce_args->{trace_file_handle};
    // $trace_file_handle //= $thick_g1_grammar->[Marpa::R2::Internal::Grammar::TRACE_FILE_HANDLE] ;

    // my $thick_g1_recce =
    //     $slr->[Marpa::R2::Internal::Scanless::R::THICK_G1_RECCE] = bless [],
    //     'Marpa::R2::Recognizer';

    // local $Marpa::R2::Internal::TRACE_FH =
    //     $thick_g1_recce->[Marpa::R2::Internal::Recognizer::TRACE_FILE_HANDLE] = $trace_file_handle;

    // $thick_g1_recce->[Marpa::R2::Internal::Recognizer::GRAMMAR] = $thick_g1_grammar;

    // my $grammar_c = $thick_g1_grammar->[Marpa::R2::Internal::Grammar::C];

    // my $recce_c = $thick_g1_recce->[Marpa::R2::Internal::Recognizer::C] =
    //     Marpa::R2::Thin::R->new($grammar_c);
    // if ( not defined $recce_c ) {
    //     Marpa::R2::exception( $grammar_c->error() );
    // }

    // $recce_c->ruby_slippers_set(1);

    // if (   defined $thick_g1_grammar->[Marpa::R2::Internal::Grammar::ACTION_OBJECT]
    //     or defined $thick_g1_grammar->[Marpa::R2::Internal::Grammar::ACTIONS]
    //     or not defined $thick_g1_grammar->[Marpa::R2::Internal::Grammar::INTERNAL] )
    // {
    //     $thick_g1_recce->[Marpa::R2::Internal::Recognizer::RESOLVE_PACKAGE_SOURCE] =
    //         'legacy';
    // } //// end if ( defined $grammar->[...])

    // if ( defined( my $value = $g1_recce_args->{'leo'} ) ) {
    //         my $boolean = $value ? 1 : 0;
    //         $thick_g1_recce->use_leo_set($boolean);
    //         delete $g1_recce_args->{leo};
    //     }

    // $thick_g1_recce->[Marpa::R2::Internal::Recognizer::WARNINGS]       = 1;
    // $thick_g1_recce->[Marpa::R2::Internal::Recognizer::RANKING_METHOD] = 'none';
    // $thick_g1_recce->[Marpa::R2::Internal::Recognizer::MAX_PARSES]     = 0;
    // $thick_g1_recce->[Marpa::R2::Internal::Recognizer::TRACE_TERMINALS]     = 0;

    // // Position 0 is not used because 0 indicates an unvalued token.
    // // Position 1 is reserved for undef.
    // // Position 2 is reserved for literal tokens (used in SLIF).
    // $thick_g1_recce->[Marpa::R2::Internal::Recognizer::TOKEN_VALUES] = [undef, undef, undef];

    // $thick_g1_recce->reset_evaluation();

    // my $thin_slr =
    //     Marpa::R2::Thin::SLR->new( $slg->[Marpa::R2::Internal::Scanless::G::C],
    //     $thick_g1_recce->thin() );
    // $thin_slr->earley_item_warning_threshold_set($too_many_earley_items)
    //     if defined $too_many_earley_items;
    // $slr->[Marpa::R2::Internal::Scanless::R::C]      = $thin_slr;
    // $slr->[Marpa::R2::Internal::Scanless::R::EVENTS] = [];

    // my $symbol_ids_by_event_name_and_type =
    //     $slg->[
    //     Marpa::R2::Internal::Scanless::G::SYMBOL_IDS_BY_EVENT_NAME_AND_TYPE];

    // my $event_is_active_arg = $flat_args->{event_is_active} // {};
    // if (ref $event_is_active_arg ne 'HASH') {
    //     Marpa::R2::exception( 'event_is_active named argument must be ref to hash' );
    // }

    // // Completion/nulled/prediction events are always initialized by
    // // Libmarpa to 'on'.  So here we need to override that if and only
    // // if we in fact want to initialize them to 'off'.

    // // Events are already initialized as described by
    // // the DSL.  Here we override that with the recce arg, if
    // // necessary.
    
    // EVENT: for my $event_name ( keys %{$event_is_active_arg} ) {

    //     my $is_active = $event_is_active_arg->{$event_name};

    //     my $symbol_ids =
    //         $symbol_ids_by_event_name_and_type->{$event_name}->{lexeme};
    //     $thin_slr->lexeme_event_activate( $_, $is_active )
    //         for @{$symbol_ids};
    //     my $lexer_rule_ids =
    //         $symbol_ids_by_event_name_and_type->{$event_name}->{discard};
    //     $thin_slr->discard_event_activate( $_, $is_active )
    //         for @{$lexer_rule_ids};

    //     $symbol_ids =
    //         $symbol_ids_by_event_name_and_type->{$event_name}->{completion}
    //         // [];
    //     $recce_c->completion_symbol_activate( $_, $is_active )
    //         for @{$symbol_ids};
    //     $symbol_ids =
    //         $symbol_ids_by_event_name_and_type->{$event_name}->{nulled} // [];
    //     $recce_c->nulled_symbol_activate( $_, $is_active ) for @{$symbol_ids};
    //     $symbol_ids =
    //         $symbol_ids_by_event_name_and_type->{$event_name}->{prediction}
    //         // [];
    //     $recce_c->prediction_symbol_activate( $_, $is_active )
    //         for @{$symbol_ids};
    // } //// end EVENT: for my $event_name ( keys %{$event_is_active_arg} )

    // if ( not $recce_c->start_input() ) {
    //     my $error = $grammar_c->error();
    //     Marpa::R2::exception( 'Recognizer start of input failed: ', $error );
    // }

    // $thick_g1_recce->set($g1_recce_args);

    // if ( $thick_g1_recce->[Marpa::R2::Internal::Recognizer::TRACE_TERMINALS] > 1 ) {
    //     my @terminals_expected = @{ $thick_g1_recce->terminals_expected() };
    //     for my $terminal ( sort @terminals_expected ) {
    //         say {$Marpa::R2::Internal::TRACE_FH}
    //             qq{Expecting "$terminal" at earleme 0}
    //             or Marpa::R2::exception("Cannot print: $ERRNO");
    //     }
    // } //// end if ( $thick_g1_recce->[Marpa::R2::Internal::Recognizer::TRACE_TERMINALS...])

    // Marpa::R2::Internal::Scanless::convert_libmarpa_events($slr);

    slr
  } //// end sub Marpa::R2::Scanless::R::new


  pub fn ambiguous(&self) -> Option<bool> { None } // TODO
  pub fn value(&self) -> Option<bool> { None } // TODO
  pub fn read(&self, input: &str) -> Result<(),()> { Ok(()) } // TODO
}