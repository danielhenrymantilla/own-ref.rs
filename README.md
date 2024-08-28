Rust is notorious for its pervasive and ubiquitous `T`, `&mut T`, `&T` triptic set of types, or more generally, of patterns.

  - For instance, the typical receivers for a method, and with which `.` "autoref" semantics can interact, are `self`, `&mut self`, and `&self`.

  - In turn, this also leads to traits themselves embodying this paradigm too:

    ```rust
    # r#"
    // an Fn is also FnMut
          Fn     :   FnMut   :     FnOnce
    //            an FnMut is also FnOnce
    # "#
    ```

    Which stems from:

    ```rust
    # r#"
    // &self  ← &mut self ←  self
          Fn  ⊆   FnMut   ⊆  FnOnce
    # "#
    ```

      - (this inversion of the flow ought to be reminiscent, to the astute reader, of the contravariance in function arg position (here, method receiver), even if we cannot talk of subtyping w.r.t. `mut self` _vs._ `&mut self`.)

But `T`, `&mut T`, and `&T` are not the only types in these categories.

pattern, mind shift // WTF did I mean here?

Rust's type system is notorious for putting types in one of the following three boxes or categories:

  - _Ownership_, such as `T`,
  - _Exclusive borrow_, such as `&mut T`.
    - More recently, I've found myself talking of _Usufruct_, here.
  - _Shared borrow_, such as `&T`.

<!-- picture of the `T, &mut T, &T` troika -->

Also, many people here conflate ownership with the property of being `: 'static`, _i.e._, of being `: UsableFor<'forever>`, sort to speak.

Indeed, the converse is kind of right: a `&T` or `&mut T` borrow is almost never long-lived, since having a long-lived borrow kind of defeats the point of having a borrow altogether. For instance, you will very rarely see a function expecting _exactly_ a `&'static [u8]`; they will more often take any `&[u8]`.

```rust
# r#"
&'short [mut] Thing :! 'static
# "#
```

But now consider the following types:

 1. `Box<&'short str>`,

 1. `Box<dyn 'short + Display>`

 3. `&'static mut i32`

What can we say about them?

 1. A `Box<&'short str>`, is basically a boxed borrow, so it could be perceived in the borrow category. But since it is `Box`ed, it cannot be `Copy`ed, and more generally, it will have a meaningful "destructor" / drop glue (one releasing the heap-allocated `Box` pointee), _i.e._, it has meaningful _ownership_.

    So, despite its `'short`-lived-ness (something very much _not_ `: 'static`) a `Box<&'short str>` is very much an owned type.

 1. A `Box<dyn 'short + Debug>` boils down to the same reasoning as with the `Box<&'short str>`, except for not knowing exactly _what_ is in the box: it could be a `&'short str`, for instance, or actually a fully `: 'static` type, such as `i32`, or a combination of these:

    ```rust
    # r#"
    Box<&'short str> = Box<impl 'short + Debug>
     can be coërced to Box<dyn  'short + Debug>

    Box<(&'short str, i32)> = Box<impl 'short + Debug>
            can be coërced to Box<dyn  'short + Debug>

    Box<(String, i32)> = Box<impl 'short + Debug>
       can be coërced to Box<dyn  'short + Debug>

    # "#
    ```

    (for that last example, a `(String, i32)` is usable forever, _i.e._, it is `: UsableFor<'forever>`, _a fortiori_ it is usable for any `'short`er duration `: UsableFor<'short>`, _i.e._, `= impl 'short + …`.)

    So we have, yet again, an owned and yet non-`: 'static`, _i.e._, non-`: 'forever`, _i.e._, non-`: UsableFor<'forever>` type.

 1. The family of `&'static mut …` types, such as `&'static mut dyn FnMut()` is quite interesting too.

    Indeed, they have to be among the `&mut …` category of borrows.

    But there is something you can do with the general category of `&mut …` that you cannot do with `&'static mut …`s: you can re-borrow a `&'short mut T` kind of borrow down into a `&'shorter mut T` borrow, and go back to using the original `&'short mut T` once `'shorter` has ended,

    ```rust
    # r#"
    let short: &mut i32 = &mut 42;
    {
        let shorter: &mut i32 = short; // a `'shorter` reborrow!
        …
        stuff(shorter);
    }
    stuff(short) // OK
    # "#
    ```

    But the moment you constrain `'short` to be `'forever = 'static`, as well as `&'shorter = 'static` (in order to remain using `&'static mut …` types exclusively), then it turns out you don't really have reborrowing semantics anymore, since now we are requiring that the `'shorter` reborrow last for as long as the original `'short` borrow: if they span the same, then the original `'short` borrow never gets to be re-usable again:

    ```rust
    # r#"
        type R = &'static mut i32;

        let short: R = Box::leak(Box::new(42)); // OK
        {
            let shorter: R = short; // is this a reborrow?
            …
            stuff(shorter);
        }
        stuff(short); // Error! "Use of moved value" or "value
                      // used while still borrowed"!
    # "#
    ```

    So, since `&mut …` is not `Copy`, and since we don't get actual reborrowing semantics because of the "every lifetime mut be `'static`" constraint, in practice it means we are back to good old move/"single owner" semantics, _i.e._, to ownership.

    That is, if we replaced the `&'static mut i32`s above with `Box<i32>` (by removing the `Box::leak`), we wouldn't end up with code any more restrictive nor lenient:

    ```rust
    # r#"
     // type R = &'static mut i32;
        type R = Box<i32>;

        let short: R = Box::new(42); // OK
        {
            let shorter: R = short; // move!
            …
            stuff(shorter);
        }
        stuff(short); // Error! "Use of moved value".
    # "#
    ```

