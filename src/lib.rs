//! `stringbytes` provides a common interface for string and byte-buffer storage.
//!
//! The [`StringBytes`] trait can be used as a bound on the storage parameter of
//! generic string and byte-buffer types.
//!
//! Both borrowed and owned storage are supported, including shared buffers.
//!
//! Implementations expose a consistent byte sequence. Byte conversions and
//! successful checked string conversions preserve that sequence.
//! The bytes need not be valid UTF-8; validation is performed when needed to
//! borrow them as [`&str`](prim@str) or convert them into a [`String`].
//! Unchecked conversions let callers uphold UTF-8 validity themselves.
//! Lossy conversions follow the same behavior as [`String::from_utf8_lossy`].
//!
//! # Examples
//!
//! ## `&[u8]`
//!
//! ```rust
//! use std::borrow::Cow;
//! use stringbytes::StringBytes;
//!
//! let bytes: &[u8] = b"hello";
//! assert_eq!(bytes.as_bytes(), b"hello");
//!
//! // Validate UTF-8 and borrow a string slice without copying.
//! assert_eq!(bytes.try_borrow_str()?, "hello");
//!
//! // SAFETY: the bytes come from a UTF-8 string literal.
//! let text = unsafe { bytes.borrow_str_unchecked() };
//! assert_eq!(text, "hello");
//!
//! // Lossy conversion borrows valid UTF-8 and replaces invalid UTF-8 as needed.
//! assert!(matches!(bytes.to_string_lossy(), Cow::Borrowed("hello")));
//! let invalid: &[u8] = b"he\xfflo";
//! assert_eq!(invalid.to_string_lossy(), "he�lo");
//!
//! // SAFETY: bytes still contains the UTF-8 literal above; copy without validation.
//! let string = unsafe { bytes.to_string_unchecked() };
//! assert_eq!(string, "hello");
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
//! let bytes = StringBytes::into_bytes(string);
//! assert_eq!(bytes, b"hello");
//! assert_eq!(bytes.as_ptr(), address);
//!
//! // SAFETY: these bytes came from a String and have not been modified.
//! let string = unsafe { bytes.into_string_unchecked() };
//! assert_eq!(string, "hello");
//! assert_eq!(string.as_ptr(), address);
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
//!
//! ## `&CStr`
//!
//! ```rust
//! use stringbytes::StringBytes;
//!
//! let text = c"hello";
//!
//! // C strings expose their contents without the trailing NUL.
//! assert_eq!(text.try_borrow_str()?, "hello");
//!
//! // CStr::to_bytes borrows a slice; the trait method copies into a vector.
//! let bytes: Vec<u8> = StringBytes::to_bytes(text);
//! assert_eq!(bytes, b"hello");
//! # Ok::<(), std::str::Utf8Error>(())
//! ```
//!
//! ## `CString`
//!
//! ```rust
//! use std::ffi::CString;
//! use stringbytes::StringBytes;
//!
//! let text = CString::from(c"hello");
//! let address = text.as_bytes().as_ptr();
//!
//! // Validate UTF-8 and transfer the buffer, excluding the trailing NUL.
//! let string = text.try_into_string()?;
//! assert_eq!(string, "hello");
//! assert_eq!(string.as_ptr(), address);
//!
//! // C strings may also contain invalid UTF-8.
//! let text = CString::from(c"he\xfflo");
//! assert_eq!(text.into_string_lossy(), "he�lo");
//! # Ok::<(), std::string::FromUtf8Error>(())
//! ```

use std::borrow::Cow;
use std::rc::Rc;
use std::str::{self, Utf8Error};
use std::string::FromUtf8Error;
use std::sync::Arc;

mod impls;
mod sealed;

