use super::*;

pub
struct Args {
    _impl: Token![impl],
    _for: Token![for],
    pub OwnRef: Type,
    _trailing_punct: Option<Token![,]>,
}

impl Parse for Args {
    fn parse(
        input: ParseStream<'_>,
    ) -> Result<Args>
    {
        let result = || -> Result<_> {
            Ok(Self {
                _impl: input.parse()?,
                _for: input.parse()?,
                OwnRef: input.parse()?,
                _trailing_punct: input.parse()?,
            })
        }();
        result.map_err(|parse_err| {
            let mut err = Error::new(
                Span::mixed_site(),
                "\
                    usage `#[dyn_safe(\
                        impl for OwnRef\
                    )]`\
                ",
            );
            err.combine(parse_err);
            err
        })
    }
}
