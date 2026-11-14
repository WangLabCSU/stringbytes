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
//! # Features
//!
//! Enable `simd` to use SIMD-accelerated UTF-8 validation where supported.
//! This feature is disabled by default. Both configurations return the same
//! [`Utf8Error`] and [`FromUtf8Error`] types.
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
//! # Ok::<(), stringbytes::Utf8Error>(())
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
//! # Ok::<(), stringbytes::FromUtf8Error<Vec<u8>>>(())
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
//! # Ok::<(), stringbytes::Utf8Error>(())
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
//! # Ok::<(), stringbytes::FromUtf8Error<String>>(())
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
//! # Ok::<(), stringbytes::Utf8Error>(())
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
//! # Ok::<(), stringbytes::FromUtf8Error<CString>>(())
//! ```

use std::borrow::Cow;
use std::rc::Rc;
use std::str;
use std::sync::Arc;

mod error;
mod impls;
mod sealed;

#[doc(inline)]
pub use error::{FromUtf8Error, Utf8Error};

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
    /// # Ok::<(), stringbytes::Utf8Error>(())
    /// ```
    #[inline]
    fn try_borrow_str(&self) -> Result<&str, Utf8Error> {
        #[cfg(feature = "simd")]
        {
            // compat reports the error position directly, without a second
            // validation pass through std on invalid input.
            simdutf8::compat::from_utf8(self.as_bytes()).map_err(Utf8Error::from)
        }
        #[cfg(not(feature = "simd"))]
        {
            str::from_utf8(self.as_bytes()).map_err(Utf8Error::from)
        }
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
    /// Checks UTF-8 when needed, before copying. Invalid input returns an error
    /// without allocating or copying the bytes.
    ///
    /// # Errors
    ///
    /// Returns [`Utf8Error`] if the bytes are not valid UTF-8. The original bytes
    /// remain available through `self`.
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
    /// # Ok::<(), stringbytes::Utf8Error>(())
    /// ```
    #[inline]
    fn try_to_string(&self) -> Result<String, Utf8Error> {
        // The caller still owns the original bytes, so an error needs no copy.
        self.try_borrow_str().map(str::to_owned)
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
        let bytes = self.as_bytes();
        #[cfg(feature = "simd")]
        {
            match simdutf8::compat::from_utf8(bytes) {
                Ok(text) => Cow::Borrowed(text),
                Err(error) => {
                    let (valid, rest) = bytes.split_at(error.valid_up_to());
                    let mut text = String::with_capacity(bytes.len());
                    // SAFETY: compat validated the prefix before the first error.
                    text.push_str(unsafe { str::from_utf8_unchecked(valid) });
                    // Resume at the first error instead of rescanning the valid
                    // prefix. utf8_chunks supplies std's replacement boundaries.
                    for chunk in rest.utf8_chunks() {
                        text.push_str(chunk.valid());
                        if !chunk.invalid().is_empty() {
                            text.push('\u{fffd}');
                        }
                    }
                    Cow::Owned(text)
                }
            }
        }
        #[cfg(not(feature = "simd"))]
        {
            String::from_utf8_lossy(bytes)
        }
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
    /// On success, follows the allocation and copying rules of
    /// [`into_bytes`](Self::into_bytes).
    ///
    /// # Errors
    ///
    /// Returns [`FromUtf8Error<Self>`](FromUtf8Error) for invalid UTF-8, retaining
    /// the original value without converting it or allocating. Use
    /// [`into_inner`](FromUtf8Error::into_inner) to recover `self`.
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
    /// assert_eq!(error.into_inner(), vec![0xff]);
    /// # Ok::<(), stringbytes::FromUtf8Error<Box<[u8]>>>(())
    /// ```
    #[inline]
    fn try_into_string(self) -> Result<String, FromUtf8Error<Self>>
    where
        Self: Sized,
    {
        // Validate while self is intact so an error can return the original
        // value, including its pointer ownership or borrowed storage.
        match self.try_borrow_str() {
            Ok(_) => {
                // SAFETY: validation succeeded and into_bytes preserves the bytes.
                Ok(unsafe { String::from_utf8_unchecked(self.into_bytes()) })
            }
            Err(error) => Err(FromUtf8Error::from_parts(self, error)),
        }
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
    /// Returns [`FromUtf8Error`] containing the original pointer if the bytes
    /// are not valid UTF-8. The pointer is returned unchanged.
    #[doc(hidden)]
    #[inline]
    fn box_into_string(self: Box<Self>) -> Result<String, FromUtf8Error<Box<Self>>> {
        // Keep the pointer intact until validation succeeds; unwrapping first
        // would prevent returning the original allocation and ownership on error.
        match self.try_borrow_str() {
            Ok(_) => {
                // SAFETY: validation succeeded and box_into_bytes preserves the bytes.
                Ok(unsafe { String::from_utf8_unchecked(self.box_into_bytes()) })
            }
            Err(error) => Err(FromUtf8Error::from_parts(self, error)),
        }
    }

    /// Converts the contents of an [`Rc`] into a [`String`].
    ///
    /// Allocation reuse follows [`Self::rc_into_bytes`].
    ///
    /// # Errors
    ///
    /// Returns [`FromUtf8Error`] containing the original pointer if the bytes
    /// are not valid UTF-8. The pointer is returned unchanged.
    #[doc(hidden)]
    #[inline]
    fn rc_into_string(self: Rc<Self>) -> Result<String, FromUtf8Error<Rc<Self>>> {
        // Keep the pointer intact until validation succeeds; unwrapping first
        // would prevent returning the original allocation and ownership on error.
        match self.try_borrow_str() {
            Ok(_) => {
                // SAFETY: validation succeeded and rc_into_bytes preserves the bytes.
                Ok(unsafe { String::from_utf8_unchecked(self.rc_into_bytes()) })
            }
            Err(error) => Err(FromUtf8Error::from_parts(self, error)),
        }
    }

    /// Converts the contents of an [`Arc`] into a [`String`].
    ///
    /// Allocation reuse follows [`Self::arc_into_bytes`].
    ///
    /// # Errors
    ///
    /// Returns [`FromUtf8Error`] containing the original pointer if the bytes
    /// are not valid UTF-8. The pointer is returned unchanged.
    #[doc(hidden)]
    #[inline]
    fn arc_into_string(self: Arc<Self>) -> Result<String, FromUtf8Error<Arc<Self>>> {
        // Keep the pointer intact until validation succeeds; unwrapping first
        // would prevent returning the original allocation and ownership on error.
        match self.try_borrow_str() {
            Ok(_) => {
                // SAFETY: validation succeeded and arc_into_bytes preserves the bytes.
                Ok(unsafe { String::from_utf8_unchecked(self.arc_into_bytes()) })
            }
            Err(error) => Err(FromUtf8Error::from_parts(self, error)),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;
    use std::cell::RefCell;
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

    struct ByteBuffer(Vec<u8>);

    impl super::sealed::Sealed for ByteBuffer {}

    impl StringBytes for ByteBuffer {
        fn as_bytes(&self) -> &[u8] {
            &self.0
        }

        fn into_bytes(self) -> Vec<u8> {
            self.0
        }
    }

    struct TrackedConversions<'a> {
        bytes: &'a [u8],
        calls: &'a RefCell<Vec<&'static str>>,
    }

    impl super::sealed::Sealed for TrackedConversions<'_> {}

    impl StringBytes for TrackedConversions<'_> {
        fn as_bytes(&self) -> &[u8] {
            self.calls.borrow_mut().push("as_bytes");
            self.bytes
        }

        fn to_bytes(&self) -> Vec<u8> {
            self.calls.borrow_mut().push("to_bytes");
            self.bytes.to_vec()
        }

        fn into_bytes(self) -> Vec<u8> {
            self.calls.borrow_mut().push("into_bytes");
            self.bytes.to_vec()
        }

        fn try_borrow_str(&self) -> Result<&str, super::Utf8Error> {
            self.calls.borrow_mut().push("try_borrow_str");
            StringBytes::try_borrow_str(self.bytes)
        }
    }

    struct CheckedCalls<'a> {
        bytes: &'a [u8],
        calls: &'a RefCell<Vec<&'static str>>,
    }

    impl super::sealed::Sealed for CheckedCalls<'_> {}

    impl StringBytes for CheckedCalls<'_> {
        fn as_bytes(&self) -> &[u8] {
            self.bytes
        }

        fn try_borrow_str(&self) -> Result<&str, super::Utf8Error> {
            self.calls.borrow_mut().push("try_borrow_str");
            std::str::from_utf8(self.bytes).map_err(super::Utf8Error::from)
        }

        fn try_to_string(&self) -> Result<String, super::Utf8Error> {
            self.calls.borrow_mut().push("try_to_string");
            std::str::from_utf8(self.bytes)
                .map(str::to_owned)
                .map_err(super::Utf8Error::from)
        }

        fn try_into_string(self) -> Result<String, super::FromUtf8Error<Self>> {
            self.calls.borrow_mut().push("try_into_string");
            match std::str::from_utf8(self.bytes) {
                Ok(text) => Ok(text.to_owned()),
                Err(error) => Err(super::FromUtf8Error::from_parts(self, error.into())),
            }
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
    fn try_borrow_str_default_borrows_valid_text_across_block_boundaries() {
        for prefix_len in [
            0, 1, 31, 32, 60, 61, 62, 63, 64, 65, 124, 125, 126, 127, 128, 129,
        ] {
            for suffix in [
                "",
                "\0",
                "é",
                "中",
                "🦀",
                "\u{80}\u{7ff}\u{800}\u{d7ff}\u{e000}\u{ffff}\u{10000}\u{10ffff}",
            ] {
                let expected = "a".repeat(prefix_len) + suffix;
                let bytes = expected.as_bytes();
                let storage = ByteView(bytes);
                let actual = storage.try_borrow_str().unwrap();
                assert_eq!(actual, expected);
                assert!(std::ptr::eq(actual.as_bytes(), bytes));
            }
        }
    }

    #[test]
    fn try_borrow_str_default_reports_invalid_sequences_at_block_boundaries() {
        for prefix_len in [0, 60, 61, 62, 63, 64, 65, 124, 125, 126, 127, 128, 129] {
            for (suffix, error_len) in [
                (b"\xff".as_slice(), Some(1)),
                (b"\xc0\x80", Some(1)),
                (b"\xed\xa0\x80", Some(1)),
                (b"\xf4\x90\x80\x80", Some(1)),
                (b"\xe2(\xa1", Some(1)),
                (b"\xe2\x82(", Some(2)),
                (b"\xf0\x9f\xa6(", Some(3)),
                (b"\xe2\x82", None),
                (b"\xf0\x9f\xa6", None),
            ] {
                let mut bytes = vec![b'a'; prefix_len];
                bytes.extend_from_slice(suffix);
                let error = ByteView(&bytes).try_borrow_str().unwrap_err();
                assert_eq!(error.valid_up_to(), prefix_len);
                assert_eq!(error.error_len(), error_len);
                assert_eq!(error, std::str::from_utf8(&bytes).unwrap_err().into());
            }
        }
    }

    #[test]
    fn try_borrow_str_default_matches_std_for_byte_pairs_across_a_block_boundary() {
        // Sample every byte value next to representative UTF-8 boundary bytes.
        let mut bytes = vec![b'a'; 128];
        for first in 0..=u8::MAX {
            for second in [
                0, 0x7f, 0x80, 0x8f, 0x90, 0x9f, 0xa0, 0xbf, 0xc0, 0xc2, 0xdf, 0xe0, 0xed, 0xef,
                0xf0, 0xf4, 0xf5, 0xff,
            ] {
                bytes[63] = first;
                bytes[64] = second;
                assert_eq!(
                    ByteView(&bytes).try_borrow_str(),
                    std::str::from_utf8(&bytes).map_err(super::Utf8Error::from),
                    "{first:02x} {second:02x}"
                );
            }
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
        let calls = RefCell::new(Vec::new());
        let storage = CheckedCalls {
            bytes: "étext".as_bytes(),
            calls: &calls,
        };
        // SAFETY: the fixture exposes the valid UTF-8 literal above.
        let value = unsafe { storage.borrow_str_unchecked() };
        assert!(calls.borrow().is_empty());
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
    fn try_to_string_default_reports_invalid_utf8_without_changing_storage() {
        for (input, valid_up_to, error_len) in [
            (b"ok\xff".as_slice(), 2, Some(1)),
            (b"ok\xe2\x82", 2, None),
            (b"\xc0\x80", 0, Some(1)),
            (b"\xed\xa0\x80", 0, Some(1)),
        ] {
            let storage = ByteView(input);
            let error = StringBytes::try_to_string(&storage).unwrap_err();
            assert_eq!(error.valid_up_to(), valid_up_to);
            assert_eq!(error.error_len(), error_len);
            assert_eq!(StringBytes::as_bytes(&storage), input);
        }
    }

    #[test]
    fn try_to_string_default_copies_validated_text_without_to_bytes() {
        let storage = TrackedConversions {
            bytes: "étext".as_bytes(),
            calls: &RefCell::new(Vec::new()),
        };

        let text = storage.try_to_string().unwrap();

        assert_eq!(*storage.calls.borrow(), ["try_borrow_str"]);
        assert_eq!(text, "étext");
        assert_ne!(text.as_ptr(), storage.bytes.as_ptr());
        assert_eq!(storage.bytes, "étext".as_bytes());
    }

    #[test]
    fn try_to_string_default_reports_errors_without_to_bytes() {
        let storage = TrackedConversions {
            bytes: b"ok\xff",
            calls: &RefCell::new(Vec::new()),
        };

        let error = storage.try_to_string().unwrap_err();

        assert_eq!(*storage.calls.borrow(), ["try_borrow_str"]);
        assert_eq!(error.valid_up_to(), 2);
        assert_eq!(error.error_len(), Some(1));
        assert_eq!(storage.bytes, b"ok\xff");
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
    fn to_string_lossy_default_borrows_valid_text_across_block_boundaries() {
        for prefix_len in [0, 1, 60, 61, 62, 63, 64, 65, 124, 125, 126, 127, 128, 129] {
            let expected = "a".repeat(prefix_len) + "é中🦀";
            let bytes = expected.as_bytes();
            let storage = ByteView(bytes);
            let text = storage.to_string_lossy();
            assert!(matches!(text, Cow::Borrowed(_)));
            assert_eq!(text, expected);
            assert!(std::ptr::eq(text.as_bytes(), bytes));
        }
    }

    #[test]
    fn to_string_lossy_default_matches_std_replacements_at_block_boundaries() {
        for prefix_len in [0, 60, 61, 62, 63, 64, 65, 124, 125, 126, 127, 128, 129] {
            for (suffix, replacement) in [
                (b"\xff".as_slice(), "�"),
                (b"\xc0\x80", "��"),
                (b"\xed\xa0\x80", "���"),
                (b"\xe2\x82", "�"),
                (b"\xe2(\xa1", "�(�"),
                (b"\xf0\x9f\xa6(", "�("),
            ] {
                let mut bytes = vec![b'a'; prefix_len];
                bytes.extend_from_slice(suffix);
                let storage = ByteView(&bytes);
                let text = storage.to_string_lossy();
                assert!(matches!(text, Cow::Owned(_)));
                assert_eq!(text, "a".repeat(prefix_len) + replacement);
                assert_eq!(text, String::from_utf8_lossy(&bytes));
                assert_eq!(&bytes[prefix_len..], suffix);
            }
        }
    }

    #[test]
    fn to_string_lossy_default_preserves_unicode_between_multiple_errors() {
        let prefix = "é中🦀".repeat(8);
        let mut bytes = prefix.as_bytes().to_vec();
        bytes.push(0xff);
        bytes.extend_from_slice("中🦀".as_bytes());
        bytes.extend_from_slice(b"\xc0\x80ok\xe2\x82");
        let storage = ByteView(&bytes);

        let text = storage.to_string_lossy();

        assert!(matches!(text, Cow::Owned(_)));
        assert_eq!(text, format!("{prefix}�中🦀��ok�"));
        assert_eq!(text, String::from_utf8_lossy(&bytes));
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
        let calls = RefCell::new(Vec::new());
        let storage = CheckedCalls {
            bytes: "étext".as_bytes(),
            calls: &calls,
        };
        // SAFETY: the fixture exposes the valid UTF-8 literal above.
        let value = unsafe { storage.to_string_unchecked() };
        assert!(calls.borrow().is_empty());
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
    fn try_into_string_default_error_returns_original() {
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
            let bytes = error.into_inner();
            assert_eq!(StringBytes::as_bytes(&bytes), input);
            if !input.is_empty() {
                assert_eq!(StringBytes::as_bytes(&bytes).as_ptr(), address);
            }
        }
    }

    #[test]
    fn try_into_string_validates_before_converting_bytes() {
        let calls = RefCell::new(Vec::new());
        let storage = TrackedConversions {
            bytes: "étext".as_bytes(),
            calls: &calls,
        };

        assert_eq!(storage.try_into_string().unwrap(), "étext");
        assert_eq!(*calls.borrow(), ["try_borrow_str", "into_bytes"]);
    }

    #[test]
    fn try_into_string_returns_original_without_converting_bytes() {
        let calls = RefCell::new(Vec::new());
        let storage = TrackedConversions {
            bytes: b"ok\xff",
            calls: &calls,
        };
        let error = storage.try_into_string().unwrap_err();
        assert_eq!(error.utf8_error().valid_up_to(), 2);
        assert_eq!(error.utf8_error().error_len(), Some(1));
        let recovered = error.into_inner();
        assert_eq!(recovered.bytes, b"ok\xff");
        assert_eq!(*calls.borrow(), ["try_borrow_str"]);
    }

    #[test]
    fn try_into_string_default_keeps_valid_buffers_and_spare_capacity() {
        for prefix_len in [0, 1, 60, 61, 62, 63, 64, 65, 124, 125, 126, 127, 128, 129] {
            let expected = "a".repeat(prefix_len) + "é中🦀";
            let mut bytes = Vec::with_capacity(expected.len() + 32);
            bytes.extend_from_slice(expected.as_bytes());
            let address = bytes.as_ptr();
            let capacity = bytes.capacity();
            let text = ByteBuffer(bytes).try_into_string().unwrap();
            assert_eq!(text, expected);
            assert_eq!(text.as_ptr(), address);
            assert_eq!(text.capacity(), capacity);
        }
    }

    #[test]
    fn try_into_string_default_keeps_invalid_buffers_and_error_details() {
        for prefix_len in [0, 62, 63, 64, 65, 126, 127, 128, 129] {
            for (suffix, error_len) in [(b"\xff".as_slice(), Some(1)), (b"\xe2\x82", None)] {
                let mut expected = vec![b'a'; prefix_len];
                expected.extend_from_slice(suffix);
                let mut bytes = Vec::with_capacity(expected.len() + 32);
                bytes.extend_from_slice(&expected);
                let address = StringBytes::as_bytes(&bytes).as_ptr();
                let capacity = bytes.capacity();
                let error = ByteBuffer(bytes).try_into_string().unwrap_err();
                assert_eq!(error.utf8_error().valid_up_to(), prefix_len);
                assert_eq!(error.utf8_error().error_len(), error_len);
                let bytes = error.into_inner();
                assert_eq!(StringBytes::as_bytes(&bytes), &expected[..]);
                assert_eq!(StringBytes::as_bytes(&bytes).as_ptr(), address);
                assert_eq!(bytes.0.capacity(), capacity);
            }
        }
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
            let calls = RefCell::new(Vec::new());
            let storage = TrackedConversions {
                bytes: input,
                calls: &calls,
            };
            assert_eq!(storage.into_string_lossy(), expected);
            assert_eq!(*calls.borrow(), ["as_bytes"]);
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
        let calls = RefCell::new(Vec::new());
        let storage = CheckedCalls {
            bytes: "étext".as_bytes(),
            calls: &calls,
        };
        // SAFETY: the fixture exposes the valid UTF-8 literal above.
        let value = unsafe { storage.into_string_unchecked() };
        assert!(calls.borrow().is_empty());
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
    fn box_into_string_default_error_returns_original() {
        let storage = Box::new(ByteView(b"ok\xff"));
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let error = StringBytes::box_into_string(storage).unwrap_err();
        assert_eq!(error.utf8_error().valid_up_to(), 2);
        assert_eq!(error.utf8_error().error_len(), Some(1));
        let value = error.into_inner();
        assert_eq!(StringBytes::as_bytes(&value), b"ok\xff");
        assert_eq!(StringBytes::as_bytes(&value).as_ptr(), address);
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
    fn rc_into_string_default_error_returns_original() {
        let storage = Rc::new(ByteView(b"ok\xff"));
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let error = StringBytes::rc_into_string(storage).unwrap_err();
        assert_eq!(error.utf8_error().valid_up_to(), 2);
        assert_eq!(error.utf8_error().error_len(), Some(1));
        let value = error.into_inner();
        assert_eq!(StringBytes::as_bytes(&value), b"ok\xff");
        assert_eq!(StringBytes::as_bytes(&value).as_ptr(), address);
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
    fn arc_into_string_default_error_returns_original() {
        let storage = Arc::new(ByteView(b"ok\xff"));
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let error = StringBytes::arc_into_string(storage).unwrap_err();
        assert_eq!(error.utf8_error().valid_up_to(), 2);
        assert_eq!(error.utf8_error().error_len(), Some(1));
        let value = error.into_inner();
        assert_eq!(StringBytes::as_bytes(&value), b"ok\xff");
        assert_eq!(StringBytes::as_bytes(&value).as_ptr(), address);
    }
}
