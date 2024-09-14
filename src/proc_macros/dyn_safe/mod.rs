use super::*;

mod args;

pub fn macro_(
    args: TokenStream2,
    input: TokenStream2,
) -> Result<TokenStream2>
{
    let args::Args {
        OwnRef, ..
    } = parse2(args)?;
    Err(Error::new(Span::mixed_site(), "TODO"))
}
