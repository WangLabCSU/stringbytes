//! `stringbytes` provides a common interface for string and byte-buffer storage.
//!
//! The [`StringBytes`] trait can be used as a bound on the storage parameter of
//! generic string and byte-buffer types.
//!
//! Implementations are provided for [`[u8; N]`](prim@array), [`[u8]`](prim@slice),
//! [`Vec<u8>`], and [`Cow<'_, [u8]>`](Cow); [`str`](prim@str), [`String`], and
//! [`Cow<'_, str>`](Cow); and [`&T`](prim@reference), [`&mut T`](prim@reference),
//! [`Box<T>`], [`Rc<T>`], and [`Arc<T>`] where `T: StringBytes + ?Sized`.
//! The optional `bytes` feature adds [`Bytes`] and [`BytesMut`].
//!
//! Implementations expose a consistent byte sequence and preserve it when
//! converting to [`Vec<u8>`], [`str`](prim@str), or [`String`]. The bytes need not
//! be valid UTF-8; validation is performed when needed to borrow them as
//! [`&str`](prim@str) or convert them into a [`String`].
//!
//! # Examples
//!
//! ## `&[u8]`
//!
//! ```rust
//! use stringbytes::StringBytes;
//!
//! let bytes: &[u8] = b"hello";
//!
//! // Validate UTF-8 and borrow a string slice without copying.
//! assert_eq!(bytes.try_borrow_str()?, "hello");
//! # Ok::<(), std::str::Utf8Error>(())
//! ```
//!
//! ## `Vec<u8>`
//!
//! ```rust
//! use stringbytes::StringBytes;
//!
//! let bytes = b"hello".to_vec();
//! let address = bytes.as_ptr();
//!
//! // Validate UTF-8 and transfer the vector's allocation to the string.
//! let string = bytes.try_into_string()?;
//! assert_eq!(string, "hello");
//! assert_eq!(string.as_ptr(), address);
//!
//! // Convert back to bytes, keeping the same allocation.
//! let bytes = string.into_bytes();
//! assert_eq!(bytes, b"hello");
//! assert_eq!(bytes.as_ptr(), address);
//! # Ok::<(), std::string::FromUtf8Error>(())
//! ```
//!
//! ## `Box<[u8]>`
//!
//! ```rust
//! use stringbytes::StringBytes;
//!
//! let bytes = Box::<[u8]>::from("hello".as_bytes());
//!
//! // Borrow the bytes as a string slice without copying.
//! assert_eq!(bytes.try_borrow_str()?, "hello");
//!
//! // Copy the bytes, keeping the original value.
//! assert_eq!(bytes.try_to_string()?, "hello");
//!
//! // Consume the box and reuse its allocation.
//! assert_eq!(bytes.try_into_string()?, "hello");
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## `&str`
//!
//! ```rust
//! use stringbytes::StringBytes;
//!
//! let text: &str = "hello";
//!
//! // Copy the borrowed string without checking UTF-8 again.
//! let string = text.try_to_string()?;
//! assert_eq!(string, text);
//! # Ok::<(), std::string::FromUtf8Error>(())
//! ```
//!
//! ## `String`
//!
//! ```rust
//! use stringbytes::StringBytes;
//!
//! let string = String::from("hello");
//! let address = string.as_ptr();
//! let capacity = string.capacity();
//!
//! // Copy the bytes, keeping the string available.
//! let bytes = string.to_bytes();
//! assert_eq!(bytes, b"hello");
//!
//! // Consume the string, keeping its allocation and capacity.
//! let string = string.try_into_string()?;
//! assert_eq!(string, "hello");
//! assert_eq!(string.as_ptr(), address);
//! assert_eq!(string.capacity(), capacity);
//! # Ok::<(), std::string::FromUtf8Error>(())
//! ```

#[cfg(doc)]
use std::borrow::Cow;
use std::rc::Rc;
use std::str::{self, Utf8Error};
use std::string::FromUtf8Error;
use std::sync::Arc;

#[cfg(all(doc, feature = "bytes"))]
use bytes::{Bytes, BytesMut};

mod impls;
mod sealed;

