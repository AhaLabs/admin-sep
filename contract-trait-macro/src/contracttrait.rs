use deluxe::HasAttributes;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::{punctuated::Punctuated, Attribute, FnArg, Item, ItemTrait, Signature, Token, TraitItem};

use crate::{
    args::{InnerArgs, MyMacroArgs, MyTraitMacroArgs},
    error::Error,
};

pub fn generate(args: &MyTraitMacroArgs, item: &Item) -> TokenStream {
    inner_generate(args, item).unwrap_or_else(Into::into)
}

pub fn derive_contract(args: &MyMacroArgs, trait_impls: &Item) -> TokenStream {
    derive_contract_inner(args, trait_impls).unwrap_or_else(Into::into)
}

fn generate_method(
    (trait_item, item_trait): (&syn::TraitItem, &syn::ItemTrait),
) -> Option<(TokenStream, TokenStream)> {
    let syn::TraitItem::Fn(method) = trait_item else {
        return None;
    };
    let sig = &method.sig;
    let name = &sig.ident;
    if sig.receiver().is_some() {
        return None;
    };
    let args = args_to_idents(&sig.inputs);
    let attrs = &method.attrs;
    Some((
        generate_static_method(item_trait, sig, attrs, name, &args),
        generate_trait_method(sig, attrs, name, &args),
    ))
}

