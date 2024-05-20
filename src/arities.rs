crate::utils::match_cfg! {
    doc => {
        macro_rules! feed_all {(
            => $macro_name:ident!
        ) => (
            $macro_name! {
                /* nothing */
            }
            $macro_name! {
                _0
            }
            $macro_name! {
                _1 _0
            }
            $macro_name! {
                _11 _10 _9 _8 _7 _6
                _5 _4 _3 _2 _1 _0
            }
        )}

    },
    _ => {
        macro_rules! feed_all {(
            => $macro_name:ident!
        ) => (
            $macro_name! {
                /* nothing */
            }
            $macro_name! {
                _0
            }
            $macro_name! {
                _1 _0
            }
            $macro_name! {
                _2 _1 _0
            }
            $macro_name! {
                _3 _2 _1 _0
            }
            $macro_name! {
                _4 _3 _2 _1 _0
            }
            $macro_name! {
                _5 _4 _3 _2 _1 _0
            }
            $macro_name! {
                _6
                _5 _4 _3 _2 _1 _0
            }
            $macro_name! {
                _7 _6
                _5 _4 _3 _2 _1 _0
            }
            $macro_name! {
                _8 _7 _6
                _5 _4 _3 _2 _1 _0
            }
            $macro_name! {
                _9 _8 _7 _6
                _5 _4 _3 _2 _1 _0
            }
            $macro_name! {
                _10 _9 _8 _7 _6
                _5 _4 _3 _2 _1 _0
            }
            $macro_name! {
                _11 _10 _9 _8 _7 _6
                _5 _4 _3 _2 _1 _0
            }
        )}
    },
}
pub(crate) use feed_all;

macro_rules! max {() => ("11")}
pub(crate) use max;