I hope all these examples help shatter a bit the simplified "`: 'static` = ownership ≠ borrowing" mindset.

  - Ownership is primarily about `move` semantics, which are very often motivated by the ability and responsibility to be running the drop glue;

  - Borrowing is, ultimately, behind most non-`: 'static` restrictions; but that does not mean that some more complex or compound type built atop a borrow cannot be, then, imbued with extra ownership semantics.

---

Now, let's go back to the `Box<T>` example, but this time remembering that a `Box` may involve a non-global allocator, and, at least conceptually, that said allocator may be non-`: 'static`:

```rust
# r#"
Box<T, A>
where
    A : Allocator,
    A : ?'static, // <- pseudo-code

# "#
```

And let's consider a special form of `Allocator`: a simple one-shot allocation "arena".

```rust
# r#"
struct Slot<T>(Option<T>);

// pseudo-code
impl<T> Slot<T> {
    fn alloc<'slot>(&'slot mut self, value: T)
      -> ptr::NonNull<T>
    {
        assert!(self.0.is_none(), "can only alloc at most one item!");
        let r: &'slot mut T = self.0.insert(value);
        r.into()
    }

    fn deälloc(&mut self, _ptr: ptr::NonNull<T>)
    {
        // nothing needed, the backing memory of that `*_ptr` is in no
        // managed heap or memory-map, it's just inside of us
        // (inside of `*self`), so its reclamation goes above our head, literally.
    }
}
# "#
```

or, for future reference, let's consider a `Slot` behind a `&mut` borrow:

```rust
# r#"
type OutSlot<'storage, T> = &'storage mut Slot<T>;

// pseudo-code
impl OutSlot<'_, T> {
    fn alloc(&mut self, value: T) -> &mut T {
        assert!(self.0.is_none(), "can only alloc at most one item!");
        self.0.insert(value)
    }
}
# "#
```

And now think of the type `Box<T, OutSlot<'short, T>>` (_each_ of these `Box`es will be using their own, dedicated, single-item/one-shot allocator, but all of them are `'short`-lived one way or another).

```rust
# r#"
'local: {
    let mut slot = Slot(None);
    let b = Box::new_in(42, &mut slot);
    …
} // <- b cannot be used beyond this point
# "#
```

Now our `b` instance is some owning `Box`, only, one which isn't really living in the heap, but rather, something which is using _borrowed_ `'local` storage.

In fact, we may be tempted to talk of "stack storage" rather than "local storage", since, traditionally, the local storage of a `fn` lives in the stack[^stack].

[^stack]: even though this assumption breaks when dealing with `Coroutine`s/`Generator`s which `yield`, such as `Future`s which `.await`, as in the body of an `async fn` or an `async {}` block: the local storage living through `yield/.await` points is then stored within the state machine itself, which may very well have been heap-allocated while polled.

We could thus define:

```rust
# r#"
type StackBox<'local, T> = Box<T, &'local mut Slot<T>>;
# "#
```

And in fact, modulo reïnventing the whole API, rather than piggybacking off `Box`'s, this is exactly what the [`::stackbox`](https://docs.rs/stackbox) crate is all about!

  - Note that whilst I have been using `Option<T>` to very naïvely and simply implement the `Slot<T>` logic, the reality is that precisely thanks to the ownership semantics of a `Box`, it is possible to skip the `Option`'s drop-tagging discriminant, and to be using an unchecked discriminant-less `Option<T>`: a `MaybeUninit<T>`.

---

Now, this all seems like Just Another Third-Party Type™: there could be a whole myriad of `Box<T, …>` which can be designed merely by playing with its `Allocator`, or rather, with its backing `Storage`[^storage].

[^storage]: there is, somewhere some proposal for a nicer API design than `Allocator`, precisely to allow for expressing these very useful things in a way less wave-handed fashion.

