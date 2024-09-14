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

define_proc_macro!(dyn_safe_owned_dispatch);
define_proc_macro!(dyn_safe);

macro_rules! define_proc_macro {( $macro_name:ident $(,)? ) => (
    mod $macro_name;

    #[proc_macro_attribute] pub
    fn $macro_name(
        args: TokenStream,
        input: TokenStream,
    ) -> TokenStream
    {
        $macro_name::macro_(args.into(), input.into())
            // .map(|ts| { println!("{ts}"); ts }) /* when debugging */
            // .map(|ts| {
            //     let file_name = concat!("/tmp/", stringify!($macro_name), ".rs");
            //     ::std::fs::write(
            //         file_name, ::prettyplease::unparse(&parse_quote!(#ts)),
            //     ).unwrap();
            //     quote!(
            //         include!(file_name);
            //     )
            // })
            .map_err(|mut err| {
                // Prefix the compile error message(s) with `#[$macro_name]: `.
                let mut errs = err.into_iter().map(|e| Error::new_spanned(
                    &e.to_compile_error(),
                    format!("#[{}]: {e}", stringify!($macro_name)),
                ));
                err = errs.next().unwrap();
                errs.for_each(|e| err.combine(e));
                err
            })
            .unwrap_or_else(Error::into_compile_error)
            .into()
    }
)} use define_proc_macro;
