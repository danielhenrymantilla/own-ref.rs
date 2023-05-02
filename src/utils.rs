#[cfg(doctest)]
macro_rules! compile_fail {(
 $( #$attr:tt )*
    fn $fname:ident $($rest:tt)*
) => (
 $( #$attr )*
    /// ```rust, compile_fail
    /// use ::own_ref::*;
    ///
    #[doc = stringify!( fn main $($rest)* )]
    #[doc = "\n```"]
    fn $fname() {}
)}
