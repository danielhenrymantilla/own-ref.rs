crate::utils::match_cfg! {
    doc => {
        macro_rules! feed_all {(
            => $macro_name:ident!
        ) => (
            $macro_name! {
                _2 _1 _0
            }
        )}
    },
    _ => {
        macro_rules! feed_all {(
            => $macro_name:ident!
        ) => (
            $macro_name! {
                _11 _10 _9 _8 _7 _6
                _5  _4  _3 _2 _1 _0
            }
        )}
    },
}

pub(crate) use feed_all;