/// A common interface for string and byte-buffer storage.
///
/// Types implementing [`StringBytes`] can be used as the storage parameter of
/// generic string and byte-buffer types. Implementations expose a consistent
/// byte sequence and preserve it when converting to [`Vec<u8>`], [`str`](prim@str),
/// or [`String`].
///
/// The bytes need not be valid UTF-8. [`as_bytes`](Self::as_bytes) borrows them
/// directly, [`to_bytes`](Self::to_bytes) copies them, and
/// [`into_bytes`](Self::into_bytes) consumes the value and reuses its allocation
/// where possible.
///
/// [`try_borrow_str`](Self::try_borrow_str) borrows the bytes as a string slice,
/// checking UTF-8 when needed. To obtain an owned [`String`], use
/// [`try_to_string`](Self::try_to_string) to copy the bytes or
/// [`try_into_string`](Self::try_into_string) to consume the value and reuse its
/// allocation where possible.
///
/// This trait is sealed and cannot be implemented outside this crate.
///
/// # Guarantees
///
/// All implementations obey the following rules:
///
/// - [`Self::as_bytes`] returns the same bytes unless the value is modified
///   through mutable or owned access. Interior mutability must not change the
///   bytes exposed by this method, [`AsRef<[u8]>`](AsRef), or [`AsMut<[u8]>`](AsMut).
/// - If [`AsRef<[u8]>`](AsRef) is implemented, it borrows the same slice as
///   [`as_bytes`](Self::as_bytes).
/// - If [`AsMut<[u8]>`](AsMut) is implemented, it mutably borrows that same slice.
/// - [`Self::to_bytes`] and [`Self::into_bytes`] preserve the bytes returned by
///   [`as_bytes`](Self::as_bytes).
/// - Successful calls to [`Self::try_borrow_str`], [`Self::try_to_string`], and
///   [`Self::try_into_string`] preserve the bytes returned by [`as_bytes`](Self::as_bytes).
///
/// The rules for [`AsRef`] and [`AsMut`] apply only when those traits are
/// implemented; they are not required by [`StringBytes`].
///
/// # Dyn compatibility
///
/// This trait is [dyn compatible].
///
/// In older versions of Rust, dyn compatibility was called "object safety".
///
/// [dyn compatible]: https://doc.rust-lang.org/reference/items/traits.html#dyn-compatibility
pub trait StringBytes: sealed::Sealed {
    /// Returns a borrowed view of the underlying bytes.
    ///
    /// This method does not allocate, copy, or validate UTF-8. The returned
    /// slice may contain any byte sequence.
    ///
    /// # Examples
    ///
    /// ```
    /// use stringbytes::StringBytes;
    ///
    /// let bytes = [b'a', 0xff];
    /// assert_eq!(bytes.as_bytes(), b"a\xff");
    /// ```
    fn as_bytes(&self) -> &[u8];

    /// Copies the bytes into a [`Vec<u8>`], leaving `self` unchanged.
    ///
    /// This method does not validate UTF-8. The returned vector owns a copy of
    /// the bytes, even when `self` owns its storage.
    ///
    /// # Examples
    ///
    /// ```
    /// use stringbytes::StringBytes;
    ///
    /// let text = String::from("hello");
    /// let bytes = text.to_bytes();
    /// assert_eq!(bytes, b"hello");
    /// assert_eq!(text, "hello");
    /// ```
    #[inline]
    fn to_bytes(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }

    /// Converts `self` into a [`Vec<u8>`], consuming the value.
    ///
    /// Reuses the existing allocation where possible and copies the bytes
    /// otherwise. This method does not validate UTF-8.
    ///
    /// # Allocation reuse
    ///
    /// - [`&T`](prim@reference), [`&mut T`](prim@reference), and
    ///   [`[u8; N]`](prim@array) are copied.
    /// - [`Vec<u8>`] and [`String`] keep their buffer and capacity.
    /// - [`Cow`] reuses owned storage and copies borrowed storage.
    /// - [`Box<[u8]>`](Box), [`Box<[u8; N]>`](Box), and [`Box<str>`] reuse their allocation.
    ///   For other sized `T`, [`Box<T>`] moves out the value and uses `T`'s conversion.
    /// - [`Rc<T>`] and [`Arc<T>`], where `T` is [`Sized`], take ownership of
    ///   the inner value when only one strong reference remains, then use `T`'s
    ///   conversion. Otherwise, they copy the bytes. [`rc::Weak`](std::rc::Weak)
    ///   and [`sync::Weak`](std::sync::Weak) pointers do not prevent this.
    ///   [`Rc<[u8]>`](Rc), [`Arc<[u8]>`](Arc), [`Rc<str>`], and [`Arc<str>`] always copy.
    /// - With the `bytes` feature, [`Bytes`] and [`BytesMut`] use their conversion to
    ///   [`Vec<u8>`]. Static data, shared buffers, and [`Bytes::from_owner`] storage
    ///   are copied. Reusing a sliced buffer may move bytes within its allocation.
    ///
    /// Nested wrappers and trait objects follow the same conversion rules.
    ///
    /// # Examples
    ///
    /// ```
    /// use stringbytes::StringBytes;
    ///
    /// let bytes = vec![b'a', 0xff];
    /// let address = bytes.as_ptr();
    /// let capacity = bytes.capacity();
    /// let bytes = bytes.into_bytes();
    /// assert_eq!(bytes, b"a\xff");
    /// assert_eq!(bytes.as_ptr(), address);
    /// assert_eq!(bytes.capacity(), capacity);
    /// ```
    #[inline]
    fn into_bytes(self) -> Vec<u8>
    where
        Self: Sized,
    {
        self.to_bytes()
    }

