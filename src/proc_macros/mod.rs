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

mod dyn_self;

#[proc_macro_attribute] pub
fn dyn_self(
    args: TokenStream,
    input: TokenStream,
) -> TokenStream
{
    dyn_self::macro_(args.into(), input.into())
        // .map(|ts| { println!("{ts}"); ts }) /* when debugging */
        // .map(|ts| {
        //     ::std::fs::write(
        //         "/tmp/dyn_self.rs", ::prettyplease::unparse(&parse_quote!(#ts)),
        //     ).unwrap();
        //     quote!(
        //         include!("/tmp/dyn_self.rs");
        //     )
        // })
        .map_err(|mut err| {
            // Prefix the compile error message(s) with `#[dyn_self]: `.
            let mut errs = err.into_iter().map(|e| Error::new_spanned(
                &e.to_compile_error(),
                format!("#[dyn_self]: {e}"),
            ));
            err = errs.next().unwrap();
            errs.for_each(|e| err.combine(e));
            err
        })
        .unwrap_or_else(Error::into_compile_error)
        .into()
}
