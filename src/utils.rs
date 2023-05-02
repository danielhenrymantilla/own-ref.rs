
macro_rules! compile_fail {(
    fn $fname:ident $($rest:tt)*
) => (
    #[cfg(doctest)]
    /// ```rust, compile_fail
    /// use ::own_ref::*;
    ///
    #[doc = stringify!( fn main $($rest)* )]
    #[doc = "\n```"]
    fn $fname() {}
)}
