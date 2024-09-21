use super::*;

mod kw {
    ::syn::custom_keyword!(owned_dispatch_naming_template);
}

pub
struct Args {
    _as: Token![as],
    pub pub_: Visibility,
    _trait: Token![trait],
    pub TraitName: Ident,
    _comma: Token![,],
    _owned_dispatch_naming_template: kw::owned_dispatch_naming_template,
    _eq: Token![=],
    pub pattern: RenamePattern,
    _trailing_comma: Option<Token![,]>
}

pub
struct RenamePattern {
    prefix: String,
    suffix: String,
    span: Span,
}

impl RenamePattern {
    pub
    fn identity() -> Self
    {
        Self {
            prefix: "".into(),
            suffix: "".into(),
            span: Span::mixed_site(),
        }
    }
}

impl Parse for Args {
    fn parse(input: ParseStream<'_>)
      -> Result<Args>
    {
        let result = || -> Result<_> {
            let args = Self {
                _as: input.parse()?,
                pub_: input.parse()?,
                _trait: input.parse()?,
                TraitName: input.parse()?,
                _comma: input.parse()?,
                _owned_dispatch_naming_template: input.parse()?,
                _eq: input.parse()?,
                pattern: input.parse()?,
                _trailing_comma: input.parse()?,
            };
            Ok(args)
        }();
        result.map_err(|parse_err| {
            let mut err = Error::new(
                Span::mixed_site(),
                "\
                    usage `#[dyn_safe_owned_dispatch(\
                        <pub> trait <TraitName>, \
                        method_rename_logic = \"<prefix>{}<suffix>\"\
                    )]`\
                ",
            );
            err.combine(parse_err);
            err
        })
    }
}

impl Parse for RenamePattern {
    fn parse(input: ParseStream<'_>)
      -> Result<RenamePattern>
    {
        // Validate that the format string is of the form `"<ident_prefix>{}<ident_suffix>"`.
        let lit_str: LitStr = input.parse()?;
        if let Some((prefix, suffix)) = lit_str.value().split_once("{}") {
            let validate = |s: &str| {
                false
                || s.is_empty()
                || s == "_"
                || (
                    Parser::parse_str(<Ident as ext::IdentExt>::parse_any, s).is_ok()
                    &&
                    s.chars().any(|c| c.is_whitespace()).not()
                )
            };
            if validate(prefix) && validate(suffix) {
                return Ok(RenamePattern {
                    prefix: prefix.into(),
                    suffix: suffix.into(),
                    span: lit_str.span(),
                });
            }
        }
        Err(input.error(r#"expected `"<ident_prefix>{}<ident_suffix>"`"#))
    }
}

impl RenamePattern {
    pub
    fn rename(
        &self,
        method_name: &Ident,
    ) -> Ident
    {
        let Self { prefix, suffix, span } = self;
        Ident::new(
            &format!("{prefix}{}{suffix}", method_name),
            span.resolved_at(method_name.span()),
        )
    }
}
