use super::*;

mod args;

pub fn macro_(
    args: TokenStream2,
    input: TokenStream2,
) -> Result<TokenStream2>
{
    let args::Args {
        pub_: extension_trait_pub,
        TraitName: ExtensionTraitName @ _,
        pattern: extension_trait_method_renaming_logic,
        ..
    } = &parse2(args)?;
    let input: ItemTrait = parse2(input)?;
    common_logic(
        input,
        Some((extension_trait_pub, ExtensionTraitName, extension_trait_method_renaming_logic)),
        None,
    )
}

pub(super) // `#[dyn_safe]` uses this too
fn common_logic(
    mut input: ItemTrait,
    mb_ext_trait: Option<(&Visibility, &Ident, &args::RenamePattern)>,
    OwnRef @ _: Option<Path>,
) -> Result<TokenStream2>
{
    let mut ret = quote!();
    let mut storage = None;
    let extension_trait_method_renaming_logic = mb_ext_trait.map_or_else(
        || &*storage.insert(args::RenamePattern::identity()),
        |(_, _, it)| it,
    );
    let Trait @ _ = &input.ident;
    let implTrait @ _ = &format_ident!("ඞimpl{Trait}");
    let (intro_generics, fwd_generics, where_clause) = &input.generics.split_for_impl();
    // assert that the trait is `dyn`-safe.
    // ret.extend(quote_spanned!(Span::mixed_site()=>
    //     impl #intro_generics dyn #Trait #fwd_generics
    //     #where_clause
    //     {}
    // ));
    let mut orig_methods = Vec::<TraitItemFn>::new();
    let mut ref_mut_self_unsafe_methods = Vec::<TraitItemFn>::new();
    let mut ref_mut_self_method_impls = Vec::<TraitItemFn>::new();
    let mut own_ref_forwarding_impls = Vec::<TraitItemFn>::new();
    for trait_item in &input.items {
        // ignore / keep as-is the non-fn `trait` items;
        let TraitItem::Fn(trait_fn) = trait_item
        else {
            continue;
        };
        // ignore / keep as-is the non-methods;
        let Some(receiver) = trait_fn.sig.receiver()
        else {
            continue;
        };
        // ignore / keep as-is the (explicitly) `where Self : Sized`-bounded methods.
        if has_where_self_sized_clause(&trait_fn.sig) {
            continue;
        }
        // ignore / keep as-is the non-owned-self-methods;
        if receiver_is_owned_self(&receiver).not() {
            continue;
        }
        // By now we must be dealing with an owned `self` method.
        let method = trait_fn;

        orig_methods.push(method.clone());
        own_ref_forwarding_impls.push(
            own_ref_forwarding_impl(trait_fn, Trait, implTrait, fwd_generics, extension_trait_method_renaming_logic)
        );

        let self_ = &receiver.self_token;
        let method_name = &method.sig.ident;
        let ref_mut_self_unsafe_method = {
            // To be the same as `method` but for three things.
            let mut ref_mut_self_unsafe_method = method.clone();
            // 1. Make sure it is an unsafe fn.
            ref_mut_self_unsafe_method.sig.unsafety = parse_quote!(
                unsafe
            );
            // 2. `&mut self` instead of `self` receiver.
            *ref_mut_self_unsafe_method.sig.inputs.first_mut().unwrap() =
                parse_quote_spanned!(receiver.span()=>
                    & /* TODO: unelided lifetime */ mut #self_
                )
            ;
            // 3. mangle the name to prevent users from unknowingly calling it.
            ref_mut_self_unsafe_method.sig.ident = format_ident!(
                "ඞdyn_{method_name}_ownref"
            );
            ref_mut_self_unsafe_method
        };
        // For a given owned-`self`-method among the associated trait items, add to it
        // an implementation body to be used in the following context:
        // ```rust ,ignore
        // impl<T : #Trait #fwd_impls, ...>
        //     #UncheckedMutSelfTrait #fwd_impls
        // for
        //     T
        // ```
        // Mainly, `T` here is `Sized`, so we can offer the `&own self` method implementation
        // using "`.deref_move()`".
        // Except we cannot just have `&own self` `dyn`-safe methods (since we are just a macro!),
        // so we work around that by having a `&mut self` `unsafe` method, which we will expect
        // to only be called on a `&mut **MD::new(own_ref)`.
        // (Conceptually, we replace `self: OwnRef<'_, Self>` with `self: &mut MD<Self>`, but
        // lying to the type system about it (since only `&mut self` gets to be `dyn` able)).
        //
        // All this trickery has a concrete example over the `src/traits/fn_own.rs` file, which
        // should illustrate what we are doing.
        let ref_mut_self_method_impl = {
            let mut ref_mut_self_method_impl = ref_mut_self_unsafe_method.clone();
            let mut each_fn_arg = Vec::<Ident>::new();
            // Make sure the args are forwardable (_e.g._, `ident` patterns).
            ref_mut_self_method_impl
                .sig
                .inputs
                .iter_mut()
                .zip(0..)
                .skip(1)
                .for_each(|(fn_arg, i)| {
                    let FnArg::Typed(fn_arg) = fn_arg
                    else {
                        unreachable!();
                    };
                    if let Pat::Ident(PatIdent { ident: arg_ident, .. }) = &*fn_arg.pat {
                        each_fn_arg.push(arg_ident.clone());
                    } else {
                        let arg_ident = format_ident!("ඞ{i}", span=fn_arg.pat.span());
                        *fn_arg.pat = Pat::Verbatim(quote!( #arg_ident ));
                        each_fn_arg.push(arg_ident)
                    }
                })
            ;
            // Should be empty in order to be `dyn`-safe, but we handle these generics
            // anyways to avoid confusing error message noise in the diagnostics.
            let turbofish = turbofish(&method.sig.generics);
            ref_mut_self_method_impl.semi_token = None;
            ref_mut_self_method_impl.default = Some(parse_quote!({
                let this: #implTrait = unsafe {
                    // SAFETY: as per the caller precondition;
                    // this is morally equivalent to `ManuallyDrop::take()`.
                    <*const #implTrait>::read(#self_)
                };
                <
                    #implTrait
                    as
                    #Trait #fwd_generics
                >::#method_name ::<#(#turbofish, )*> (
                    this #(, #each_fn_arg)*
                )
            }));
            ref_mut_self_method_impl
        };
        ref_mut_self_unsafe_methods.push(ref_mut_self_unsafe_method);
        ref_mut_self_method_impls.push(ref_mut_self_method_impl);
    }
    let mut_self_trait = {
        let mut mut_self_trait;
        (input.items, mut_self_trait) = (mem::take(&mut input.items), input.clone());
        mut_self_trait.items.extend(
            ref_mut_self_unsafe_methods.into_iter().map(TraitItem::Fn)
        );
        mut_self_trait.ident = format_ident!(
            "ඞDyn{}OwnRef", mut_self_trait.ident,
        );
        mut_self_trait.attrs.push(parse_quote!(
            #[doc(hidden)]
        ));
        mut_self_trait
    };
    let UncheckedMutSelfTrait @ _ = &mut_self_trait.ident;
    input.supertraits.push(parse_quote!(
        #UncheckedMutSelfTrait #fwd_generics
    ));
    input.to_tokens(&mut ret);
    mut_self_trait.to_tokens(&mut ret);
    {
        let mut generics_with_implTrait = input.generics.clone();
        let num_lifetimes = input.generics.lifetimes().count();
        generics_with_implTrait.params.insert(
            num_lifetimes,
            parse_quote_spanned!(Span::mixed_site()=>
                #implTrait : ?::core::marker::Sized + #Trait #fwd_generics
            ),
        );
        let mut generics_with_implTrait_sized = generics_with_implTrait.clone();
        generics_with_implTrait_sized.make_where_clause().predicates.push(
            parse_quote_spanned!(Span::mixed_site()=>
                #implTrait : Sized
            )
        );
        let (intro_generics_with_implTrait, _, where_clause_unsized) =
            generics_with_implTrait.split_for_impl()
        ;
        let (_, _, where_clause_sized) = generics_with_implTrait_sized.split_for_impl();
        let own_ref_defs = orig_methods.into_iter().map(|mut method| {
            method.attrs.clear();
            method.default = None;
            method.semi_token.get_or_insert_with(<_>::default);
            method.sig.ident = extension_trait_method_renaming_logic.rename(
                &method.sig.ident,
            );
            method
        });
        ret.extend(quote_spanned!(Span::mixed_site()=>
            impl #intro_generics_with_implTrait
                #UncheckedMutSelfTrait #fwd_generics
            for
                #implTrait
            #where_clause_sized
            {
                #( #ref_mut_self_method_impls )*
            }
        ));
        let MbExtTrait @ _ = if let Some((extension_trait_pub, ExtensionTrait, _)) = mb_ext_trait {
            ret.extend(quote_spanned!(ExtensionTrait.span() =>
                #[doc = concat!(
                    "Extension trait allowing `dyn`amic dispatch of the (owned) `self` ",
                    "methods of [`", ::core::stringify!(#Trait), "`],\n",
                    "\n",
                    "from within an ",
                    r"<code>[OwnRef][::own_ref::OwnRef]\<\'_, dyn [", ::core::stringify!(#Trait), r"] + …\></code> receiver.",
                )]
                #extension_trait_pub
                trait #ExtensionTrait #intro_generics
                #where_clause
                {
                    #( #own_ref_defs )*
                }
            ));
            ExtensionTrait
        } else {
            Trait
        };
        let OwnRef @ _ = OwnRef.unwrap_or_else(|| parse_quote!(
            ::own_ref::OwnRef
        ));
        ret.extend(quote!(
            impl #intro_generics_with_implTrait
                #MbExtTrait #fwd_generics
            for
                #OwnRef<'_, #implTrait>
            #where_clause_unsized
            {
                #( #own_ref_forwarding_impls )*
            }
        ));
    }
    Ok(ret)
}

