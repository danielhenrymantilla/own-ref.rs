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
    let input: ItemTrait = parse2(input)?;
    super::dyn_safe_owned_dispatch::common_logic(
        input,
        None,
        Some(OwnRef),
    )
}