    /// Returns a string slice if the bytes are valid UTF-8.
    ///
    /// The slice borrows directly from `self`; no allocation or copying is
    /// needed. Byte sequences are validated on each call. Types that already
    /// guarantee UTF-8 skip validation and always return [`Ok`].
    ///
    /// # Errors
    ///
    /// Returns [`Utf8Error`] if the bytes are not valid UTF-8.
    ///
    /// # Examples
    ///
    /// ```
    /// use stringbytes::StringBytes;
    ///
    /// let bytes = b"hello";
    /// assert_eq!(bytes.try_borrow_str()?, "hello");
    /// # Ok::<(), std::str::Utf8Error>(())
    /// ```
    #[inline]
    fn try_borrow_str(&self) -> Result<&str, Utf8Error> {
        str::from_utf8(self.as_bytes())
    }

    /// Copies the bytes into a [`String`], leaving `self` unchanged.
    ///
    /// Byte sequences are validated once per call. Types that already
    /// guarantee UTF-8 are copied without validation and always return [`Ok`].
    ///
    /// # Errors
    ///
    /// Returns [`FromUtf8Error`] containing the copied bytes if they are not
    /// valid UTF-8.
    ///
    /// # Examples
    ///
    /// ```
    /// use stringbytes::StringBytes;
    ///
    /// let bytes = vec![b'h', b'i'];
    /// let text = bytes.try_to_string()?;
    /// assert_eq!(text, "hi");
    /// assert_eq!(bytes, b"hi");
    /// # Ok::<(), std::string::FromUtf8Error>(())
    /// ```
    #[inline]
    fn try_to_string(&self) -> Result<String, FromUtf8Error> {
        // Copy through to_bytes, then always validate UTF-8. Types that already
        // guarantee UTF-8 override this to avoid validating those bytes again.
        String::from_utf8(self.to_bytes())
    }

    /// Converts `self` into a [`String`], consuming the value.
    ///
    /// Reuses the existing allocation where possible and copies the bytes
    /// otherwise.
    ///
    /// Byte sequences are validated once. Types that already guarantee UTF-8
    /// skip validation and always return [`Ok`].
    ///
    /// # Allocation reuse
    ///
    /// Follows the same allocation rules as [`into_bytes`](Self::into_bytes).
    ///
    /// # Errors
    ///
    /// Returns [`FromUtf8Error`] for invalid UTF-8. The error keeps the buffer
    /// used for conversion, following the same allocation rules as a successful
    /// result. Use [`into_bytes`](FromUtf8Error::into_bytes) to recover the bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use stringbytes::StringBytes;
    ///
    /// let bytes = Box::<[u8]>::from("hello".as_bytes());
    /// assert_eq!(bytes.try_into_string()?, "hello");
    ///
    /// let invalid = vec![0xff];
    /// let error = invalid.try_into_string().unwrap_err();
    /// assert_eq!(error.utf8_error().valid_up_to(), 0);
    /// assert_eq!(error.into_bytes(), vec![0xff]);
    /// # Ok::<(), std::string::FromUtf8Error>(())
    /// ```
    #[inline]
    fn try_into_string(self) -> Result<String, FromUtf8Error>
    where
        Self: Sized,
    {
        // Use the allocation rules of into_bytes, then always validate UTF-8.
        // Types that already guarantee UTF-8 override their string conversions
        // to avoid validating those bytes again.
        String::from_utf8(self.into_bytes())
    }

    // Owned pointer receivers preserve the concrete type's conversion through
    // dyn StringBytes.

    /// Converts the contents of a [`Box`] into a [`Vec<u8>`].
    ///
    /// Boxed byte slices, byte arrays, and strings reuse their allocation.
    /// Other boxed values are moved out and converted with [`Self::into_bytes`].
    /// This also supports [`Box<dyn StringBytes>`](Box).
    #[doc(hidden)]
    #[inline]
    fn box_into_bytes(self: Box<Self>) -> Vec<u8> {
        // Unsized contents need a copying fallback unless they can transfer
        // the boxed allocation directly.
        (*self).to_bytes()
    }

