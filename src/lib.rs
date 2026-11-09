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
/// [`into_string_lossy`](Self::into_string_lossy) replace invalid sequences
/// with `�` (U+FFFD), losing their original bytes.
///
/// This trait is sealed and cannot be implemented outside this crate.
///
/// # Guarantees
///
/// All implementations obey the following rules:
///
/// - [`Self::as_bytes`] returns the same bytes unless the value is modified
///   through mutable or owned access. Interior mutability must not change the
///   bytes exposed through this method, [`AsRef<[u8]>`](AsRef),
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
    /// For [`CStr`](std::ffi::CStr) and [`CString`](std::ffi::CString), it
    /// excludes the trailing NUL byte.
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
    /// For [`CStr`](std::ffi::CStr), use `StringBytes::to_bytes(value)` to
    /// distinguish this method from [`CStr::to_bytes`](std::ffi::CStr::to_bytes),
    /// which returns a borrowed slice.
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

    /// Returns a string, replacing invalid UTF-8 sequences with `�` (U+FFFD).
    ///
    /// Borrows valid UTF-8 without allocating. Otherwise, returns an owned
    /// string with the same replacements as [`String::from_utf8_lossy`].
    /// The original bytes remain unchanged. Types that already guarantee UTF-8
    /// skip validation.
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
    /// Byte sequences are validated once. Types that already guarantee UTF-8
    /// skip validation and always return [`Ok`].
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

    /// Converts `self` into a [`String`], replacing invalid UTF-8 with `�` (U+FFFD).
    ///
    /// Valid UTF-8 follows the allocation rules of
    /// [`into_bytes`](Self::into_bytes). Invalid sequences are replaced
    /// as in [`String::from_utf8_lossy`], without guaranteeing reuse of the
    /// original allocation. Types that already guarantee UTF-8 skip validation.
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
        // not need an intermediate copy. Text types skip validation here.
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
    // Owned pointer receivers preserve the concrete type's conversion through
    // dyn StringBytes.
    // ------------------------------------------------------------------------

    /// Converts the contents of a [`Box`] into a [`Vec<u8>`].
    ///
    /// Boxed byte slices, byte arrays, [`str`](prim@str), and
    /// [`CStr`](std::ffi::CStr) reuse their allocation.
    /// Other boxed values follow the allocation rules of [`Self::into_bytes`].
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
    /// Boxed byte slices, byte arrays, [`str`](prim@str), and
    /// [`CStr`](std::ffi::CStr) reuse their allocation.
    /// Other boxed values follow the allocation rules of [`Self::try_into_string`].
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
mod test_utils {
    use std::borrow::Cow;
    use std::ops::Deref;
    use std::str::Utf8Error;
    use std::string::FromUtf8Error;

    use super::StringBytes;

    // Concrete inputs stay in each implementation module. Conversion checks
    // return their results so callers can also check copying or allocation reuse.

    pub(super) fn assert_stable_bytes<T: StringBytes + ?Sized>(storage: &T) {
        let expected = StringBytes::as_bytes(storage).to_vec();

        for _ in 0..3 {
            assert_eq!(StringBytes::as_bytes(storage), expected);

            let _ = StringBytes::to_bytes(storage);
            assert_eq!(StringBytes::as_bytes(storage), expected);

            let _ = StringBytes::try_borrow_str(storage);
            assert_eq!(StringBytes::as_bytes(storage), expected);

            let _ = StringBytes::try_to_string(storage);
            assert_eq!(StringBytes::as_bytes(storage), expected);

            let _ = StringBytes::to_string_lossy(storage);
            assert_eq!(StringBytes::as_bytes(storage), expected);
        }
    }

    pub(super) fn assert_as_ref_matches_as_bytes<T>(storage: &T)
    where
        T: StringBytes + AsRef<[u8]> + ?Sized,
    {
        let expected = StringBytes::as_bytes(storage).to_vec();

        for _ in 0..3 {
            let bytes = StringBytes::as_bytes(storage);
            let borrowed = <T as AsRef<[u8]>>::as_ref(storage);
            // Slice pointer equality checks both the address and the length.
            assert!(std::ptr::eq(bytes, borrowed));
            assert_eq!(borrowed, expected);
            assert_eq!(StringBytes::as_bytes(storage), expected);
        }
    }

