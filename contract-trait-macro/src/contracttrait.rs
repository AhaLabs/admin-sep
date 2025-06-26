use deluxe::HasAttributes;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::{punctuated::Punctuated, Attribute, FnArg, Item, ItemTrait, Signature, Token};

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
            <$impl_name as #trait_name>::#name(#(#args_without_self),*)
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

    let impl_trait = if let Some(default) = default {
        quote! {
            impl #trait_ident for $contract_name {
                type Impl = #default;
            }
        }
    } else {
        quote! {
            impl #trait_ident for $contract_name {}
        }
    };

    let trait_ = if default.is_some() {
        quote! {
            pub trait #trait_ident {
                type Impl: #trait_ident;
                #(#trait_methods)*
            }
        }
    } else {
        quote! {
            #input_trait
        }
    };

    let first_case = if *ext_required {
        let message = format!(
            "The contract trait `{}` requires an extension for authentication but none were provided.",
            trait_ident
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
            #macro_rules_name!($contract_name, $contract_name);
            #first_case
        };
        ($contract_name:ident, $impl_name:path) => {
            #impl_trait
            #[soroban_sdk::contractimpl]
            impl $contract_name {
                #(#generated_methods)*
            }
        };
    () => {};

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
                let base = default.as_ref().unwrap_or(strukt_name).clone();
                let base = quote! { #base };
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
        let result = generate(
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
            pub struct contract;
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
}