fn receiver_is_owned_self(receiver: &Receiver)
  -> bool
{
    // self                      // : Self
    receiver.reference.is_none() && matches!(
        &*receiver.ty,
        Type::Path(TypePath {
            qself: None,
            path,
        }) if path.is_ident("Self")
    )
}

fn has_where_self_sized_clause(
    sig: &Signature,
) -> bool
{
    sig.generics.where_clause.as_ref().map_or(
        false,
        |where_clause| where_clause.predicates.iter().any(|predicate| matches!(
            predicate,
            WherePredicate::Type(PredicateType {
                bounded_ty: Type::Path(TypePath {
                    qself: None,
                    path: lhs_ty_path,
                }),
                bounds,
                // deliberately ignored, to support things such as `for<'trivial> Self : Sized`.
                lifetimes: _,
                ..
            })  if lhs_ty_path.is_ident("Self") // `Self : …`
                && bounds.iter().any(|bound| matches!(
                    bound, TypeParamBound::Trait(TraitBound {
                        paren_token: None,
                        modifier: TraitBoundModifier::None,
                        lifetimes: _,
                        path: Path {
                            segments: rhs_trait_path,
                            ..
                        },
                    }) if {
                        // `… : [[[::]? <core|std>::]? marker::]? Sized`
                        let len = rhs_trait_path.len();
                        rhs_trait_path[len - 1].ident == "Sized"
                        && (len < 2 || rhs_trait_path[len - 2].ident == "marker")
                        && (len < 3 || ["core", "std"].contains(
                            &&*rhs_trait_path[len - 3].ident.to_string()
                        ))
                    }
                ))
        ))
    )
}