/// A common interface for string and byte-buffer storage.
///
/// Types implementing [`StringBytes`] can be used as the storage parameter of
/// generic string and byte-buffer types. Implementations expose a consistent
/// byte sequence. Byte conversions and successful checked string conversions
/// preserve that sequence.
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
/// The unchecked methods skip UTF-8 validation and require the caller to
/// guarantee validity. [`to_string_lossy`](Self::to_string_lossy) and
/// [`into_string_lossy`](Self::into_string_lossy) follow the behavior of
/// [`String::from_utf8_lossy`].
///
/// This trait is sealed and cannot be implemented outside this crate.
///
/// # Guarantees
///
/// All implementations obey the following rules:
///
/// - [`Self::as_bytes`] returns the same bytes unless the value is modified
///   through mutable or owned access. Interior mutability does not change
///   the bytes exposed through this method, [`AsRef<[u8]>`](AsRef),
///   [`AsMut<[u8]>`](AsMut), or [`Deref`](std::ops::Deref).
/// - If [`AsRef<[u8]>`](AsRef) is implemented, it borrows the same slice as
///   [`as_bytes`](Self::as_bytes).
/// - If [`AsMut<[u8]>`](AsMut) is implemented, it mutably borrows that same slice.
/// - If [`Deref`](std::ops::Deref) is implemented, its
///   [`Target`](std::ops::Deref::Target) also implements [`StringBytes`].
///   Calling [`as_bytes`](Self::as_bytes) on `self` or its dereference target
///   borrows the same slice.
/// - [`Self::to_bytes`] and [`Self::into_bytes`] preserve the bytes returned by
///   [`as_bytes`](Self::as_bytes).
/// - Successful calls to [`Self::try_borrow_str`], [`Self::try_to_string`], and
///   [`Self::try_into_string`] preserve the bytes returned by [`as_bytes`](Self::as_bytes).
/// - [`Self::borrow_str_unchecked`], [`Self::to_string_unchecked`], and
///   [`Self::into_string_unchecked`] preserve those bytes when their safety
///   requirements are met.
///
/// The rules for [`AsRef`], [`AsMut`], and [`Deref`](std::ops::Deref) apply only
/// when those traits are implemented; they are not required by [`StringBytes`].
///
/// # Dyn compatibility
///
/// This trait is not [dyn compatible].
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
    /// needed.
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

    /// Borrows the bytes as a string slice without checking UTF-8.
    ///
    /// This method does not allocate or copy.
    ///
    /// # Safety
    ///
    /// The bytes returned by [`as_bytes`](Self::as_bytes) must be valid UTF-8.
    ///
    /// # Examples
    ///
    /// ```
    /// use stringbytes::StringBytes;
    ///
    /// let bytes = b"hello";
    /// // SAFETY: the bytes come from a UTF-8 string literal.
    /// assert_eq!(unsafe { bytes.borrow_str_unchecked() }, "hello");
    /// ```
    #[inline]
    unsafe fn borrow_str_unchecked(&self) -> &str {
        // SAFETY: the caller guarantees that as_bytes returns valid UTF-8.
        unsafe { str::from_utf8_unchecked(self.as_bytes()) }
    }

    /// Copies the bytes into a [`String`], leaving `self` unchanged.
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
        // Validate the copied buffer so an error can return it to the caller.
        String::from_utf8(self.to_bytes())
    }

    /// Returns a string, replacing invalid UTF-8 sequences with `�` (U+FFFD).
    ///
    /// Borrows valid UTF-8 without allocating. Otherwise, returns an owned
    /// string with the same replacements as [`String::from_utf8_lossy`].
    /// The original bytes remain unchanged.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::borrow::Cow;
    /// use stringbytes::StringBytes;
    ///
    /// assert!(matches!(b"hello".to_string_lossy(), Cow::Borrowed("hello")));
    /// assert_eq!(b"a\xffb".to_string_lossy(), "a�b");
    /// ```
    #[inline]
    fn to_string_lossy(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(self.as_bytes())
    }

    /// Copies the bytes into a [`String`] without checking UTF-8.
    ///
    /// Leaves `self` unchanged, following the copying rules of
    /// [`to_bytes`](Self::to_bytes).
    ///
    /// # Safety
    ///
    /// The bytes returned by [`as_bytes`](Self::as_bytes) must be valid UTF-8.
    ///
    /// # Examples
    ///
    /// ```
    /// use stringbytes::StringBytes;
    ///
    /// let bytes = b"hello";
    /// // SAFETY: the bytes come from a UTF-8 string literal.
    /// let text = unsafe { bytes.to_string_unchecked() };
    /// assert_eq!(text, "hello");
    /// ```
    #[inline]
    unsafe fn to_string_unchecked(&self) -> String {
        // SAFETY: to_bytes preserves the bytes the caller guarantees are UTF-8.
        unsafe { String::from_utf8_unchecked(self.to_bytes()) }
    }

    /// Converts `self` into a [`String`], consuming the value.
    ///
    /// Reuses the existing allocation where possible and copies the bytes
    /// otherwise, as with [`into_bytes`](Self::into_bytes).
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
        // Validate after conversion so an error retains the converted buffer.
        String::from_utf8(self.into_bytes())
    }

    /// Converts `self` into a [`String`], replacing invalid UTF-8 with `�` (U+FFFD).
    ///
    /// Valid UTF-8 follows the allocation rules of
    /// [`into_bytes`](Self::into_bytes). Invalid sequences are replaced
    /// as in [`String::from_utf8_lossy`], without guaranteeing reuse of the
    /// original allocation.
    ///
    /// # Examples
    ///
    /// ```
    /// use stringbytes::StringBytes;
    ///
    /// let bytes = b"a\xffb".to_vec();
    /// let text = bytes.into_string_lossy();
    /// assert_eq!(text, "a�b");
    /// ```
    #[inline]
    fn into_string_lossy(self) -> String
    where
        Self: Sized,
    {
        // Convert before taking ownership of the bytes so invalid input does
        // not need an intermediate copy.
        match self.to_string_lossy() {
            Cow::Owned(string) => string,
            Cow::Borrowed(_) => {
                // SAFETY: to_string_lossy borrows only when the original bytes
                // are valid UTF-8, and into_bytes preserves those bytes.
                unsafe { String::from_utf8_unchecked(self.into_bytes()) }
            }
        }
    }

    /// Converts `self` into a [`String`] without checking UTF-8.
    ///
    /// Follows the allocation reuse rules of [`into_bytes`](Self::into_bytes).
    ///
    /// # Safety
    ///
    /// The bytes returned by [`as_bytes`](Self::as_bytes) must be valid UTF-8.
    ///
    /// # Examples
    ///
    /// ```
    /// use stringbytes::StringBytes;
    ///
    /// let bytes = b"hello".to_vec();
    /// let address = bytes.as_ptr();
    /// // SAFETY: the bytes come from a UTF-8 string literal.
    /// let text = unsafe { bytes.into_string_unchecked() };
    /// assert_eq!(text, "hello");
    /// assert_eq!(text.as_ptr(), address);
    /// ```
    #[inline]
    unsafe fn into_string_unchecked(self) -> String
    where
        Self: Sized,
    {
        // SAFETY: into_bytes preserves the bytes the caller guarantees are UTF-8.
        unsafe { String::from_utf8_unchecked(self.into_bytes()) }
    }

    // ------------------------------------------------------------------------
    // Owned pointer receivers let each storage type choose whether to move
    // or copy its bytes.
    // ------------------------------------------------------------------------

    /// Converts the contents of a [`Box`] into a [`Vec<u8>`].
    ///
    /// Reuses the allocation where possible and copies the bytes otherwise.
    #[doc(hidden)]
    #[inline]
    fn box_into_bytes(self: Box<Self>) -> Vec<u8> {
        // Copy in place unless the contents can transfer an allocation.
        (*self).to_bytes()
    }

    /// Converts the contents of an [`Rc`] into a [`Vec<u8>`].
    ///
    /// Reuses the allocation where possible and copies the bytes otherwise.
    #[doc(hidden)]
    #[inline]
    fn rc_into_bytes(self: Rc<Self>) -> Vec<u8> {
        (*self).to_bytes()
    }

    /// Converts the contents of an [`Arc`] into a [`Vec<u8>`].
    ///
    /// Reuses the allocation where possible and copies the bytes otherwise.
    #[doc(hidden)]
    #[inline]
    fn arc_into_bytes(self: Arc<Self>) -> Vec<u8> {
        (*self).to_bytes()
    }

    /// Converts the contents of a [`Box`] into a [`String`].
    ///
    /// Allocation reuse follows [`Self::box_into_bytes`].
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
    /// Allocation reuse follows [`Self::rc_into_bytes`].
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
    /// Allocation reuse follows [`Self::arc_into_bytes`].
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
    use std::borrow::Cow;
    use std::rc::Rc;
    use std::sync::Arc;

    use super::StringBytes;

    struct ByteView<'a>(&'a [u8]);

    impl super::sealed::Sealed for ByteView<'_> {}

    impl StringBytes for ByteView<'_> {
        fn as_bytes(&self) -> &[u8] {
            self.0
        }
    }

    struct NoCheckedConversions(&'static [u8]);

    impl super::sealed::Sealed for NoCheckedConversions {}

    impl StringBytes for NoCheckedConversions {
        fn as_bytes(&self) -> &[u8] {
            self.0
        }

        fn try_borrow_str(&self) -> Result<&str, std::str::Utf8Error> {
            panic!("unchecked borrowing must not validate UTF-8")
        }

        fn try_to_string(&self) -> Result<String, std::string::FromUtf8Error> {
            panic!("unchecked copying must not validate UTF-8")
        }

        fn try_into_string(self) -> Result<String, std::string::FromUtf8Error> {
            panic!("unchecked consumption must not validate UTF-8")
        }
    }

    struct NoPrevalidation(&'static [u8]);

    impl super::sealed::Sealed for NoPrevalidation {}

    impl StringBytes for NoPrevalidation {
        fn as_bytes(&self) -> &[u8] {
            self.0
        }

        fn try_borrow_str(&self) -> Result<&str, std::str::Utf8Error> {
            panic!("owned conversion must validate its own buffer")
        }
    }

    struct NoIntermediateCopy(&'static [u8]);

    impl super::sealed::Sealed for NoIntermediateCopy {}

    impl StringBytes for NoIntermediateCopy {
        fn as_bytes(&self) -> &[u8] {
            self.0
        }

        fn to_bytes(&self) -> Vec<u8> {
            panic!("invalid input must not be copied to an intermediate Vec")
        }

        fn into_bytes(self) -> Vec<u8> {
            panic!("invalid input must not be converted to an intermediate Vec")
        }
    }

    #[test]
    fn to_bytes_default_returns_an_independent_copy() {
        for input in [
            b"".as_slice(),
            b"text",
            "é说明".as_bytes(),
            b"\0e\xcc\x81",
            b"ok\xff",
            b"ok\xe2\x82",
            b"\xc0\x80",
            b"\xed\xa0\x80",
        ] {
            let storage = ByteView(input);
            let mut bytes = StringBytes::to_bytes(&storage);
            assert_eq!(bytes, input);
            if !bytes.is_empty() {
                assert_ne!(bytes.as_ptr(), StringBytes::as_bytes(&storage).as_ptr());
                bytes[0] ^= 0xff;
            }
            assert_eq!(StringBytes::as_bytes(&storage), input);
        }
    }

    #[test]
    fn into_bytes_default_copies_storage() {
        for input in [
            b"".as_slice(),
            b"text",
            "é说明".as_bytes(),
            b"\0e\xcc\x81",
            b"ok\xff",
            b"ok\xe2\x82",
            b"\xc0\x80",
            b"\xed\xa0\x80",
        ] {
            let storage = ByteView(input);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let bytes = StringBytes::into_bytes(storage);
            assert_eq!(bytes, input);
            if !input.is_empty() {
                assert_ne!(bytes.as_ptr(), address);
            }
        }
    }

    #[test]
    fn try_borrow_str_default_borrows_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let storage = ByteView(input);
            let value = StringBytes::try_borrow_str(&storage).unwrap();
            assert_eq!(value, expected);
            assert!(std::ptr::eq(
                value.as_bytes(),
                StringBytes::as_bytes(&storage)
            ));
        }
    }

    #[test]
    fn try_borrow_str_default_reports_invalid_utf8() {
        for (input, valid_up_to, error_len) in [
            (b"ok\xff".as_slice(), 2, Some(1)),
            (b"ok\xe2\x82", 2, None),
            (b"\xc0\x80", 0, Some(1)),
            (b"\xed\xa0\x80", 0, Some(1)),
        ] {
            let storage = ByteView(input);
            let error = StringBytes::try_borrow_str(&storage).unwrap_err();
            assert_eq!(error.valid_up_to(), valid_up_to);
            assert_eq!(error.error_len(), error_len);
            assert_eq!(StringBytes::as_bytes(&storage), input);
        }
    }

    #[test]
    fn borrow_str_unchecked_default_borrows_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let storage = ByteView(input);
            // SAFETY: these test literals are valid UTF-8.
            let value = unsafe { StringBytes::borrow_str_unchecked(&storage) };
            assert_eq!(value, expected);
            assert!(std::ptr::eq(
                value.as_bytes(),
                StringBytes::as_bytes(&storage)
            ));
        }
    }

    #[test]
    fn borrow_str_unchecked_does_not_call_checked_conversions() {
        let storage = NoCheckedConversions("étext".as_bytes());
        // SAFETY: the fixture exposes the valid UTF-8 literal above.
        let value = unsafe { storage.borrow_str_unchecked() };
        assert_eq!(value, "étext");
    }

    #[test]
    fn try_to_string_default_copies_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let storage = ByteView(input);
            let mut value = StringBytes::try_to_string(&storage).unwrap();
            assert_eq!(value, expected);
            if !value.is_empty() {
                assert_ne!(value.as_ptr(), StringBytes::as_bytes(&storage).as_ptr());
            }
            value.make_ascii_uppercase();
            assert_eq!(StringBytes::as_bytes(&storage), input);
        }
    }

    #[test]
    fn try_to_string_default_returns_a_copy_of_invalid_bytes() {
        for (input, valid_up_to, error_len) in [
            (b"ok\xff".as_slice(), 2, Some(1)),
            (b"ok\xe2\x82", 2, None),
            (b"\xc0\x80", 0, Some(1)),
            (b"\xed\xa0\x80", 0, Some(1)),
        ] {
            let storage = ByteView(input);
            let error = StringBytes::try_to_string(&storage).unwrap_err();
            assert_eq!(error.utf8_error().valid_up_to(), valid_up_to);
            assert_eq!(error.utf8_error().error_len(), error_len);
            assert_eq!(error.as_bytes(), input);
            assert_ne!(
                error.as_bytes().as_ptr(),
                StringBytes::as_bytes(&storage).as_ptr()
            );
            assert_eq!(StringBytes::as_bytes(&storage), input);
        }
    }

    #[test]
    fn try_to_string_validates_without_preborrowing() {
        let storage = NoPrevalidation("étext".as_bytes());
        assert_eq!(storage.try_to_string().unwrap(), "étext");
    }

    #[test]
    fn try_to_string_reports_invalid_utf8_without_preborrowing() {
        let storage = NoPrevalidation(b"ok\xff");
        let error = storage.try_to_string().unwrap_err();
        assert_eq!(error.utf8_error().valid_up_to(), 2);
        assert_eq!(error.utf8_error().error_len(), Some(1));
        assert_eq!(error.into_bytes(), b"ok\xff");
    }

    #[test]
    fn to_string_lossy_default_borrows_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let storage = ByteView(input);
            let value = StringBytes::to_string_lossy(&storage);
            assert!(matches!(value, Cow::Borrowed(_)));
            assert_eq!(value, expected);
            // Empty buffers may borrow the standard library's static empty string.
            if !value.is_empty() {
                assert!(std::ptr::eq(
                    value.as_bytes(),
                    StringBytes::as_bytes(&storage)
                ));
            }
        }
    }

    #[test]
    fn to_string_lossy_default_owns_replacement_text() {
        for (input, expected) in [
            (b"a\xffb".as_slice(), "a�b"),
            (b"ok\xe2\x82", "ok�"),
            (b"\xc0\x80", "��"),
            (b"\xed\xa0\x80", "���"),
        ] {
            let storage = ByteView(input);
            let value = StringBytes::to_string_lossy(&storage);
            assert!(matches!(value, Cow::Owned(_)));
            assert_eq!(value, expected);
            assert_eq!(StringBytes::as_bytes(&storage), input);
        }
    }

    #[test]
    fn to_string_unchecked_default_copies_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let storage = ByteView(input);
            // SAFETY: these test literals are valid UTF-8.
            let mut value = unsafe { StringBytes::to_string_unchecked(&storage) };
            assert_eq!(value, expected);
            if !value.is_empty() {
                assert_ne!(value.as_ptr(), StringBytes::as_bytes(&storage).as_ptr());
            }
            value.make_ascii_uppercase();
            assert_eq!(StringBytes::as_bytes(&storage), input);
        }
    }

    #[test]
    fn to_string_unchecked_does_not_call_checked_conversions() {
        let storage = NoCheckedConversions("étext".as_bytes());
        // SAFETY: the fixture exposes the valid UTF-8 literal above.
        let value = unsafe { storage.to_string_unchecked() };
        assert_eq!(value, "étext");
    }

    #[test]
    fn try_into_string_default_copies_storage() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let storage = ByteView(input);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let value = StringBytes::try_into_string(storage).unwrap();
            assert_eq!(value, expected);
            if !input.is_empty() {
                assert_ne!(value.as_ptr(), address);
            }
        }
    }

    #[test]
    fn try_into_string_default_error_copies_storage() {
        for (input, valid_up_to, error_len) in [
            (b"ok\xff".as_slice(), 2, Some(1)),
            (b"ok\xe2\x82", 2, None),
            (b"\xc0\x80", 0, Some(1)),
            (b"\xed\xa0\x80", 0, Some(1)),
        ] {
            let storage = ByteView(input);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let error = StringBytes::try_into_string(storage).unwrap_err();
            assert_eq!(error.utf8_error().valid_up_to(), valid_up_to);
            assert_eq!(error.utf8_error().error_len(), error_len);
            let bytes = error.into_bytes();
            assert_eq!(bytes, input);
            if !input.is_empty() {
                assert_ne!(bytes.as_ptr(), address);
            }
        }
    }

    #[test]
    fn try_into_string_validates_without_preborrowing() {
        let storage = NoPrevalidation("étext".as_bytes());
        assert_eq!(storage.try_into_string().unwrap(), "étext");
    }

    #[test]
    fn try_into_string_reports_invalid_utf8_without_preborrowing() {
        let storage = NoPrevalidation(b"ok\xff");
        let error = storage.try_into_string().unwrap_err();
        assert_eq!(error.utf8_error().valid_up_to(), 2);
        assert_eq!(error.utf8_error().error_len(), Some(1));
        assert_eq!(error.into_bytes(), b"ok\xff");
    }

    #[test]
    fn into_string_lossy_default_valid_utf8_copies_storage() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let storage = ByteView(input);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let value = StringBytes::into_string_lossy(storage);
            assert_eq!(value, expected);
            if !input.is_empty() {
                assert_ne!(value.as_ptr(), address);
            }
        }
    }

    #[test]
    fn into_string_lossy_default_replaces_invalid_utf8() {
        for (input, expected) in [
            (b"a\xffb".as_slice(), "a�b"),
            (b"ok\xe2\x82", "ok�"),
            (b"\xc0\x80", "��"),
            (b"\xed\xa0\x80", "���"),
        ] {
            let storage = ByteView(input);
            let value = StringBytes::into_string_lossy(storage);
            assert_eq!(value, expected);
        }
    }

    #[test]
    fn into_string_lossy_does_not_copy_invalid_bytes_before_conversion() {
        for (input, expected) in [
            (b"a\xffb".as_slice(), "a�b"),
            (b"ok\xe2\x82", "ok�"),
            (b"\xc0\x80", "��"),
            (b"\xed\xa0\x80", "���"),
        ] {
            let storage = NoIntermediateCopy(input);
            assert_eq!(storage.into_string_lossy(), expected);
        }
    }

    #[test]
    fn into_string_unchecked_default_copies_storage() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let storage = ByteView(input);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            // SAFETY: these test literals are valid UTF-8.
            let value = unsafe { StringBytes::into_string_unchecked(storage) };
            assert_eq!(value, expected);
            if !input.is_empty() {
                assert_ne!(value.as_ptr(), address);
            }
        }
    }

    #[test]
    fn into_string_unchecked_does_not_call_checked_conversions() {
        let storage = NoCheckedConversions("étext".as_bytes());
        // SAFETY: the fixture exposes the valid UTF-8 literal above.
        let value = unsafe { storage.into_string_unchecked() };
        assert_eq!(value, "étext");
    }

    #[test]
    fn box_into_bytes_default_copies_bytes() {
        let storage = Box::new(ByteView(b"text"));
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let value = StringBytes::box_into_bytes(storage);
        assert_eq!(value, b"text");
        assert_ne!(value.as_ptr(), address);
    }

    #[test]
    fn rc_into_bytes_default_copies_bytes() {
        let storage = Rc::new(ByteView(b"text"));
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let value = StringBytes::rc_into_bytes(storage);
        assert_eq!(value, b"text");
        assert_ne!(value.as_ptr(), address);
    }

    #[test]
    fn arc_into_bytes_default_copies_bytes() {
        let storage = Arc::new(ByteView(b"text"));
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let value = StringBytes::arc_into_bytes(storage);
        assert_eq!(value, b"text");
        assert_ne!(value.as_ptr(), address);
    }

    #[test]
    fn box_into_string_default_copies_bytes() {
        let storage = Box::new(ByteView(b"text"));
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let value = StringBytes::box_into_string(storage).unwrap();
        assert_eq!(value, "text");
        assert_ne!(value.as_ptr(), address);
    }

    #[test]
    fn box_into_string_default_error_copies_bytes() {
        let storage = Box::new(ByteView(b"ok\xff"));
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let error = StringBytes::box_into_string(storage).unwrap_err();
        assert_eq!(error.utf8_error().valid_up_to(), 2);
        assert_eq!(error.utf8_error().error_len(), Some(1));
        let value = error.into_bytes();
        assert_eq!(value, b"ok\xff");
        assert_ne!(value.as_ptr(), address);
    }

    #[test]
    fn rc_into_string_default_copies_bytes() {
        let storage = Rc::new(ByteView(b"text"));
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let value = StringBytes::rc_into_string(storage).unwrap();
        assert_eq!(value, "text");
        assert_ne!(value.as_ptr(), address);
    }

    #[test]
    fn rc_into_string_default_error_copies_bytes() {
        let storage = Rc::new(ByteView(b"ok\xff"));
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let error = StringBytes::rc_into_string(storage).unwrap_err();
        assert_eq!(error.utf8_error().valid_up_to(), 2);
        assert_eq!(error.utf8_error().error_len(), Some(1));
        let value = error.into_bytes();
        assert_eq!(value, b"ok\xff");
        assert_ne!(value.as_ptr(), address);
    }

    #[test]
    fn arc_into_string_default_copies_bytes() {
        let storage = Arc::new(ByteView(b"text"));
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let value = StringBytes::arc_into_string(storage).unwrap();
        assert_eq!(value, "text");
        assert_ne!(value.as_ptr(), address);
    }

    #[test]
    fn arc_into_string_default_error_copies_bytes() {
        let storage = Arc::new(ByteView(b"ok\xff"));
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let error = StringBytes::arc_into_string(storage).unwrap_err();
        assert_eq!(error.utf8_error().valid_up_to(), 2);
        assert_eq!(error.utf8_error().error_len(), Some(1));
        let value = error.into_bytes();
        assert_eq!(value, b"ok\xff");
        assert_ne!(value.as_ptr(), address);
    }
}
