use proc_macro::TokenStream;
use quote::quote;
use std::{collections::HashMap, sync::OnceLock};
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, LitStr,
};

struct Message {
    source: LitStr,
    args: proc_macro2::TokenStream,
}

impl Parse for Message {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        Ok(Self {
            source: input.parse()?,
            args: input.parse()?,
        })
    }
}

#[proc_macro]
pub fn localized_format(input: TokenStream) -> TokenStream {
    static EN: OnceLock<HashMap<String, String>> = OnceLock::new();
    static PL: OnceLock<HashMap<String, String>> = OnceLock::new();
    let Message { source, args } = parse_macro_input!(input as Message);
    let en = EN.get_or_init(|| {
        serde_json::from_str(include_str!("../../locales/en.json")).expect("Valid English JSON")
    });
    let pl = PL.get_or_init(|| {
        serde_json::from_str(include_str!("../../locales/pl.json")).expect("Valid Polish JSON")
    });
    let localized = |catalog: &HashMap<String, String>| {
        catalog
            .get(&source.value())
            .map_or_else(|| source.clone(), |text| LitStr::new(text, source.span()))
    };
    let english = localized(en);
    let polish = localized(pl);
    quote! {
        if crate::locale::language() == crate::locale::Language::English {
            ::std::format!(#english #args)
        } else {
            ::std::format!(#polish #args)
        }
    }
    .into()
}
