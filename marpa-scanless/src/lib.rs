#![recursion_limit = "100"]
extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Lit, Meta};

#[proc_macro_derive(CompileGrammar, attributes(source))]
pub fn grammar(input: TokenStream) -> TokenStream {
  let item = parse_macro_input!(input as DeriveInput);
  let source: String = match item.attrs[0].parse_meta().unwrap() {
    Meta::NameValue(v) => match v.lit {
      Lit::Str(v) => v.value().to_string(),
      _ => panic!("only accepts #[source = \"SLIF string\"] attribute syntax, mandatory double-quotes (Lit)"),
    },
    _ => panic!("only accepts #[source = \"SLIF string\"] attribute syntax, mandatory double-quotes (parse_meta)"),
  };
  println!("SOURCE: {:?}", source);

  quote!(
    macro_rules! this_grammar {
      () => {
        Grammar::new()
        
      };
    }
  ).into()
}