    pub(super) fn assert_as_mut_matches_as_bytes<T>(storage: &mut T)
    where
        T: StringBytes + AsMut<[u8]> + ?Sized,
    {
        let bytes = StringBytes::as_bytes(storage);
        let address = bytes.as_ptr();
        let mut expected = bytes.to_vec();

        {
            let borrowed = <T as AsMut<[u8]>>::as_mut(storage);
            assert_eq!(borrowed.as_ptr(), address);
            assert_eq!(borrowed.len(), expected.len());
            assert_eq!(borrowed, expected);

            // Mutations must affect the bytes subsequently observed by as_bytes.
            for (byte, expected_byte) in borrowed.iter_mut().zip(&mut expected) {
                *expected_byte = expected_byte.wrapping_add(1);
                *byte = *expected_byte;
            }
        }

        let bytes = StringBytes::as_bytes(storage);
        assert_eq!(bytes.as_ptr(), address);
        assert_eq!(bytes, expected);
    }

    pub(super) fn assert_deref_matches_as_bytes<T>(storage: &T)
    where
        T: StringBytes + Deref + ?Sized,
        T::Target: StringBytes,
    {
        let expected = StringBytes::as_bytes(storage).to_vec();

        for _ in 0..3 {
            let bytes = StringBytes::as_bytes(storage);
            let target_bytes = StringBytes::as_bytes(T::deref(storage));
            assert!(std::ptr::eq(bytes, target_bytes));
            assert_eq!(target_bytes, expected);
            assert_eq!(StringBytes::as_bytes(storage), expected);
        }
    }

    pub(super) fn assert_to_bytes_preserves_bytes<T: StringBytes + ?Sized>(storage: &T) -> Vec<u8> {
        let expected = StringBytes::as_bytes(storage).to_vec();
        let address = StringBytes::as_bytes(storage).as_ptr();
        let mut bytes = StringBytes::to_bytes(storage);
        assert_eq!(bytes, expected);
        if !bytes.is_empty() {
            assert_ne!(bytes.as_ptr(), address);
            bytes[0] ^= 0xff;
            assert_eq!(StringBytes::as_bytes(storage), expected);
            bytes[0] ^= 0xff;
        }
        assert_eq!(StringBytes::as_bytes(storage), expected);
        assert_eq!(StringBytes::as_bytes(storage).as_ptr(), address);
        bytes
    }

    pub(super) fn assert_into_bytes_preserves_bytes<T: StringBytes>(storage: T) -> Vec<u8> {
        let expected = StringBytes::as_bytes(&storage).to_vec();
        let bytes = StringBytes::into_bytes(storage);
        assert_eq!(bytes, expected);
        bytes
    }

    pub(super) fn assert_into_bytes_reuses_allocation<T: StringBytes>(
        storage: T,
        expected: &[u8],
        address: *const u8,
        capacity: usize,
    ) {
        let bytes = assert_into_bytes_preserves_bytes(storage);
        assert_eq!(bytes, expected);
        assert_eq!(bytes.capacity(), capacity);
        // Zero capacity has no heap allocation whose address must be preserved.
        if capacity != 0 {
            assert_eq!(bytes.as_ptr(), address);
        }
    }

    pub(super) fn assert_try_borrow_str_preserves_bytes<T: StringBytes + ?Sized>(
        storage: &T,
    ) -> Result<&str, Utf8Error> {
        let expected = StringBytes::as_bytes(storage).to_vec();
        let result = StringBytes::try_borrow_str(storage);
        assert_eq!(result, std::str::from_utf8(&expected));
        if let Ok(text) = result {
            assert!(std::ptr::eq(
                text.as_bytes(),
                StringBytes::as_bytes(storage)
            ));
        }
        assert_eq!(StringBytes::as_bytes(storage), expected);
        result
    }

