Rust is notorious for its pervasive and ubiquitous `T`, `&mut T`, `&T` triptic set of types, or more generally, of patterns.

  - For instance, the typical receivers for a method, and with which `.` "autoref" semantics can interact, are `self`, `&mut self`, and `&self`.

  - In turn, this also leads to traits themselves embodying this paradigm too:

    ```rs
    // an Fn is also FnMut
          Fn     :   FnMut   :     FnOnce
    //            an FnMut is also FnOnce
    ```

    Which stems from:

    ```rs
    // &self  ← &mut self ←  self
          Fn  ⊆   FnMut   ⊆  FnOnce
    ```

      - (this inversion of the flow ought to be reminiscent, to the astute reader, of the contravariance in function arg position (here, method receiver), even if we cannot talk of subtyping w.r.t. `mut self` _vs._ `&mut self`.)

But `T`, `&mut T`, and `&T` are not the only types in these categories.

pattern, mind shift // WTF did I mean there?

Rust's type system is notorious for putting types in one of the following three boxes or categories:

  - _Ownership_, such as `T`,
  - _Exclusive borrow_, such as `&mut T`.
    - More recently, I've found myself talking of _Usufruct_, here.
  - _Shared borrow_, such as `&T`.

<!-- picture of the `T, &mut T, &T` troika -->

Also, many people here conflate ownership with the property of being `: 'static`, _i.e._, of being `: UsableFor<'forever>`, sort to speak.

Indeed, the "contraposée" is kind of right: if you have a `&T` or `&mut T` borrow, these are almost never long-lived enough, since having a long-lived borrow kind of defeats the point of having a borrow altogether. For instance, you will very rarely see a function expecting _exactly_ a `&'static str`: they will more often take any `&str`.

```rs
&'short [mut] Thing :! 'static
```

But now consider the following types:

 1. `Box<&'short str>`,

 1. `Box<dyn 'short + Display>`

 3. `&'static mut i32`

What can we say about them?

 1. A `Box<&'short str>`, is basically a boxed borrow, so it could be perceived in the borrow category. But since it is `Box`ed, it cannot be `Copy`ed, and more generally, it will have a meaningful "destructor" / drop glue (that releasing the heap-allocated `Box` pointee), _i.e._, it has meaningful _ownership_.

    So, despite its `'short`-lived-ness (something very much _not_ `: 'static`) a `Box<&'short str>` is very much an owned type.

 1. A `Box<dyn 'short + Debug>` is very much the same as a `Box<&'short str>`, except for not knowing exactly _what_ is in the box: it could be a `&'short String`, for instance, or actually a fully `: 'static` `i32`, or a combination of these:

    ```rs
    Box<&'short str> = Box<impl 'short + Debug>
      can be coërced to Box<dyn 'short + Debug>

    Box<(&'short str, i32)> = Box<impl 'short + Debug>
             can be coërced to Box<dyn 'short + Debug>

    Box<(String, i32)> = Box<impl 'short + Debug>
        can be coërced to Box<dyn 'short + Debug>
    ```

    (for that last example, a `(String, i32)` is usable forever, _i.e._, it is `: UsableFor<'forever>`, _a fortiori_ it is usable for any `'short`er duration `: UsableFor<'short>`, _i.e._, `= impl 'short + …`.)

    So we have, yet again, an owned and yet non-`: 'static`, _i.e._, non-`: 'forever`, _i.e._, non-`: UsableFor<'forever>` type.

 1. The family of `&'static mut …` types, such as `&'static mut dyn FnMut()` is quite interesting too.

    Indeed, they have to be among the `&mut …` category of borrows.

    But there is something you can do with the general category of `&mut …` that you cannot do with `&'static mut …`s: you can re-borrow a `&'short mut T` kind of borrow down into a `&'shorter mut T` borrow, and go back to using the original `&'short mut T` once `'shorter` has ended,

    ```rs
    let short: &mut i32 = &mut 42;
    {
        let shorter: &mut i32 = short; // a `'shorter` reborrow!
        // …
        stuff(shorter);
    }
    stuff(short) // OK
    ```

    But the moment you constrain `'short` to be `'forever = 'static`, as well as `&'shorter = 'static` (in order to remain using a `&'static mut …` type), then it turns out you don't really have reborrowing semantics anymore, since now we're requiring that the `'shorter` reborrow last for as long as the original `'short` borrow: if they span the same, then the original `'short` borrow never gets to be re-usable again:

    ```rs
    type R = &'static mut i32;

    let short: R = Box::leak(Box::new(42)); // OK
    {
        let shorter: R = short; // is this a reborrow?
        // …
        stuff(shorter);
    }
    stuff(short); // Error! "Use of moved value" or "value
                  // used while still borrowed"!
    ```

    So, since `&mut …` is not `Copy`, and since we don't get actual reborrowing semantics because of the "every lifetime mut be `'static`" constraint, in practice it means we are back to good old move/"single owner" semantics, _i.e._, to ownership.

    That is, if we removed the `Box::leak` above, we wouldn't be ending up with code any more lenient:

    ```rs
    type R = Box<i32>;
    let short: R = Box::new(42); // OK
    {
        let shorter: R = short; // move!
        // …
        stuff(shorter);
    }
    stuff(short); // Error! "Use of moved value".
    ```

I hope all these examples help shatter a bit the simplified "`: 'static` = ownership ≠ borrowing" mindset.

  - Ownership is primarily about `move` semantics, which are very often motivated by the ability and responsibility to be running the drop glue;

  - Borrowing is, ultimately, behind most non-`: 'static` restrictions; but that does not mean that some more complex or compound type built atop a borrow cannot be, then, imbued with extra ownership semantics.

