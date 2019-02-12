#!/usr/bin/perl
#
# adapted from sl_to_hash.pl
#
# Use as : 
# marpa$ perl meta/slif_to_rust.pl meta/metag.bnf > marpa/src/metag.rs

use 5.010001;
use strict;
use warnings;
use English qw( -no_match_vars );

# This is a 'meta' tool, so I relax some of the
# restrictions I use to guarantee portability.
use autodie;

# I expect to be run from a subdirectory in the
# development heirarchy
use lib '../../../';
use lib '../../../../blib/arch';
use Marpa::R2;

use Getopt::Long;
my $verbose         = 1;
my $help_flag       = 0;
my $result          = Getopt::Long::GetOptions(
    'help'       => \\$help_flag,
);
die "usage $PROGRAM_NAME [--help] file ...\n" if $help_flag;

sub rules_generator {
    my $rules = shift;
    my $generated = "";
    for my $rule (sort sort_bnf @$rules) {
        my $mask = "";
        my $rhs = "";
        my $min = $$rule{min} ? "Some($$rule{min})" : "None";
        my $action = $$rule{action} || "";
        my $bless = $$rule{bless} || "";
        my $lhs = $$rule{lhs} || "";
        my $name = $$rule{name} || "";
        my $proper = $$rule{proper} || "";
        my $separator = $$rule{separator} || "";
        my $description = $$rule{description} || "";
        my $symbol_as_event = $$rule{symbol_as_event} || "";

        $generated .= 
        "                MetaRecceRule {
                action: \"$action\",
                bless: \"$bless\",
                lhs: \"$lhs\",
                mask: vec![$mask],
                name: \"$name\",
                rhs: vec![$rhs],
                min: $min,
                proper: \"$proper\",
                separator: \"$separator\",
                description: \"$description\",
                symbol_as_event: \"$symbol_as_event\"
            },\n";
    }
    return $generated;
}