    pub(super) fn assert_borrow_str_unchecked_preserves_bytes<T: StringBytes + ?Sized>(
        storage: &T,
    ) -> &str {
        let expected = StringBytes::as_bytes(storage).to_vec();
        let expected_text = std::str::from_utf8(&expected).unwrap();
        // SAFETY: the bytes were validated above and shared access preserves them.
        let text = unsafe { StringBytes::borrow_str_unchecked(storage) };
        assert_eq!(text, expected_text);
        assert!(std::ptr::eq(
            text.as_bytes(),
            StringBytes::as_bytes(storage)
        ));
        text
    }

    pub(super) fn assert_try_to_string_preserves_bytes<T: StringBytes + ?Sized>(
        storage: &T,
    ) -> Result<String, FromUtf8Error> {
        let expected = StringBytes::as_bytes(storage).to_vec();
        let address = StringBytes::as_bytes(storage).as_ptr();
        let mut result = StringBytes::try_to_string(storage);
        assert_string_result(&result, &expected);
        match &mut result {
            Ok(text) => {
                if !text.is_empty() {
                    assert_ne!(text.as_ptr(), address);
                }
                text.make_ascii_uppercase();
                assert_eq!(StringBytes::as_bytes(storage), expected);
                // Restore the contents for the caller's additional assertions.
                text.clear();
                text.push_str(std::str::from_utf8(&expected).unwrap());
            }
            Err(error) => assert_ne!(error.as_bytes().as_ptr(), address),
        }
        assert_eq!(StringBytes::as_bytes(storage), expected);
        assert_eq!(StringBytes::as_bytes(storage).as_ptr(), address);
        result
    }

    // Check both outcomes so always returning an error cannot pass the test.
    fn assert_string_result(result: &Result<String, FromUtf8Error>, expected: &[u8]) {
        match std::str::from_utf8(expected) {
            Ok(text) => assert_eq!(result.as_ref().unwrap().as_str(), text),
            Err(expected_error) => {
                let error = result.as_ref().unwrap_err();
                assert_eq!(error.utf8_error(), expected_error);
                assert_eq!(error.as_bytes(), expected);
            }
        }
    }