/// For a given owned-`self`-method among the associated trait items, add to it
/// an implementation body to be used in the following context:
/// ```rust ,ignore
/// impl<#implTrait : #Trait #fwd_impls, ...>
///     #ExtensionTraitName #fwd_impls
/// for
///     OwnRef<'_, #implTrait>
/// ```
fn own_ref_forwarding_impl(
    method: &TraitItemFn,
    Trait @ _: &Ident,
    implTrait @ _: &Ident,
    fwd_generics: &TypeGenerics<'_>,
    extension_trait_method_renaming_logic: &args::RenamePattern,
) -> TraitItemFn
{
    let mut ret = method.clone();

    let self_ = method.sig.receiver().unwrap().self_token;
    let mut each_fn_arg = Vec::<Ident>::new();

    ret.attrs = vec![
        parse_quote!(
            #[allow(warnings, clippy::all)]
        ),
        parse_quote!(
            #[inline]
        ),
    ];
    ret.sig.ident = extension_trait_method_renaming_logic.rename(&ret.sig.ident);
    ret.sig.inputs.iter_mut().zip(0..).skip(1).for_each(|(fn_arg, i)| {
        let FnArg::Typed(fn_arg) = fn_arg
        else {
            unreachable!();
        };
        if let Pat::Ident(PatIdent { ident: arg_ident, .. }) = &*fn_arg.pat {
            each_fn_arg.push(arg_ident.clone());
        } else {
            let arg_ident = format_ident!("ඞ{i}", span=fn_arg.pat.span());
            *fn_arg.pat = Pat::Verbatim(quote!( #arg_ident ));
            each_fn_arg.push(arg_ident)
        }
    });
    ret.default = Some({
        let dyn_method_ownref = format_ident!(
            "ඞdyn_{}_ownref", method.sig.ident,
        );
        let DynTraitOwnRef @ _ = format_ident!(
            "ඞDyn{Trait}OwnRef",
        );
        let turbofish = turbofish(&method.sig.generics);
        Block {
            // the choice of this span appears to be yielding nicer error messages
            // should the `method_rename_logic` be `{}` and method resolution ambiguity ensue.
            brace_token: token::Brace(method.sig.ident.span()),
            stmts: parse_quote_spanned!(Span::mixed_site()=>
                let this: &mut #implTrait =
                    &mut **::core::mem::ManuallyDrop::<::own_ref::OwnRef<'_, #implTrait>>::new(
                        #self_
                    )
                ;
                unsafe {
                    <
                        #implTrait
                        as
                        #DynTraitOwnRef #fwd_generics
                    >::#dyn_method_ownref ::<#(#turbofish, )*> (
                        this #(, #each_fn_arg)*
                    )
                }
            ),
        }
    });
    ret.semi_token = None;
    ret
}

/// To be `quote!`-d as `:: < #(#turbofish, )* >`.
fn turbofish(generics: &Generics)
  -> impl Iterator<Item = &Ident>
{
    Iterator::chain(
        generics.type_params().map(|p| &p.ident),
        generics.const_params().map(|p| &p.ident),
    )
}
