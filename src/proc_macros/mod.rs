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

mod own_ref_extension;

#[proc_macro_attribute] pub
fn own_ref_extension(
    args: TokenStream,
    input: TokenStream,
) -> TokenStream
{
    own_ref_extension::macro_(args.into(), input.into())
        // .map(|ts| { println!("{ts}"); ts }) /* when debugging */
        // .map(|ts| {
        //     ::std::fs::write(
        //         "/tmp/own_ref_extension.rs", ::prettyplease::unparse(&parse_quote!(#ts)),
        //     ).unwrap();
        //     quote!(
        //         include!("/tmp/own_ref_extension.rs");
        //     )
        // })
        .map_err(|mut err| {
            // Prefix the compile error message(s) with `#[own_ref_extension]: `.
            let mut errs = err.into_iter().map(|e| Error::new_spanned(
                &e.to_compile_error(),
                format!("#[own_ref_extension]: {e}"),
            ));
            err = errs.next().unwrap();
            errs.for_each(|e| err.combine(e));
            err
        })
        .unwrap_or_else(Error::into_compile_error)
        .into()
}