    pub(super) fn assert_to_string_lossy_matches_std<T: StringBytes + ?Sized>(
        storage: &T,
    ) -> Cow<'_, str> {
        let expected = StringBytes::as_bytes(storage).to_vec();
        let text = StringBytes::to_string_lossy(storage);
        assert_eq!(text, String::from_utf8_lossy(&expected));
        match &text {
            Cow::Borrowed(text) => {
                assert!(std::str::from_utf8(&expected).is_ok());
                // The standard library may return a static string for empty input.
                if !text.is_empty() {
                    assert!(std::ptr::eq(
                        text.as_bytes(),
                        StringBytes::as_bytes(storage)
                    ));
                }
            }
            Cow::Owned(text) => {
                assert!(std::str::from_utf8(&expected).is_err());
                assert_ne!(text.as_ptr(), StringBytes::as_bytes(storage).as_ptr());
            }
        }
        assert_eq!(StringBytes::as_bytes(storage), expected);
        text
    }

    pub(super) fn assert_to_string_unchecked_preserves_bytes<T: StringBytes + ?Sized>(
        storage: &T,
    ) -> String {
        let expected = StringBytes::as_bytes(storage).to_vec();
        let expected_text = std::str::from_utf8(&expected).unwrap();
        // SAFETY: the bytes were validated above and shared access preserves them.
        let text = unsafe { StringBytes::to_string_unchecked(storage) };
        assert_eq!(text, expected_text);
        assert_eq!(StringBytes::as_bytes(storage), expected);
        if !text.is_empty() {
            assert_ne!(text.as_ptr(), StringBytes::as_bytes(storage).as_ptr());
        }
        text
    }

    pub(super) fn assert_try_into_string_preserves_bytes<T: StringBytes>(
        storage: T,
    ) -> Result<String, FromUtf8Error> {
        let expected = StringBytes::as_bytes(&storage).to_vec();
        let result = StringBytes::try_into_string(storage);
        assert_string_result(&result, &expected);
        result
    }

    pub(super) fn assert_try_into_string_reuses_allocation<T: StringBytes>(
        storage: T,
        expected: &[u8],
        address: *const u8,
        capacity: usize,
    ) {
        let bytes = match assert_try_into_string_preserves_bytes(storage) {
            Ok(text) => text.into_bytes(),
            Err(error) => error.into_bytes(),
        };
        assert_eq!(bytes, expected);
        assert_eq!(bytes.capacity(), capacity);
        if capacity != 0 {
            assert_eq!(bytes.as_ptr(), address);
        }
    }

    pub(super) fn assert_into_string_lossy_matches_std<T: StringBytes>(storage: T) -> String {
        let expected = String::from_utf8_lossy(StringBytes::as_bytes(&storage)).into_owned();
        let text = StringBytes::into_string_lossy(storage);
        assert_eq!(text, expected);
        text
    }

    pub(super) fn assert_into_string_lossy_reuses_valid_allocation<T: StringBytes>(
        storage: T,
        expected: &[u8],
        address: *const u8,
        capacity: usize,
    ) {
        let text = assert_into_string_lossy_matches_std(storage);
        assert_eq!(text, String::from_utf8_lossy(expected));
        if std::str::from_utf8(expected).is_ok() {
            assert_eq!(text.capacity(), capacity);
            if capacity != 0 {
                assert_eq!(text.as_ptr(), address);
            }
        }
    }

    pub(super) fn assert_into_string_unchecked_preserves_bytes<T: StringBytes>(
        storage: T,
    ) -> String {
        let expected = StringBytes::as_bytes(&storage).to_vec();
        let expected_text = std::str::from_utf8(&expected).unwrap();
        // SAFETY: the bytes were validated above; conversion preserves them.
        let text = unsafe { StringBytes::into_string_unchecked(storage) };
        assert_eq!(text, expected_text);
        text
    }

    pub(super) fn assert_into_string_unchecked_reuses_allocation<T: StringBytes>(
        storage: T,
        expected: &[u8],
        address: *const u8,
        capacity: usize,
    ) {
        let text = assert_into_string_unchecked_preserves_bytes(storage);
        assert_eq!(text.as_bytes(), expected);
        assert_eq!(text.capacity(), capacity);
        if capacity != 0 {
            assert_eq!(text.as_ptr(), address);
        }
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
    fn unchecked_methods_do_not_call_checked_conversions() {
        struct ValidBytes;

        impl super::sealed::Sealed for ValidBytes {}

        impl StringBytes for ValidBytes {
            fn as_bytes(&self) -> &[u8] {
                "étext".as_bytes()
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

        // SAFETY: ValidBytes always exposes the UTF-8 literal above.
        unsafe {
            assert_eq!(ValidBytes.borrow_str_unchecked(), "étext");
            assert_eq!(ValidBytes.to_string_unchecked(), "étext");
            assert_eq!(ValidBytes.into_string_unchecked(), "étext");
            let storage: Box<dyn StringBytes> = Box::new(ValidBytes);
            assert_eq!(storage.borrow_str_unchecked(), "étext");
            assert_eq!(storage.to_string_unchecked(), "étext");
            assert_eq!(storage.into_string_unchecked(), "étext");
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

    #[test]
    fn into_string_lossy_does_not_copy_invalid_bytes_before_conversion() {
        struct InvalidBytes(&'static [u8]);

        impl super::sealed::Sealed for InvalidBytes {}

        impl StringBytes for InvalidBytes {
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

        for input in [b"a\xffb".as_slice(), b"ok\xe2\x82", b"\xed\xa0\x80"] {
            assert_eq!(
                InvalidBytes(input).into_string_lossy(),
                String::from_utf8_lossy(input)
            );
        }
    }
}
