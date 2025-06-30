use deluxe::HasAttributes;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::{
    punctuated::Punctuated, Attribute, FnArg, Item, ItemTrait, PatType, Signature, Token, TraitItem, Type
};

use crate::{
    args::{InnerArgs, MyMacroArgs, MyTraitMacroArgs},
    error::Error, util::has_attr,
};

pub fn generate(args: &MyTraitMacroArgs, item: &Item) -> TokenStream {
    inner_generate(args, item).unwrap_or_else(Into::into)
}

pub fn derive_contract(args: &MyMacroArgs, trait_impls: &Item) -> TokenStream {
    derive_contract_inner(args, trait_impls).unwrap_or_else(Into::into)
}

fn generate_method(
    (trait_item, item_trait): (&syn::TraitItem, &syn::ItemTrait),
) -> Option<(Option<TokenStream>, TokenStream)> {
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
    let static_method = if has_attr(attrs, "internal") {
        None
    } else  {
        Some(generate_static_method(item_trait, sig, attrs, name, &args))
    };
    Some((
        static_method,
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
    args: &[&Ident],
) -> TokenStream {
    let trait_name = &trait_name.ident;
    let output = &sig.output;

    // Transform inputs and generate call arguments
    let (transformed_inputs, call_args): (Vec<_>, Vec<_>) = sig
        .inputs
        .iter()
        .zip(args.iter())
        .filter_map(|(input, arg_name)| {
            if let FnArg::Typed(PatType { pat, ty, .. }) = input {
                let (new_ty, call_expr) = transform_type_and_call(ty, arg_name);
                Some((quote! { #pat: #new_ty }, call_expr))
            } else {
                // Skip 'self' parameters
                None
            }
        })
        .unzip();

    quote! {
        #(#attrs)*
        pub fn #name(#(#transformed_inputs),*) #output {
            <$contract_name as #trait_name>::#name(#(#call_args),*)
        }
    }
}

fn transform_type_and_call(ty: &Type, arg_name: &Ident) -> (TokenStream, TokenStream) {
    match ty {
        // &T -> T, call with &arg
        Type::Reference(type_ref) if type_ref.mutability.is_none() => {
            let inner_type = &type_ref.elem;
            (quote! { #inner_type }, quote! { &#arg_name })
        }
        // &mut T -> T, call with &mut arg
        Type::Reference(type_ref) if type_ref.mutability.is_some() => {
            let inner_type = &type_ref.elem;
            (quote! { #inner_type }, quote! { &mut #arg_name })
        }
        // Any other type -> keep as is, call with arg
        _ => (quote! { #ty }, quote! { #arg_name }),
    }
}

fn generate_trait_method(
    sig: &Signature,
    attrs: &[Attribute],
    name: &Ident,
    args: &[&Ident],
) -> TokenStream {
    let inputs = sig.inputs.iter();
    let output = &sig.output;
    quote! {
        #(#attrs)*
        fn #name(#(#inputs),*) #output {
            Self::Impl::#name(#(#args),*)
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
    items.insert(
        0,
        syn::parse_quote! {
            type Impl: #trait_ident;
        },
    );
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
    let strukt_name = &strukt.ident;
    let macro_calls = args
        .args
        .iter()
        .map(|(trait_ident, InnerArgs { exts, default })| {
            let init = default.as_ref().map_or_else(
                || quote! {#trait_ident!()},
                |default| {
                    quote! {#default }
                },
            );
            let default_impl = exts.iter().fold(
                init,
                |acc, extension| quote! { #extension<#strukt_name, #acc> },
            );
            quote! {
                #trait_ident!(#strukt_name, #default_impl);
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
        pub trait Administratable {
            type Impl: Administratable;
            #[doc = r" Get current admin"]
            fn admin_get(env: Env) -> soroban_sdk::Address {
                Self::Impl::admin_get(env)
            }
            fn admin_set(env: Env, new_admin: soroban_sdk::Address) {
                Self::Impl::admin_set(env, new_admin)
            }
        }
        #[macro_export]
        macro_rules! Administratable {
            ($contract_name: ident) => {
                Administratable!($contract_name, Admin);
            };

            ($contract_name: ident, $($impl_type: tt)+) => {
                Administratable!(@dispatch $contract_name, $($impl_type)+);
            };

            (@dispatch $contract_name: ident, $impl_name: ident) => {
                impl Administratable for $contract_name {
                    type Impl = $impl_name;
                }

                #[soroban_sdk::contractimpl]
                impl $contract_name {
                    #[doc = r" Get current admin"]
                    pub fn admin_get(env: Env) -> soroban_sdk::Address {
                        < $contract_name as Administratable >::admin_get(env)
                    }

                    pub fn admin_set(env: Env, new_admin: soroban_sdk::Address) {
                        < $contract_name as Administratable >::admin_set(env, new_admin)
                    }
                }
            };

            (@dispatch $contract_name: ident, $($impl_type: tt)+) => {
                impl Administratable for $contract_name {
                    type Impl = $($impl_type)+;
                }

                #[soroban_sdk::contractimpl]
                impl $contract_name {
                    #[doc = r" Get current admin"]
                    pub fn admin_get(env: Env) -> soroban_sdk::Address {
                        < $contract_name as Administratable >::admin_get(env)
                    }

                    pub fn admin_set(env: Env, new_admin: soroban_sdk::Address) {
                        < $contract_name as Administratable >::admin_set(env, new_admin)
                    }
                }
            };

            () => {
                Admin
            };
        }


                };
        equal_tokens(&output, &result);
    }

    #[test]
    fn derive() {
        let input: Item = syn::parse_quote! {
            pub struct Contract;
        };
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
        pub struct Contract;
        Upgradable ! (Contract , AdministratableExt < Contract , Upgradable ! () >);
        Administratable!(Contract, Administratable!());
        };
        equal_tokens(&output, &result);
    }
}
