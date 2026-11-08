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

Implementations expose a consistent byte sequence and preserve it when
borrowing as [`&str`](https://doc.rust-lang.org/stable/std/primitive.str.html) or converting into [`Vec<u8>`](https://doc.rust-lang.org/stable/alloc/vec/struct.Vec.html) or [`String`](https://doc.rust-lang.org/stable/alloc/string/struct.String.html).
The bytes need not be valid UTF-8; validation is performed when needed to
borrow them as [`&str`](https://doc.rust-lang.org/stable/std/primitive.str.html) or convert them into a [`String`](https://doc.rust-lang.org/stable/alloc/string/struct.String.html).

## Examples

### `&[u8]`

```rust
use stringbytes::StringBytes;

let bytes: &[u8] = b"hello";

// Validate UTF-8 and borrow a string slice without copying.
assert_eq!(bytes.try_borrow_str()?, "hello");
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
let bytes = string.into_bytes();
assert_eq!(bytes, b"hello");
assert_eq!(bytes.as_ptr(), address);
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

<!-- cargo-rdme end -->
