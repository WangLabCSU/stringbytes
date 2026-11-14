# stringbytes

[<img alt="github" src="https://img.shields.io/badge/github-WangLabCSU/stringbytes-8da0cb?style=for-the-badge&labelColor=555555&logo=github" height="20">](https://github.com/WangLabCSU/stringbytes)
[<img alt="crates.io" src="https://img.shields.io/crates/v/stringbytes.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/stringbytes)
[<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-stringbytes-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs" height="20">](https://docs.rs/stringbytes)
[<img alt="build status" src="https://img.shields.io/github/actions/workflow/status/WangLabCSU/stringbytes/ci.yml?branch=main&style=for-the-badge" height="20">](https://github.com/WangLabCSU/stringbytes/actions?query=branch%3Amain)

<!-- Generated from src/lib.rs. Edit the crate documentation there, then run cargo rdme. -->

<!-- cargo-rdme start -->

`stringbytes` provides a common interface for string and byte-buffer storage.

The [`StringBytes`](https://docs.rs/stringbytes/latest/stringbytes/trait.StringBytes.html) trait can be used as a bound on the storage parameter of
generic string and byte-buffer types.

Both borrowed and owned storage are supported, including shared buffers.

Implementations expose a consistent byte sequence. Byte conversions and
successful checked string conversions preserve that sequence.
The bytes need not be valid UTF-8; validation is performed when needed to
borrow them as [`&str`](https://doc.rust-lang.org/stable/std/primitive.str.html) or convert them into a [`String`](https://doc.rust-lang.org/stable/alloc/string/struct.String.html).
Unchecked conversions let callers uphold UTF-8 validity themselves.
Lossy conversions follow the same behavior as [`String::from_utf8_lossy`](https://doc.rust-lang.org/stable/alloc/string/struct.String.html#method.from_utf8_lossy).

## Features

Enable `simd` to use SIMD-accelerated UTF-8 validation where supported.
This feature is disabled by default. Both configurations return the same
[`Utf8Error`](https://docs.rs/stringbytes/latest/stringbytes/error/struct.Utf8Error.html) and [`FromUtf8Error`](https://docs.rs/stringbytes/latest/stringbytes/error/struct.FromUtf8Error.html) types.

## Examples

### `&[u8]`

```rust
use std::borrow::Cow;
use stringbytes::StringBytes;

let bytes: &[u8] = b"hello";
assert_eq!(bytes.as_bytes(), b"hello");

// Validate UTF-8 and borrow a string slice without copying.
assert_eq!(bytes.try_borrow_str()?, "hello");

// SAFETY: the bytes come from a UTF-8 string literal.
let text = unsafe { bytes.borrow_str_unchecked() };
assert_eq!(text, "hello");

// Lossy conversion borrows valid UTF-8 and replaces invalid UTF-8 as needed.
assert!(matches!(bytes.to_string_lossy(), Cow::Borrowed("hello")));
let invalid: &[u8] = b"he\xfflo";
assert_eq!(invalid.to_string_lossy(), "he�lo");

// SAFETY: bytes still contains the UTF-8 literal above; copy without validation.
let string = unsafe { bytes.to_string_unchecked() };
assert_eq!(string, "hello");
```

### `Vec<u8>`

```rust
use stringbytes::StringBytes;

let bytes = b"hello".to_vec();
let address = bytes.as_ptr();

// Validate UTF-8 and transfer the vector's allocation to the string.
let string = bytes.try_into_string()?;
assert_eq!(string, "hello");
assert_eq!(string.as_ptr(), address);

// Convert back to bytes, keeping the same allocation.
let bytes = StringBytes::into_bytes(string);
assert_eq!(bytes, b"hello");
assert_eq!(bytes.as_ptr(), address);

// SAFETY: these bytes came from a String and have not been modified.
let string = unsafe { bytes.into_string_unchecked() };
assert_eq!(string, "hello");
assert_eq!(string.as_ptr(), address);
```

### `Box<[u8]>`

```rust
use stringbytes::StringBytes;

let bytes = Box::<[u8]>::from("hello".as_bytes());

// Borrow the bytes as a string slice without copying.
assert_eq!(bytes.try_borrow_str()?, "hello");

// Copy the bytes, keeping the original value.
assert_eq!(bytes.try_to_string()?, "hello");

// Consume the box and reuse its allocation.
assert_eq!(bytes.try_into_string()?, "hello");
```

### `&str`

```rust
use stringbytes::StringBytes;

let text: &str = "hello";

// Copy the borrowed string without checking UTF-8 again.
let string = text.try_to_string()?;
assert_eq!(string, text);
```

### `String`

```rust
use stringbytes::StringBytes;

let string = String::from("hello");
let address = string.as_ptr();
let capacity = string.capacity();

// Copy the bytes, keeping the string available.
let bytes = string.to_bytes();
assert_eq!(bytes, b"hello");

// Consume the string, keeping its allocation and capacity.
let string = string.try_into_string()?;
assert_eq!(string, "hello");
assert_eq!(string.as_ptr(), address);
assert_eq!(string.capacity(), capacity);
```

### `&CStr`

```rust
use stringbytes::StringBytes;

let text = c"hello";

// C strings expose their contents without the trailing NUL.
assert_eq!(text.try_borrow_str()?, "hello");

// CStr::to_bytes borrows a slice; the trait method copies into a vector.
let bytes: Vec<u8> = StringBytes::to_bytes(text);
assert_eq!(bytes, b"hello");
```

### `CString`

```rust
use std::ffi::CString;
use stringbytes::StringBytes;

let text = CString::from(c"hello");
let address = text.as_bytes().as_ptr();

// Validate UTF-8 and transfer the buffer, excluding the trailing NUL.
let string = text.try_into_string()?;
assert_eq!(string, "hello");
assert_eq!(string.as_ptr(), address);

// C strings may also contain invalid UTF-8.
let text = CString::from(c"he\xfflo");
assert_eq!(text.into_string_lossy(), "he�lo");
```

<!-- cargo-rdme end -->
