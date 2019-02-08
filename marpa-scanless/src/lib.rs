#![recursion_limit = "100"]
extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(CompileGrammar, attributes(source))]
pub fn grammar(input: TokenStream) -> TokenStream {
  let item = parse_macro_input!(input as DeriveInput);
  quote!(
    macro_rules! this_grammar {
      () => { Grammar::new() };
    }
  ).into()
}

