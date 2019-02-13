use crate::scanless::G;
use crate::result::*;

pub struct R {
  pub c: bool,
  pub grammar: G,
  pub thick_g1_recce: bool,
  pub p_input_string: bool,
  pub exhaustion_action: String,
  pub rejection_action: String,
  pub trace_file_handle: bool,
  pub trace_lexers: bool,
  pub trace_terminals: bool,
  pub read_string_error: bool,
  pub events: bool,
}

impl Default for R {
  fn default() -> Self {
    R {
        c: false,
        grammar: G::default(),
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

struct CommonNaifRecceArgs {
  end: Option<bool>,
  max_parses:Option<bool>,
  semantics_package:Option<bool>,
  too_many_earley_items:Option<bool>,
  trace_actions: Option<bool>,
  trace_file_handle: Option<bool>,
  trace_terminals:Option<bool>,
  trace_values:Option<bool>,
}
impl Default for CommonNaifRecceArgs {
  fn default() -> Self {
    CommonNaifRecceArgs { 
      end: None,
      max_parses: None,
      semantics_package: None,
      too_many_earley_items: None,
      trace_actions: None,
      trace_file_handle: None,
      trace_terminals: None,
      trace_values: None,
    }
  }
}
struct CommonSlifRecceArgs {
  trace_lexers: Option<bool>,
  rejection: Option<bool>,
  exhaustion: Option<bool>
}
impl Default for CommonSlifRecceArgs {
  fn default() -> Self {
    CommonSlifRecceArgs {
      trace_lexers: None,
      rejection: None,
      exhaustion: None
    }
  }
}

struct SetMethodArgs {
  slif: CommonSlifRecceArgs,
  naif: CommonNaifRecceArgs
}
struct NewMethodArgs {
  grammar: Option<G>,
  ranking_method: Option<bool>,
  event_is_active: Option<bool>,
  method_args: SetMethodArgs
}
type SeriesRestartMethodArgs = SetMethodArgs;

impl R {
  pub fn meta_recce() -> Result<Self> {
    let meta_grammar = super::G::meta_grammar()?;
    Ok(R::new(meta_grammar))
  }


  // sub Marpa::R2::Scanless::R::new {
  pub fn new(grammar: super::G) -> Self {
    let mut slr = R::default();
    
    // Set SLIF (not NAIF) recognizer args to default
    slr.exhaustion_action =  String::from("fatal");
    slr.rejection_action = String::from("fatal");
    slr.trace_lexers = false;
    slr.trace_terminals = false;

    let (g1_recce_args, flat_args) =
        slr.internal_set("new", grammar);
    let too_many_earley_items = g1_recce_args.too_many_earley_items;

    let slg = &mut slr.grammar;

    let thick_g1_grammar = &slg.thick_g1_grammar;

    let trace_file_handle = &g1_recce_args.trace_file_handle;
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

  // The context flag indicates whether this set is called directly by the user
  // or is for series reset or the constructor.  "Context" flags of this kind
  // are much decried practice, and for good reason, but in this case
  // I think it is justified.
  // This logic really needs to be all in one place, and so a flag
  // to trigger the minor differences needed by the various calling
  // contexts is a small price to pay.
  fn internal_set(&mut self, method: &str, hash_ref_args: G) -> (CommonNaifRecceArgs,CommonNaifRecceArgs) {
    // These NAIF recce args are allowed in all contexts
    
    // for my $args (@hash_ref_args) {
    //     my $ref_type = ref $args;
    //     if ( not $ref_type ) {
    //         Marpa::R2::exception( q{$slr->}
    //                 . $method
    //                 . qq{() expects args as ref to HASH; got non-reference instead}
    //         );
    //     } //// end if ( not $ref_type )
    //     if ( $ref_type ne 'HASH' ) {
    //         Marpa::R2::exception( q{$slr->}
    //                 . $method
    //                 . qq{() expects args as ref to HASH, got ref to $ref_type instead}
    //         );
    //     } //// end if ( $ref_type ne 'HASH' )
    // } //// end for my $args (@hash_ref_args)

    let flat_args = CommonNaifRecceArgs::default();
    // for my $hash_ref (@hash_ref_args) {
    //     ARG: for my $arg_name ( keys %{$hash_ref} ) {
    //         $flat_args{$arg_name} = $hash_ref->{$arg_name};
    //     }
    // }
    // my $ok_args = $set_method_args;
    // $ok_args = $new_method_args            if $method eq 'new';
    // $ok_args = $series_restart_method_args if $method eq 'series_restart';
    // my @bad_args = grep { not $ok_args->{$_} } keys %flat_args;
    // if ( scalar @bad_args ) {
    //     Marpa::R2::exception(
    //         q{Bad named argument(s) to $slr->}
    //             . $method
    //             . q{() method: }
    //             . join q{ },
    //         @bad_args
    //     );
    // } //// end if ( scalar @bad_args )

    // // Special SLIF (not NAIF) recce arg processing goes here
    // if ( exists $flat_args{'exhaustion'} ) {

    //     state $exhaustion_actions = { map { ( $_, 0 ) } qw(fatal event) };
    //     my $value = $flat_args{'exhaustion'} // 'undefined';
    //     Marpa::R2::exception(
    //         qq{'exhaustion' named arg value is $value (should be one of },
    //         (   join q{, },
    //             map { q{'} . $_ . q{'} } keys %{$exhaustion_actions}
    //         ),
    //         ')'
    //     ) if not exists $exhaustion_actions->{$value};
    //     $slr->[Marpa::R2::Internal::Scanless::R::EXHAUSTION_ACTION] = $value;

    // } //// end if ( exists $flat_args{'exhaustion'} )

    // // Special SLIF (not NAIF) recce arg processing goes here
    // if ( exists $flat_args{'rejection'} ) {

    //     state $rejection_actions = { map { ( $_, 0 ) } qw(fatal event) };
    //     my $value = $flat_args{'rejection'} // 'undefined';
    //     Marpa::R2::exception(
    //         qq{'rejection' named arg value is $value (should be one of },
    //         (   join q{, },
    //             map { q{'} . $_ . q{'} } keys %{$rejection_actions}
    //         ),
    //         ')'
    //     ) if not exists $rejection_actions->{$value};
    //     $slr->[Marpa::R2::Internal::Scanless::R::REJECTION_ACTION] = $value;

    // } //// end if ( exists $flat_args{'rejection'} )

    // // A bit hack-ish, but some named args are copies straight to an member of
    // // the Scanless::R class, so this maps named args to the index of the array
    // // that holds the members.
    // state $copy_arg_to_index = {
    //     trace_file_handle =>
    //         Marpa::R2::Internal::Scanless::R::TRACE_FILE_HANDLE,
    //     trace_lexers    => Marpa::R2::Internal::Scanless::R::TRACE_LEXERS,
    //     trace_terminals => Marpa::R2::Internal::Scanless::R::TRACE_TERMINALS,
    //     grammar         => Marpa::R2::Internal::Scanless::R::GRAMMAR,
    // };

    // ARG: for my $arg_name ( keys %flat_args ) {
    //     my $index = $copy_arg_to_index->{$arg_name};
    //     next ARG if not defined $index;
    //     my $value = $flat_args{$arg_name};
    //     $slr->[$index] = $value;
    // } //// end ARG: for my $arg_name ( keys %flat_args )

    // // Normalize trace levels to numbers
    // for my $trace_level_arg (
    //     Marpa::R2::Internal::Scanless::R::TRACE_TERMINALS,
    //     Marpa::R2::Internal::Scanless::R::TRACE_LEXERS
    //     )
    // {
    //     $slr->[$trace_level_arg] = 0
    //         if
    //         not Scalar::Util::looks_like_number( $slr->[$trace_level_arg] );
    // } //// end for my $trace_level_arg ( ...)

    // // Trace file handle can never be undefined
    // if (not defined $slr->[Marpa::R2::Internal::Scanless::R::TRACE_FILE_HANDLE] )
    // {
    //     my $slg = $slr->[Marpa::R2::Internal::Scanless::R::GRAMMAR];
    //     $slr->[Marpa::R2::Internal::Scanless::R::TRACE_FILE_HANDLE] =
    //         $slg->[Marpa::R2::Internal::Scanless::G::TRACE_FILE_HANDLE];
    // } //// end if ( not defined $slr->[...])

    // // These NAIF recce args, when applicable, are simply copies of the the
    // // SLIF args of the same name
    // state $copyable_naif_recce_args = {
    //     map { ( $_, 1 ); }
    //         qw(end max_parses semantics_package too_many_earley_items ranking_method
    //         trace_actions trace_file_handle trace_terminals trace_values)
    // };

    // // Prune flat args of all those named args which are NOT to be copied
    // // into the NAIF recce args
    let g1_recce_args = CommonNaifRecceArgs::default();
    // for my $arg_name ( grep { $copyable_naif_recce_args->{$_} }
    //     keys %flat_args )
    // {
    //     $g1_recce_args{$arg_name} = $flat_args{$arg_name};
    // }

    (g1_recce_args, flat_args)
  } // end internal_set

  pub fn ambiguous(&self) -> Option<bool> { None } // TODO
  pub fn value(&self) -> Option<bool> { None } // TODO
  pub fn read(&self, input: &str) -> Result<()> { Ok(()) } // TODO
}