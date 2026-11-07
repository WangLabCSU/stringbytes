# stringbytes

[<img alt="github" src="https://img.shields.io/badge/github-WangLabCSU/stringbytes-8da0cb?style=for-the-badge&labelColor=555555&logo=github" height="20">](https://github.com/WangLabCSU/stringbytes)
[<img alt="crates.io" src="https://img.shields.io/crates/v/stringbytes.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/stringbytes)
[<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-stringbytes-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs" height="20">](https://docs.rs/stringbytes)
[<img alt="build status" src="https://img.shields.io/github/actions/workflow/status/WangLabCSU/stringbytes/ci.yml?branch=main&style=for-the-badge" height="20">](https://github.com/WangLabCSU/stringbytes/actions?query=branch%3Amain)

A trait for byte sequences and UTF-8 strings.

`StringBytes` can be used as a bound on the storage parameter of generic string
and byte-buffer types. It is implemented for arrays, slices, strings, vectors,
and shared buffers.

Implementations expose a consistent byte sequence and preserve it when
converting to `str` or `String`. The bytes need not be valid UTF-8; validation
is performed when needed to borrow them as `&str` or convert them into a `String`.

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

| Method              | Result                          | Behavior                                                      |
| ------------------- | ------------------------------- | ------------------------------------------------------------- |
| `as_bytes()`        | `&[u8]`                         | Borrows bytes without allocating, copying, or checking UTF-8. |
| `try_borrow_str()`  | `Result<&str, Utf8Error>`       | Borrows as `&str`, validating UTF-8 when needed.              |
| `try_to_string()`   | `Result<String, FromUtf8Error>` | Copies into a new `String`, leaving the source unchanged.     |
| `try_into_string()` | `Result<String, FromUtf8Error>` | Consumes the value and reuses its allocation when possible.   |

Passing a reference to `try_into_string()` copies the bytes and leaves the
referenced value unchanged.

For byte sequences, each conversion to `&str` or `String` validates UTF-8
once.

For `str`, `String`, and `Cow<'_, str>`, the bytes are already valid UTF-8.
All three string conversion methods skip validation and always return `Ok`,
including through references and smart pointers to these types.

## Supported types

The table shows when `try_into_string()` reuses an allocation;
`try_to_string()` always copies. References and smart pointers support
`T: StringBytes + ?Sized` and can be nested.

For unsized `[u8]` and `str`, the table lists the references and owning pointers
used for `try_into_string()`.

| Type                             | Reuses allocation | Conditions                                                          |
| -------------------------------- | ----------------- | ------------------------------------------------------------------- |
| `&[u8]`                          | No                | Copies the borrowed bytes into a new `String`.                      |
| `[u8; N]`                        | No                | Copies the inline bytes.                                            |
| `Vec<u8>`                        | **Yes**           | Transfers the existing buffer.                                      |
| `Cow<'_, [u8]>`                  | **Conditional**   | Owned values reuse their buffer; borrowed values are copied.        |
| `Box<[u8]>`, `Box<[u8; N]>`      | **Yes**           | Transfers the boxed allocation.                                     |
| `Rc<[u8]>`, `Arc<[u8]>`          | No                | Copies even with unique ownership.                                  |
| `&str`                           | No                | Copies the borrowed string into a new `String`.                     |
| `String`                         | **Yes**           | Transfers the existing buffer.                                      |
| `Cow<'_, str>`                   | **Conditional**   | Owned values reuse their buffer; borrowed values are copied.        |
| `Box<str>`                       | **Yes**           | Transfers the boxed allocation.                                     |
| `Rc<str>`, `Arc<str>`            | No                | Copies even with unique ownership.                                  |
| `&T`, `&mut T`                   | No                | Copies the referenced data, leaving the source unchanged.           |
| `Box<T>` for other sized `T`     | **Follows `T`**   | Uses the contained value's conversion.                              |
| `Rc<T>`, `Arc<T>` with sized `T` | **Conditional**   | Unique ownership uses `T`'s conversion; shared contents are copied. |

For sized `T`, `Rc<T>` and `Arc<T>` take ownership of the inner value when only
one strong reference remains, using
[`Rc::try_unwrap`](https://doc.rust-lang.org/std/rc/struct.Rc.html#method.try_unwrap)
or [`Arc::try_unwrap`](https://doc.rust-lang.org/std/sync/struct.Arc.html#method.try_unwrap);
`Weak` pointers do not prevent this. Conversion then follows the inner type's
rules. If another strong reference remains, the bytes are copied:

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

Enable the `bytes` feature to use `StringBytes` with `bytes::Bytes` and
`bytes::BytesMut`. This feature is disabled by default.

`try_into_string()` converts these types through `Vec<u8>`, reusing the allocation
when possible. `Bytes` copies static data, shared buffers, and storage supplied
by `from_owner`. `BytesMut` copies while its buffer is shared. Reusing a sliced
buffer may move bytes within the same allocation.