But for three rather important observations:

  - ### Helper temporary storage elision

    What if the language were to offer some ergonomic way to define a sufficiently long-lived vacant `Slot<T>`? And automatically using it when attempting a `StackBox` construction?

    ```rust
    # r#"
    'local: {
        let b = StackBox::new_in(42, auto_slot!('local));
        …
    } // <- b cannot be used beyond this point
    # "#
    ```

    or even just:

    ```rust
    # r#"
    {
        let b = stackbox!(42);
        …
    } // <- b cannot be used beyond this point
    # "#
    ```

  - ### A fully `::core`/no`::alloc`-compatible abstraction

    Now focus on what we have needed to construct this. Say we have some type `T`, and some `value: T` (you can consider `T = Box<str>`, even if `Box<str>`, in and of itself, does need `alloc`).

    First, we need some "inlined"/local backing storage:

    ```rust
    # r#"
    let mut slot: Slot<T> = Slot(MaybeUninit::uninit()); // no magic
    # "#
    ```

    And then:

    ```rust
    # r#"
    let boxed: Box<T, &mut Slot<T>> =
        Box::new_in(value, &mut slot)
    ;
    // equivalent to:
    let ref_: &mut T = slot.0.write(value);
    let boxed: impl 'slot + DerefMut<Target = T> =
        ::scopeguard::guard(
            // Owns and DerefMuts to:
            ref_,
            |ref_: &mut T| /* on drop, do */ unsafe {
                <*mut T>::drop_in_place(r)
            },
        )
    ;
    # "#
    ```

    As you can see, no magic `Box` or heap allocation or memory management shenanigans whatsoever are involved. Just a simply 3-step logic:

     1. `MaybeUninit::uninit()` to reserve the backing space/memory/storage wherein the `value` shall be held; any local variable can be holding it (meaning the memory itself shall be local as well, called the `slot`);

     1. `MaybeUninit::write()` to write the value therein: we have our `impl 'slot + DerefMut<Target = T>`!

     1. `<*mut T>::drop_in_place()`, eventually, so as to make sure the written `T` is properly, itself, "reclaimed". Meaning, that its own drop glue is run / that the resources it itself owns (_e.g._, when `T = Box<str>`, the heap-allocated `str`) are properly reclaimed.

        This can simply happen as part of the extra `Drop` glue of our `impl DerefMut…`: we got our `StackBox`!

    And _voilà_!

  - ### Ownership + indirection: `::core`/no`::alloc` _owned_ `dyn Trait`s

     1. Have you ever wanted to write:

        ```rust
        # r#"
        trait MyDynSafeApi {
            fn method(&self, f: impl FnOnce());
        }

        /// Assert `dyn`-safe.
        impl dyn MyDynSafeApi {} // Error, method cannot be generic!
        # "#
        ```

        which runs into `dyn`-safety issues?

     1. And then you decide to replace `impl` with `dyn` to solve it, only to then be running into:

        ```rust
        # r#"
        trait MyDynSafeApi {
            // Error, `dyn FnOnce()` is not `Sized`!
            fn method(&self, f: dyn FnOnce());
        }

        /// Assert `dyn`-safe.
        impl dyn MyDynSafeApi {}
        # "#
        ```

     1. At that point, rustaceans have two options:

          - either they involve the _heap_, which is so absurd when you come to think of it, and straight up impossible when you are in a `::core`/no`::alloc` context;

          - or you give up the move semantics and require `&mut dyn FnMut()`. You can always "get ownership back" by using `Option` + `.take()` (which means that misusage of the desired move semantics, now, rather than leading to a compile error, causes panics!)

    But assuming callers had access to this very ergonomic, and `::core`/no`::alloc`-compatible, `StackBox` abstraction, we could have:

    ```rust
    # r#"
    trait MyDynSafeApi {
        fn method(&self, f: StackBox<'_, dyn FnOnce()>);
    }

    /// Assert `dyn`-safe.
    impl dyn MyDynSafeApi {} // ✅

    fn example(it: &dyn MyDynSafeApi, mutex: &Mutex<…>)
    {
        let mut guard = mutex.lock();
        it.method(stackbox!(move || {
            stuff(&mut *guard);
            drop(guard); // free the mutex on the first call
        }))
    }
    # "#
    ```

And this last `dyn` example only starts to scratch the surface of all the things you can do w.r.t. "owned `dyn Trait`s" once you have access to this "`StackBox`".

---

All in all, we have:

  - a genuinely useful type to handle heapless _owned_ `dyn Trait`s —among other things that like to be used behind indirection, such as `Sized` but _huuge_ arrays or structs, or whatnot—,

  - and with an abstraction which, at the mere cost of some language-blessed sugar to take care of the lifetime of the (local) backing memory (the `slot`s), does not have to involve heap shenanigans.

And, in fact, when we paper a bit over the lifetime of the local backing storage, what we have, at the end of the day, is: _raw ownership through indirection_.

If we think of `&` as the (by) _reference_ operator, _i.e._, the (borrowing) operator of indirection, then, what we have could be called an _owning reference_.

This could then become a language-blessed construct, becoming, w.r.t. the Rust trifecta of `T`, `&mut T`, `&T`, the by-reference-version of `T : ?Sized`:

> **`&own T`**, the owning reference.

More precisely, `&'slot own T`, wherein the backing storage of that `T` is `&'slot mut` borrowed, but offering fully owned access to the `T` pointee (_e.g._, the possibility to drop the `T` at any point, or, when `T : Sized`, to even _move_ the value out of it).

<details class="custom"><summary><span class="summary-box"><span>Click to show</span></span></summary>

Foo.

</details>