    /// Converts the contents of an [`Rc`] into a [`Vec<u8>`].
    ///
    /// With one strong owner, sized contents use [`Self::into_bytes`].
    /// Shared contents, slices, and [`str`](prim@str) are copied.
    /// Trait objects use their concrete type's conversion.
    #[doc(hidden)]
    #[inline]
    fn rc_into_bytes(self: Rc<Self>) -> Vec<u8> {
        (*self).to_bytes()
    }

    /// Converts the contents of an [`Arc`] into a [`Vec<u8>`].
    ///
    /// With one strong owner, sized contents use [`Self::into_bytes`].
    /// Shared contents, slices, and [`str`](prim@str) are copied.
    /// Trait objects use their concrete type's conversion.
    #[doc(hidden)]
    #[inline]
    fn arc_into_bytes(self: Arc<Self>) -> Vec<u8> {
        (*self).to_bytes()
    }

    // The pointer defaults below follow the corresponding byte conversion's
    // allocation rules and always validate UTF-8. Known UTF-8 types override
    // these string conversions to avoid revalidation.

    /// Converts the contents of a [`Box`] into a [`String`].
    ///
    /// Boxed byte slices, byte arrays, and strings reuse their allocation.
    /// Other boxed values are moved out and converted with [`Self::try_into_string`].
    /// This also supports [`Box<dyn StringBytes>`](Box).
    ///
    /// # Errors
    ///
    /// Returns [`FromUtf8Error`] if the stored bytes are not valid UTF-8.
    #[doc(hidden)]
    #[inline]
    fn box_into_string(self: Box<Self>) -> Result<String, FromUtf8Error> {
        String::from_utf8(self.box_into_bytes())
    }

    /// Converts the contents of an [`Rc`] into a [`String`].
    ///
    /// With one strong owner, sized contents are moved out and converted with
    /// [`Self::try_into_string`]. Shared contents, slices, and [`str`](prim@str) are copied.
    /// Trait objects use their concrete type's conversion.
    ///
    /// # Errors
    ///
    /// Returns [`FromUtf8Error`] if the stored bytes are not valid UTF-8.
    #[doc(hidden)]
    #[inline]
    fn rc_into_string(self: Rc<Self>) -> Result<String, FromUtf8Error> {
        String::from_utf8(self.rc_into_bytes())
    }

    /// Converts the contents of an [`Arc`] into a [`String`].
    ///
    /// With one strong owner, sized contents are moved out and converted with
    /// [`Self::try_into_string`]. Shared contents, slices, and [`str`](prim@str) are copied.
    /// Trait objects use their concrete type's conversion.
    ///
    /// # Errors
    ///
    /// Returns [`FromUtf8Error`] if the stored bytes are not valid UTF-8.
    #[doc(hidden)]
    #[inline]
    fn arc_into_string(self: Arc<Self>) -> Result<String, FromUtf8Error> {
        String::from_utf8(self.arc_into_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::StringBytes;

    struct ByteView(&'static [u8]);

    impl super::sealed::Sealed for ByteView {}

    impl StringBytes for ByteView {
        fn as_bytes(&self) -> &[u8] {
            self.0
        }

        fn try_borrow_str(&self) -> Result<&str, std::str::Utf8Error> {
            // Owned conversions should validate their buffer, without a prior scan.
            panic!("owned string conversion must not call try_borrow_str")
        }
    }

    #[test]
    fn try_to_string_validates_without_preborrowing() {
        let source = ByteView("étext".as_bytes());
        assert_eq!(source.try_to_string().unwrap(), "étext");
        assert_eq!(source.as_bytes(), "étext".as_bytes());

        let source = ByteView(b"ok\xff");
        let error = source.try_to_string().unwrap_err();
        assert_eq!(error.utf8_error().valid_up_to(), 2);
        assert_eq!(error.as_bytes(), b"ok\xff");
        assert_eq!(source.as_bytes(), b"ok\xff");
    }

    #[test]
    fn try_into_string_validates_without_preborrowing() {
        assert_eq!(
            ByteView("étext".as_bytes()).try_into_string().unwrap(),
            "étext"
        );
        let error = ByteView(b"ok\xff").try_into_string().unwrap_err();
        assert_eq!(error.utf8_error().valid_up_to(), 2);
        assert_eq!(error.into_bytes(), b"ok\xff");
    }
}
