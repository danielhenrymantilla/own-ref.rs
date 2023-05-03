pub
struct Unsafe(
    (),
);

impl Unsafe {
    /// # Safety
    ///
    /// None (only [`crate::own_ref!`] knows how to use this properly).
    ///
    /// Are you a macro?
    ///
    ///   - <details><summary>No</summary>
    ///     I didn't think so.
    ///     </details>
    ///
    ///   - <details><summary>Yes</summary>
    ///     🤨
    ///
    ///     OK, but are you _that_ macro?
    ///
    ///     <details><summary>…</summary>
    ///     I didn't think so.
    ///     </details>
    ///
    ///     </details>
    pub
    unsafe
    fn token()
      -> Self
    {
        Self(())
    }
}