---

Now, let's go back to the `Box<T>` example, but this time remembering that a `Box` may involve a non-global allocator, and, at least conceptually, said allocator may be non-`: 'static`:

```rs
Box<T, A : Allocator :! 'static>
```

Let's consider a special form of `Allocator`: a simple one-shot allocation "arena":

```rs
struct Slot<T>(Option<T>);

// pseudo-code
impl<T> Slot<T> {
    pub
    fn alloc<'slot>(&'slot mut self, value: T)
      -> ptr::NonNull<T>
    {
        assert!(self.0.is_none());
        let r: &'slot mut T = self.0.insert(value);
        r.into()
    }

    pub
    fn deälloc(&mut self, _: ptr::NonNull<T>)
    {
        // nothing to do, the "allocated memory" remains within us.
    }
}
```

or, for future reference, let's consider a `Slot` behind a `&mut` borrow:

```rs
type OutSlot<'storage, T> = &'storage mut Slot<T>;

// pseudoer-code
impl OutSlot<'_, T> {
    pub fn alloc(&mut self, value: T) -> &mut T {
        self.0.insert(value)
    }
}
```

And now think of the type `Box<T, OutSlot<'short, T>>` (think _each_ of these `Box`es will be using their own, dedicated, single-item/one-shot allocator, but all of them are `'short`-lived one way or another).

```rs
'local: {
    let mut slot = Slot(None);
    let b = Box::new_in(42, &mut slot);
    …
} // <- b cannot be used beyond this point
```

Now our `b` instance is some owner `Box`, only, one which isn't really living in the heap, but rather, something which is using _borrowed_ `'local` storage.

In fact, we may be tempted to talk of "stack storage" rather than "local storage", since, traditionally, the local storage of a `fn` lives in the stack (even though this assumption breaks when dealing with `Coroutine`s/`Generator`s which `yield`, such as `Future`s which `.await`, as in the body of an `async fn` or an `async {}` block).

We could then define:

```rs
type StackBox<'local, T> = Box<T, &'local mut Slot<T>>;
```

And in fact, modulo reïnventing the whole API, rather than piggybacking off `Box`'s, this is exactly what the [`::stackbox`](https://docs.rs/stackbox) crate is all about!

  - Note that whilst I have been using `Option<T>` to very naïvely and simply implement the `Slot<T>` logic, the reality is that precisely thanks to the ownership semantics of a `Box`, it should be possible to skip the `Option`'s drop-tagging discriminant, and be using an unchecked discriminant-less `Option<T>`: a `MaybeUninit<T>`.

---

Now, this all seems like Just Another Third-Party Type™: there could be a whole myriad of `Box<T, …>` which can be designed merely by playing with its `Allocator`, or rather, with its backing `Storage`[^storage].

[^storage]: there is, somewhere some proposal for a nicer API design than `Allocator`, precisely to allow for expressing these very useful things in a way less wave-handed fashion.

But for three rather important observations:

  - ### Helper temporary storage elision

    what if the language were to offer some ergonomic manner of defining a sufficiently long-lived vacant `Slot<T>`? And automatically using it when attempting a `StackBox` construction?

    ```rs
    'local: {
        let b = StackBox::new_in(42, magic!('local));
        …
    } // <- b cannot be used beyond this point
    ```

    or even just:

    ```rs
    'local: {
        let b = stackbox!(42);
        …
    } // <- b cannot be used beyond this point
    ```

    or even just:

    ```rs
    {
        let b = stackbox!(42);
        …
    } // <- b cannot be used beyond this point
    ```

  - ### A fully `::core`/no`::alloc`-compatible abstraction

  - ### Ownership + indirection: `::core`/no`::alloc` _owned_ `dyn Trait`s

<details class="custom"><summary><span class="summary-box"><span>Click to show</span></span></summary>

Foo.

</details>