pub fn args_to_idents(inputs: &Punctuated<FnArg, Token!(,)>) -> Vec<&Ident> {
    inputs
        .iter()
        .filter_map(|arg| {
            if let syn::FnArg::Typed(syn::PatType { pat, .. }) = arg {
                match &**pat {
                    syn::Pat::Ident(pat_ident) => Some(&pat_ident.ident),
                    _ => None,
                }
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
}

fn generate_static_method(
    trait_name: &ItemTrait,
    sig: &Signature,
    attrs: &[Attribute],
    name: &Ident,
    args_without_self: &[&Ident],
) -> TokenStream {
    let inputs = sig.inputs.iter();
    let output = &sig.output;
    let trait_name = &trait_name.ident;
    quote! {
        #(#attrs)*
        pub fn #name(#(#inputs),*) #output {
            <$contract_name as #trait_name>::#name(#(#args_without_self),*)
        }
    }
}

fn generate_trait_method(
    sig: &Signature,
    attrs: &[Attribute],
    name: &Ident,
    args_without_self: &[&Ident],
) -> TokenStream {
    let inputs = sig.inputs.iter();
    let output = &sig.output;
    quote! {
        #(#attrs)*
        fn #name(#(#inputs),*) #output {
            Self::Impl::#name(#(#args_without_self),*)
        }
    }
}

fn inner_generate(
    MyTraitMacroArgs {
        default,
        ext_required,
        is_ext,
    }: &MyTraitMacroArgs,
    item: &Item,
) -> Result<TokenStream, Error> {
    let Item::Trait(input_trait) = &item else {
        return Err(Error::Stream(
            quote! { compile_error!("Input must be a trait"); },
        ));
    };
    let (generated_methods, trait_methods): (Vec<_>, Vec<_>) = input_trait
        .items
        .iter()
        .zip(std::iter::repeat(input_trait))
        .filter_map(generate_method)
        .unzip();

    let trait_ident = &input_trait.ident;
    let macro_rules_name = trait_ident;
    let attrs = input_trait.attrs.as_slice();

    let mut trait_ = input_trait.clone();
    let mut items = trait_methods
        .into_iter()
        .map(syn::parse2)
        .collect::<Result<Vec<TraitItem>, _>>()?;
    items.push(syn::parse_quote! {
        type Impl: #trait_ident;
    });
    trait_.items = items;

    let default_impl = default
        .clone()
        .map_or_else(|| quote! {$contract_name}, |default| quote! {#default});

    let ensure_default = if default.is_none() {
        let message = format!(
            "The contract trait `{trait_ident}` does not provide default implementation. \
One should be passed, e.g. default = MyDefaultImpl"
        );
        quote! {
            compile_error!(#message);
        }
    } else {
        quote! {}
    };

    let first_case = if *ext_required {
        let message = format!(
            "The contract trait `{trait_ident}` requires an extension for authentication but none were provided."
        );
        quote! { compile_error!(#message); }
    } else {
        quote! {}
    };

    let extension_type = if *is_ext {
        let extension_strukt = format_ident!("{}Ext", trait_ident);

        quote! {
            pub struct #extension_strukt<T: #trait_ident, N>(
                  core::marker::PhantomData<T>,
                  core::marker::PhantomData<N>,
            );
        }
    } else {
        quote! {}
    };
    let docs = input_trait
        .attrs()
        .iter()
        .filter(|attr| attr.path().is_ident("doc"))
        .collect::<Vec<_>>();

    let output = quote! {

    #(#attrs)*
    #trait_
    #extension_type
    #(#docs)*
    #[macro_export]
    macro_rules! #macro_rules_name {
        ($contract_name:ident) => {
            #ensure_default
            #macro_rules_name!($contract_name, #default_impl);
        };
        // Use a single tt to avoid ambiguity, then dispatch
        ($contract_name:ident, $($impl_type:tt)+) => {
            #macro_rules_name!(@dispatch $contract_name, $($impl_type)+);
        };
        // Match normal identifier
        (@dispatch  $contract_name:ident, $impl_name:ident) => {
            #first_case
            impl #trait_ident for $contract_name {
                type Impl = $impl_name;
            }
            #[soroban_sdk::contractimpl]
            impl $contract_name {
                #(#generated_methods)*
            }
        };
        // Match identifier with generics
        (@dispatch $contract_name:ident,  $($impl_type:tt)+) => {
            impl #trait_ident for $contract_name {
                type Impl = $($impl_type)+;
            }
            #[soroban_sdk::contractimpl]
            impl $contract_name {
                #(#generated_methods)*
            }
        };
        () => {
            #default_impl
        };

                }

                        };
    Ok(output)
}

pub fn derive_contract_inner(args: &MyMacroArgs, trait_impls: &Item) -> Result<TokenStream, Error> {
    let Item::Struct(strukt) = trait_impls else {
        return Err(Error::Stream(
            quote! { compile_error!("Input must be a struct"); },
        ));
    };
    // Parse attribute arguments
    let strukt_name = &strukt.ident;

    // Convert to Vec<(Ident, Option<Ident>)>
    let macro_calls = args
        .args
        .iter()
        .map(|(trait_ident, InnerArgs { exts, default })| {
            let macro_name = format_ident!("{}", trait_ident);
            if !exts.is_empty() {
                let base = default.as_ref().map_or_else(
                    || quote! {#trait_ident!()},
                    |default| {
                        quote! {#default }
                    },
                );
                let ext_args = exts
                    .iter()
                    .fold(base, |acc, ext| quote! { #ext<#strukt_name, #acc> });
                return quote! {
                    #macro_name!(#strukt_name, #ext_args);
                };
            };
            if let Some(impl_ident) = default {
                quote! {
                    #macro_name!(#strukt_name, #impl_ident);
                }
            } else {
                quote! {
                    #macro_name!(#strukt_name);
                }
            }
        });

    Ok(quote! {
        #strukt
        #(#macro_calls)*
    })
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::util::*;

    #[test]
    fn first() {
        let input: Item = syn::parse_quote! {
            pub trait Administratable {
                /// Get current admin
                fn admin_get(env: Env) -> soroban_sdk::Address;
                fn admin_set(env: Env, new_admin: soroban_sdk::Address);
            }
        };
        let default = Some(format_ident!("Admin"));
        let result: TokenStream = generate(
            &MyTraitMacroArgs {
                default,
                ..Default::default()
            },
            &input,
        );
        println!("{}", format_snippet(&result.to_string()));

        let output = quote! {
            pub trait IsOwnable {
                /// Get current admin
                fn admin_get(&self) -> Option<Address>;
                fn admin_set(&mut self, new_admin: Address) -> Result<(), Error>;
                fn admin_set_two(&mut self, new_admin: Address);
            }
            pub trait Ownable {
                /// Type that implments the instance type
                type Impl: Lazy + IsOwnable + Default;
                /// Get current admin
                fn admin_get() -> Option<Address> {
                    Self::Impl::get_lazy().unwrap_or_default().admin_get()
                }
                fn admin_set(new_admin: Address) -> Result<(), Error> {
                    let mut impl_ = Self::Impl::get_lazy().unwrap_or_default();
                    let res = impl_.admin_set(new_admin)?;
                    Self::Impl::set_lazy(impl_);
                    Ok(res)
                }
                fn admin_set_two(new_admin: Address) {
                    let mut impl_ = Self::Impl::get_lazy().unwrap_or_default();
                    let res = impl_.admin_set_two(new_admin);
                    Self::Impl::set_lazy(impl_);
                    res
                }
            }

        };
        equal_tokens(&output, &result);
        // let impl_ = syn::parse_str::<ItemImpl>(result.as_str()).unwrap();
        // println!("{impl_:#?}");
    }

    #[test]
    fn derive() {
        let input: Item = syn::parse_quote! {
            pub struct Contract;
        };
        // let default = Some(format_ident!("Admin"));
        let args = vec![
            (
                format_ident!("Administratable"),
                InnerArgs {
                    exts: vec![],
                    default: None,
                },
            ),
            (
                format_ident!("Upgradable"),
                InnerArgs {
                    exts: vec![format_ident!("AdministratableExt")],
                    default: None,
                },
            ),
        ];

        let result = derive_contract(
            &MyMacroArgs {
                args: args.into_iter().collect(),
            },
            &input,
        );
        println!("{}", format_snippet(&result.to_string()));

        // let output = quote! {
        //     pub trait IsOwnable {
        //         /// Get current admin
        //         fn admin_get(&self) -> Option<Address>;
        //         fn admin_set(&mut self, new_admin: Address) -> Result<(), Error>;
        //         fn admin_set_two(&mut self, new_admin: Address);
        //     }
        //     pub trait Ownable {
        //         /// Type that implments the instance type
        //         type Impl: Lazy + IsOwnable + Default;
        //         /// Get current admin
        //         fn admin_get() -> Option<Address> {
        //             Self::Impl::get_lazy().unwrap_or_default().admin_get()
        //         }
        //         fn admin_set(new_admin: Address) -> Result<(), Error> {
        //             let mut impl_ = Self::Impl::get_lazy().unwrap_or_default();
        //             let res = impl_.admin_set(new_admin)?;
        //             Self::Impl::set_lazy(impl_);
        //             Ok(res)
        //         }
        //         fn admin_set_two(new_admin: Address) {
        //             let mut impl_ = Self::Impl::get_lazy().unwrap_or_default();
        //             let res = impl_.admin_set_two(new_admin);
        //             Self::Impl::set_lazy(impl_);
        //             res
        //         }
        //     }

        // };
        // equal_tokens(&output, &result);
        // let impl_ = syn::parse_str::<ItemImpl>(result.as_str()).unwrap();
        // println!("{impl_:#?}");
    }
}
