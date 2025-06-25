use deluxe::ParseMetaItem;

use crate::error::Error;

// pub fn parse_and_generate<
//     T: deluxe::ParseMetaItem,
//     I: syn::parse::Parse,
//     F: FnOnce(T, &syn::Item) -> Result<proc_macro2::TokenStream, Error>,
// >(
//     args: proc_macro::TokenStream,
//     item: proc_macro::TokenStream,
//     f: F,
// ) -> Result<proc_macro2::TokenStream, Error> {
//     let (parsed_args, parsed_item) = parse(args, item)?;
//     f(parsed_args, &parsed_item)
// }

pub fn parse<T: deluxe::ParseMetaItem, I: syn::parse::Parse>(
    args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> Result<(T, I), Error> {
    Ok((deluxe::parse2(args.into())?, syn::parse(item)?))
}

#[derive(deluxe::ParseMetaItem)]
pub struct MyTraitMacroArgs {
    #[deluxe(default)]
    pub default: Option<syn::Ident>,
}

#[derive(deluxe::ParseMetaItem)]
pub struct MyMacroArgs {
    #[deluxe(rest)]
    pub args: std::collections::HashMap<syn::Ident, InnerArgs>,
}

#[derive(ParseMetaItem)]
pub struct InnerArgs {
    #[deluxe(append, rename = ext)]
    pub exts: Vec<syn::Ident>,
    #[deluxe(default)]
    pub default: Option<syn::Ident>,
}
