# stringbytes

[<img alt="github" src="https://img.shields.io/badge/github-WangLabCSU/stringbytes-8da0cb?style=for-the-badge&labelColor=555555&logo=github" height="20">](https://github.com/WangLabCSU/stringbytes)
[<img alt="crates.io" src="https://img.shields.io/crates/v/stringbytes.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/stringbytes)
[<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-stringbytes-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs" height="20">](https://docs.rs/stringbytes)
[<img alt="build status" src="https://img.shields.io/github/actions/workflow/status/WangLabCSU/stringbytes/ci.yml?branch=main&style=for-the-badge" height="20">](https://github.com/WangLabCSU/stringbytes/actions?query=branch%3Amain)

<!-- Generated from src/lib.rs. Edit the crate documentation there, then run cargo rdme. -->

<!-- cargo-rdme start -->

`stringbytes` provides a common interface for string and byte-buffer storage.

The [`StringBytes`](https://docs.rs/stringbytes/latest/stringbytes/trait.StringBytes.html) trait can be used as a bound on the storage parameter of
generic string and byte-buffer types. It is implemented for arrays, slices,
strings, vectors, and shared buffers.

Implementations expose a consistent byte sequence and preserve it when
converting to [`str`](https://doc.rust-lang.org/stable/std/primitive.str.html) or [`String`]. The bytes need not be valid UTF-8;
validation is performed when needed to borrow them as [`&str`](https://doc.rust-lang.org/stable/std/primitive.str.html) or
convert them into a [`String`].

```rust
use stringbytes::StringBytes;

let bytes = Box::<[u8]>::from("hello".as_bytes());
assert_eq!(bytes.try_borrow_str().unwrap(), "hello");

// Copy the bytes, keeping the original value.
assert_eq!(bytes.try_to_string().unwrap(), "hello");

// Consume the box and reuse its allocation.
assert_eq!(bytes.try_into_string().unwrap(), "hello");
```

## Methods

| Method                                 | Result                          | Behavior                                                      |
| -------------------------------------- | ------------------------------- | ------------------------------------------------------------- |
| [`as_bytes()`][as_bytes]               | [`&[u8]`][slice]                | Borrows bytes without allocating, copying, or checking UTF-8. |
| [`try_borrow_str()`][try_borrow_str]   | `Result<&str, Utf8Error>`       | Borrows as [`&str`][str], validating UTF-8 when needed.       |
| [`try_to_string()`][try_to_string]     | `Result<String, FromUtf8Error>` | Copies into a new [`String`], leaving the source unchanged.   |
| [`try_into_string()`][try_into_string] | `Result<String, FromUtf8Error>` | Consumes the value and reuses its allocation when possible.   |

Passing a reference to [`try_into_string()`][try_into_string]
copies the bytes and leaves the referenced value unchanged.

For byte sequences, each conversion to [`&str`](https://doc.rust-lang.org/stable/std/primitive.str.html) or [`String`]
validates UTF-8 once.

For [`str`](https://doc.rust-lang.org/stable/std/primitive.str.html), [`String`], and [`Cow<'_, str>`](https://doc.rust-lang.org/stable/alloc/borrow/enum.Cow.html), the bytes are already
valid UTF-8. All three string conversion methods skip validation and always
return [`Ok`](https://doc.rust-lang.org/stable/core/result/enum.Result.html#variant.Ok), including through references and smart pointers to these types.

## Supported types

The table shows when [`try_into_string()`][try_into_string] reuses an
allocation; [`try_to_string()`][try_to_string] always copies.
References and smart pointers support `T: StringBytes + ?Sized` and can be nested.

For unsized [`[u8]`](https://doc.rust-lang.org/stable/std/primitive.slice.html) and [`str`](https://doc.rust-lang.org/stable/std/primitive.str.html), the table lists the
references and owning pointers used for [`try_into_string()`][try_into_string].

| Type                                          | Reuses allocation | Conditions                                                          |
| --------------------------------------------- | ----------------- | ------------------------------------------------------------------- |
| [`&[u8]`][slice]                              | No                | Copies the borrowed bytes into a new [`String`].                    |
| [`[u8; N]`][array]                            | No                | Copies the inline bytes.                                            |
| [`Vec<u8>`][Vec]                              | **Yes**           | Transfers the existing buffer.                                      |
| [`Cow<'_, [u8]>`][Cow]                        | **Conditional**   | Owned values reuse their buffer; borrowed values are copied.        |
| [`Box<[u8]>`][Box], [`Box<[u8; N]>`][Box]     | **Yes**           | Transfers the boxed allocation.                                     |
| [`Rc<[u8]>`][Rc], [`Arc<[u8]>`][Arc]          | No                | Copies even with unique ownership.                                  |
| [`&str`][str]                                 | No                | Copies the borrowed string into a new [`String`].                   |
| [`String`]                                    | **Yes**           | Transfers the existing buffer.                                      |
| [`Cow<'_, str>`][Cow]                         | **Conditional**   | Owned values reuse their buffer; borrowed values are copied.        |
| [`Box<str>`][Box]                             | **Yes**           | Transfers the boxed allocation.                                     |
| [`Rc<str>`][Rc], [`Arc<str>`][Arc]            | No                | Copies even with unique ownership.                                  |
| [`&T`][reference], [`&mut T`][reference]      | No                | Copies the referenced data, leaving the source unchanged.           |
| [`Box<T>`][Box] for other sized `T`           | **Follows `T`**   | Uses the contained value's conversion.                              |
| [`Rc<T>`][Rc], [`Arc<T>`][Arc] with sized `T` | **Conditional**   | Unique ownership uses `T`'s conversion; shared contents are copied. |

For sized `T`, [`Rc<T>`](https://doc.rust-lang.org/stable/alloc/rc/struct.Rc.html) and [`Arc<T>`](https://doc.rust-lang.org/stable/alloc/sync/struct.Arc.html) take ownership of the inner value when only
one strong reference remains, using [`Rc::try_unwrap`](https://doc.rust-lang.org/stable/alloc/rc/struct.Rc.html#method.try_unwrap) or [`Arc::try_unwrap`](https://doc.rust-lang.org/stable/alloc/sync/struct.Arc.html#method.try_unwrap);
[`rc::Weak`](https://doc.rust-lang.org/stable/alloc/rc/struct.Weak.html) and [`sync::Weak`](https://doc.rust-lang.org/stable/alloc/sync/struct.Weak.html) pointers do not
prevent this. Conversion then follows the inner type's rules. If another strong
reference remains, the bytes are copied:

```rust
use std::rc::Rc;
use stringbytes::StringBytes;

let storage = Rc::new(String::from("hello"));
let address = storage.as_bytes().as_ptr();

let copy = Rc::clone(&storage).try_into_string().unwrap();
assert_eq!(copy, "hello");
assert_ne!(copy.as_ptr(), address);

let weak = Rc::downgrade(&storage);
let string = storage.try_into_string().unwrap();
assert_eq!(string.as_ptr(), address);
assert!(weak.upgrade().is_none());
```

## The `bytes` feature

Enable the `bytes` feature to use [`StringBytes`](https://docs.rs/stringbytes/latest/stringbytes/trait.StringBytes.html) with `bytes::Bytes` and
`bytes::BytesMut`. This feature is disabled by default.

[`try_into_string()`][try_into_string] converts these types through
[`Vec<u8>`](https://doc.rust-lang.org/stable/alloc/vec/struct.Vec.html), reusing the allocation when possible. `Bytes` copies static data,
shared buffers, and storage supplied by `from_owner`. `BytesMut` copies while
its buffer is shared. Reusing a sliced buffer may move bytes within the same
allocation.

[as_bytes]: https://docs.rs/stringbytes/latest/stringbytes/trait.StringBytes.html#tymethod.as_bytes
[slice]: https://doc.rust-lang.org/stable/std/primitive.slice.html
[try_borrow_str]: https://docs.rs/stringbytes/latest/stringbytes/trait.StringBytes.html#method.try_borrow_str
[str]: https://doc.rust-lang.org/stable/std/primitive.str.html
[try_to_string]: https://docs.rs/stringbytes/latest/stringbytes/trait.StringBytes.html#method.try_to_string
[`String`]: https://doc.rust-lang.org/stable/alloc/string/struct.String.html
[try_into_string]: https://docs.rs/stringbytes/latest/stringbytes/trait.StringBytes.html#method.try_into_string
[array]: https://doc.rust-lang.org/stable/std/primitive.array.html
[Vec]: https://doc.rust-lang.org/stable/alloc/vec/struct.Vec.html
[Cow]: https://doc.rust-lang.org/stable/alloc/borrow/enum.Cow.html
[Box]: https://doc.rust-lang.org/stable/alloc/boxed/struct.Box.html
[Rc]: https://doc.rust-lang.org/stable/alloc/rc/struct.Rc.html
[Arc]: https://doc.rust-lang.org/stable/alloc/sync/struct.Arc.html
[reference]: https://doc.rust-lang.org/stable/std/primitive.reference.html

<!-- cargo-rdme end -->