sub escaped {
    my $escaped = shift || "";
    $escaped =~ s/x\{/u{/g; # use \u for unicode in Rust            
    $escaped =~ s/\\/\\\\/g; # escaped for interpolated string
    return $escaped;
}

sub symbols_generator {
    my $symbols = shift;
    my $generated = "";

    for my $key (keys %{$symbols}) {
        $key = escaped($key);
        $generated .= "                \"$key\" ==> MetaRecceSymbol {\n";
        my $symbol_hash = $$symbols{$key};
        for my $subkey (qw(display_form dsl_form description)) {
            my $escaped = escaped($$symbol_hash{$subkey});
            $generated .= "                     $subkey: r#\"$escaped\"#,\n"
        }
        $generated .= "                },\n";
    }
    chomp($generated);
    chop($generated);
    
    return $generated;
}

my $bnf = do { local $RS = undef; \(<>) };
my $ast = Marpa::R2::Internal::MetaAST->new($bnf);
my $parse_result = $ast->ast_to_hash();

sub sort_bnf {
    my $cmp = $a->{lhs} cmp $b->{lhs};
    return $cmp if $cmp;
    my $a_rhs_length = scalar @{ $a->{rhs} };
    my $b_rhs_length = scalar @{ $b->{rhs} };
    $cmp = $a_rhs_length <=> $b_rhs_length;
    return $cmp if $cmp;
    for my $ix ( 0 .. ( $a_rhs_length - 1 ) ) {
        $cmp = $a->{rhs}->[$ix] cmp $b->{rhs}->[$ix];
        return $cmp if $cmp;
    }
    return 0;
} ## end sub sort_bnf

my %g = (
    discard_default_adverbs => $parse_result->{discard_default_adverbs},
    first_lhs              => $parse_result->{first_lhs},
    start_lhs              => $parse_result->{start_lhs},
    symbols                => $parse_result->{symbols},
);
my $character_classes = $parse_result->{character_classes};
my $lexeme_default_adverbs = $parse_result->{lexeme_default_adverbs};

my $declare_rules_g1 = "rules_g1: vec![\n";
$declare_rules_g1 .= rules_generator($parse_result->{rules}->{"G1"});
$declare_rules_g1 .= "        ]";

my $declare_rules_l0 = "rules_l0: vec![\n";
$declare_rules_l0 .= rules_generator($parse_result->{rules}->{"L0"});
$declare_rules_l0 .= "        ]";

my $declare_symbols_g1 = "symbols_g1: map!(\n";
$declare_symbols_g1 .= symbols_generator($parse_result->{symbols}->{"G1"});
$declare_symbols_g1 .= "\n        )";

my $declare_symbols_l0 = "symbols_l0: map!(\n";
$declare_symbols_l0 .= symbols_generator($parse_result->{symbols}->{"L"});
$declare_symbols_l0 .= "\n        )";


my $date = scalar localtime();

#  We wil directly precompile the equivalent of 
#  Marpa::R2::Internal::MetaG
#  here, to 

my $declare_character_classes = "character_classes: map!(\n";
for my $k (keys %{$character_classes}) {
    my @vals = @{$$character_classes{$k}};
    $declare_character_classes .= "          \"$k\" ==> vec![\"".join("\",\"", @vals)."\"],\n";
}
chomp($declare_character_classes);
chop($declare_character_classes); # drop last comma
$declare_character_classes .= "\n          )";
$declare_character_classes =~ s/x\{/u{/g; # use \u for unicode in Rust
$declare_character_classes =~ s/\\/\\\\/g; # also need to escape the backslashes for Rust, as we use interpolating "str" rather than Perl's 'str'

my $declare_discard_default_adverbs = "discard_default_adverbs: ".($g{discard_default_adverbs} ? "true" : "false");
my $declare_first_lhs = "first_lhs: \"$g{first_lhs}\"";

my $declare_lexeme_default_adverbs = "lexeme_default_adverbs: map!(\n";
for my $k (keys %{$lexeme_default_adverbs}) {
    my $v = $$lexeme_default_adverbs{$k};
    $declare_lexeme_default_adverbs .= "          \"$k\" ==> \"$v\",\n";
}
chomp($declare_lexeme_default_adverbs);
chop($declare_lexeme_default_adverbs);
$declare_lexeme_default_adverbs .= "\n          )";

my $declare_start_lhs = "start_lhs: \"$g{start_lhs}\"";

my $rust = <<"EOL";
// The code after this line was automatically generated by $PROGRAM_NAME
// Date: $date
use std::collections::{HashMap};

#[macro_export]
macro_rules! map {
  (\$( \$key:literal ==> \$val:expr ),*) => {{
    let mut map = ::std::collections::HashMap::new();
    \$( map.insert(\$key, \$val); )*
    map
  }}
}

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
    pub first_lhs: &'static str,
    pub start_lhs: &'static str,
    pub lexeme_default_adverbs: HashMap<&'static str, &'static str>,
    pub rules_g1: Vec<MetaRecceRule>,
    pub rules_l0: Vec<MetaRecceRule>,
    pub symbols_g1: HashMap<&'static str, MetaRecceSymbol>,
    pub symbols_l0: HashMap<&'static str, MetaRecceSymbol>,
}

/// Generates a new MetaG instance in turn used to parse SLIF sources
pub fn hashed_grammar() -> MetaRecce {
    MetaRecce {
        $declare_character_classes,
        $declare_discard_default_adverbs,
        $declare_first_lhs,
        $declare_lexeme_default_adverbs,
        $declare_rules_g1,
        $declare_rules_l0,
        $declare_start_lhs,
        $declare_symbols_g1,
        $declare_symbols_l0
    }
}
// The code before this line was automatically generated by $PROGRAM_NAME
EOL

# first_lhs
# rules
# character_classes
# start_lhs
# discard_default_adverbs
# lexeme_default_adverbs
# symbols

say $rust;
