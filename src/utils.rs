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

macro_rules! match_cfg {
    ({ $($tt:tt)* }) => ({
        match_cfg!($($tt)*)
    });

    (
        _ => { $($output:tt)* } $(,)?
    ) => (
        $($output)*
    );

    (
        $predicate:meta => $output:tt $(,
        $($rest:tt)* )?
    ) => (
        #[cfg($predicate)]
        $crate::utils::match_cfg! {
            _ => $output
        }                               $(

        #[cfg(not($predicate))]
        $crate::utils::match_cfg! {
            $($rest)*
        }                               )?
    );
} pub(in crate) use match_cfg;
