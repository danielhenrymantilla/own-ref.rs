use ext::IdentExt;

use super::*;

mod kw {
    ::syn::custom_keyword!(method_rename_logic);
}

pub
struct Args {
    pub pub_: Visibility,
    _trait: Token![trait],
    pub TraitName: Ident,
    pub rename: Option<Rename>,
    trailing_comma: Option<Token![,]>,
}

pub
struct Rename {
    _leading_comma: Token![,],
    _method_rename_logic: kw::method_rename_logic,
    _eq: Token![=],
    pub pattern: RenamePattern,
}

pub
struct RenamePattern {
    prefix: String,
    suffix: String,
}

impl Parse for Args {
    fn parse(input: ParseStream<'_>)
      -> Result<Args>
    {
        let mut args = Self {
            pub_: input.parse()?,
            _trait: input.parse()?,
            TraitName: input.parse()?,
            rename: None,
            trailing_comma: input.parse()?,
        };
        if input.is_empty() || args.trailing_comma.is_none() {
            return Ok(args);
        }
        args.rename = Some(Rename {
            _leading_comma: args.trailing_comma.unwrap(),
            _method_rename_logic: input.parse()?,
            _eq: input.parse()?,
            pattern: input.parse()?,
        });
        args.trailing_comma = input.parse()?;
        Ok(args)
    }
}

impl Parse for RenamePattern {
    fn parse(input: ParseStream<'_>)
      -> Result<RenamePattern>
    {
        // Validate that the format string is of the form `"<ident_prefix>{}<ident_suffix>"`.
        let value = LitStr::value(&input.parse()?);
        if let Some((prefix, suffix)) = value.split_once("{}") {
            let validate = |s: &str| {
                false
                || s.is_empty()
                || s == "_"
                || Parser::parse_str(Ident::parse_any, s).is_ok()
            };
            if validate(prefix) && validate(suffix) {
                return Ok(RenamePattern {
                    prefix: prefix.into(),
                    suffix: suffix.into(),
                });
            }
        }
        Err(input.error(r#"Expected a `"<ident_prefix>{}<ident_suffix>"`"#))
    }
}

impl Default for RenamePattern {
    fn default()
      -> RenamePattern
    {
        RenamePattern {
            prefix: "ownref_".into(),
            suffix: "".into(),
        }
    }
}

impl RenamePattern {
    pub
    fn rename(
        &self,
        method_name: &Ident,
    ) -> Ident
    {
        let Self { prefix, suffix } = self;
        Ident::new(
            &format!("{prefix}{}{suffix}", method_name),
            method_name.span(),
        )
    }
}
