#![allow(nonstandard_style, unused_braces, unused_imports)]
use ::core::{
    mem,
    ops::Not as _,
};
use ::proc_macro::{
    TokenStream,
};
use ::proc_macro2::{
    Span,
    TokenStream as TokenStream2,
};
use ::quote::{
    format_ident,
    quote,
    quote_spanned,
    ToTokens,
};
use ::syn::{*,
    parse::{self, Parse, Parser, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    Result, // explicitly shadow it
};

#[proc_macro_attribute] pub
fn dyn_self(
    args: TokenStream,
    input: TokenStream,
) -> TokenStream
{
    dyn_self_impl(args.into(), input.into())
     // .map(|ts| { println!("{ts}"); ts }) /* when debugging */
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

fn dyn_self_impl(
    args: TokenStream2,
    input: TokenStream2,
) -> Result<TokenStream2>
{
    let mut ret = quote!();
    let _args: parse::Nothing = parse2(args)?;
    let mut input: ItemTrait = parse2(input)?;
    let Trait @ _ = &input.ident;
    let (intro_generics, fwd_generics, where_clause) = input.generics.split_for_impl();
    // assert trait is `dyn`-safe.
    ret.extend(quote_spanned!(Span::mixed_site()=>
        impl #intro_generics dyn #Trait #fwd_generics
        #where_clause
        {}
    ));
    let mut unchecked_mut_self_methods = Vec::<TraitItemFn>::new();
    for trait_item in &input.items {
        // ignore / keep as-is the non-fn `trait` items;
        let TraitItem::Fn(trait_fn) = trait_item
        else {
            // TODO: collect assoc types.
            continue;
        };
        // ignore / keep as-is the non-methods;
        let Some(receiver) = trait_fn.sig.receiver()
        else {
            continue;
        };
        // ignore / keep as-is the non-owned-self-methods;
        if {
            let is_owned_self_receiver =
                receiver.reference.is_none() // self
                &&  matches!( // : Self
                    &*receiver.ty,
                    Type::Path(TypePath {
                        qself: None,
                        path,
                    })
                    if path.is_ident("Self")
                )
            ;
            is_owned_self_receiver.not()
        } {
            continue;
        }
        // By now we must be dealing with an owned `self` method.
        let method = trait_fn;
        unchecked_mut_self_methods.push({
            // To be the same as `method` but for three things.
            let mut unchecked_mut_self_method = method.clone();
            // 1. Make sure it is an unsafe fn.
            unchecked_mut_self_method.sig.unsafety = parse_quote!(
                unsafe
            );
            // 2. `&mut self` instead of `self` receiver.
            *unchecked_mut_self_method.sig.inputs.first_mut().unwrap() =
                parse_quote_spanned!(receiver.span()=>
                    & /* TODO: unelided lifetime */ mut self,
                )
            ;
            // 3. mangle the name to prevent users from unknowingly calling it.
            unchecked_mut_self_method.sig.ident = format_ident!(
                "ඞdyn_{}_ownref", method.sig.ident
            );
            unchecked_mut_self_method
        });
    }
    let unchecked_mut_self_trait = {
        let items = mem::take(&mut input.items);
        let mut unchecked_mut_self_trait = input.clone();
        input.items = items;
        unchecked_mut_self_trait.items.extend(
            unchecked_mut_self_methods.into_iter().map(TraitItem::Fn)
        );
        unchecked_mut_self_trait.ident = format_ident!(
            "ඞDyn{}OwnRef", unchecked_mut_self_trait.ident,
        );
        unchecked_mut_self_trait
    };
    let UncheckedMutSelfTrait @ _ = &unchecked_mut_self_trait.ident;
    input.supertraits.push(parse_quote!(
        #UncheckedMutSelfTrait #fwd_generics
    ));
    input.to_tokens(&mut ret);
    unchecked_mut_self_trait.to_tokens(&mut ret);
    ret.extend({
        let mut generics = input.generics.clone();
        let num_lifetimes = input.generics.lifetimes().count();
        generics.params.insert(num_lifetimes, parse_quote_spanned!(Span::mixed_site()=>
            __Self : #Trait #fwd_generics
        ));
        let (intro_generics, _, _) = generics.split_for_impl();
        quote_spanned!(Span::mixed_site()=>
            impl #intro_generics
                #UncheckedMutSelfTrait #fwd_generics
            for
                __Self
            {
                /* TODO */
            }
        )
    });
    Ok(ret)
}
