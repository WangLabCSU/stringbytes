//! Errors returned by checked string conversions.

use std::fmt;

use crate::StringBytes;

/// An error returned when a byte sequence is not valid UTF-8.
///
/// The position and length have the same meaning as in [`std::str::Utf8Error`].
/// [`StringBytes::try_borrow_str`] and [`StringBytes::try_to_string`] return this
/// type with or without the `simd` feature.
///
/// # Examples
///
/// ```
/// use stringbytes::StringBytes;
///
/// let error = b"hi\xff".try_borrow_str().unwrap_err();
/// assert_eq!(error.valid_up_to(), 2);
/// assert_eq!(error.error_len(), Some(1));
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Utf8Error {
    valid_up_to: usize,
    error_len: Option<u8>,
}

impl Utf8Error {
    /// Returns the length of the valid UTF-8 prefix before the first error.
    #[must_use]
    #[inline]
    pub const fn valid_up_to(&self) -> usize {
        self.valid_up_to
    }

    /// Returns the length of the invalid sequence, or [`None`] if the input ends
    /// in an incomplete sequence. Skipping this many bytes allows validation to
    /// resume after the error.
    #[must_use]
    #[inline]
    pub const fn error_len(&self) -> Option<usize> {
        match self.error_len {
            Some(len) => Some(len as usize),
            None => None,
        }
    }
}

/// Copies the error details without validating the input again.
impl From<std::str::Utf8Error> for Utf8Error {
    #[inline]
    fn from(error: std::str::Utf8Error) -> Self {
        Self {
            valid_up_to: error.valid_up_to(),
            // UTF-8 errors span at most three bytes before validation can resume.
            error_len: error.error_len().map(|len| len as u8),
        }
    }
}

/// Copies the SIMD validation error details without revalidating the input.
#[cfg(feature = "simd")]
impl From<simdutf8::compat::Utf8Error> for Utf8Error {
    #[inline]
    fn from(error: simdutf8::compat::Utf8Error) -> Self {
        Self {
            valid_up_to: error.valid_up_to(),
            // compat reports the same sequence lengths as std::str::Utf8Error.
            error_len: error.error_len().map(|len| len as u8),
        }
    }
}

impl fmt::Display for Utf8Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.error_len() {
            Some(len) => write!(
                f,
                "invalid utf-8 sequence of {len} bytes from index {}",
                self.valid_up_to
            ),
            None => write!(
                f,
                "incomplete utf-8 byte sequence from index {}",
                self.valid_up_to
            ),
        }
    }
}

impl std::error::Error for Utf8Error {}

/// An error returned when converting a value into a [`String`].
///
/// The error retains the original value without converting it or allocating.
/// Use [`into_inner`](Self::into_inner) to recover that value, and
/// [`utf8_error`](Self::utf8_error) for the position and length of the first error.
///
/// # Examples
///
/// ```
/// use stringbytes::StringBytes;
///
/// let bytes = Box::<[u8]>::from(b"hi\xff".as_slice());
/// let address = bytes.as_ptr();
/// let error = bytes.try_into_string().unwrap_err();
/// assert_eq!(error.as_bytes(), b"hi\xff");
/// assert_eq!(error.utf8_error().valid_up_to(), 2);
/// let bytes: Box<[u8]> = error.into_inner();
/// assert_eq!(&*bytes, b"hi\xff");
/// assert_eq!(bytes.as_ptr(), address);
/// ```
#[derive(Eq, PartialEq)]
pub struct FromUtf8Error<T> {
    value: T,
    error: Utf8Error,
}

/// Cloning follows `T`'s [`Clone`] implementation.
impl<T: Clone> Clone for FromUtf8Error<T> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            error: self.error,
        }
    }

    #[inline]
    fn clone_from(&mut self, source: &Self) {
        // Delegate to T so types such as Vec can retain usable capacity instead
        // of replacing the entire value with a fresh allocation.
        self.value.clone_from(&source.value);
        self.error = source.error;
    }
}

impl<T> FromUtf8Error<T> {
    /// Keeps the original value and its validation error without converting it.
    #[inline]
    pub(super) fn from_parts(value: T, error: Utf8Error) -> Self {
        Self { value, error }
    }

    /// Borrows the original value.
    #[must_use]
    #[inline]
    pub const fn get_ref(&self) -> &T {
        &self.value
    }

    /// Returns the original value without converting it or allocating.
    #[must_use]
    #[inline]
    pub fn into_inner(self) -> T {
        self.value
    }

    /// Returns the position and length of the first UTF-8 error.
    #[must_use]
    #[inline]
    pub const fn utf8_error(&self) -> Utf8Error {
        self.error
    }
}

impl<T: StringBytes> FromUtf8Error<T> {
    /// Borrows the bytes that could not be converted into a string.
    #[must_use]
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        self.value.as_bytes()
    }

    /// Converts the original value into bytes, following
    /// [`StringBytes::into_bytes`]'s allocation and copying rules.
    ///
    /// Use [`into_inner`](Self::into_inner) to recover the original type instead.
    #[must_use]
    #[inline]
    pub fn into_bytes(self) -> Vec<u8> {
        self.value.into_bytes()
    }
}

/// Transfers the original buffer and error details without copying or
/// revalidating the bytes.
impl From<std::string::FromUtf8Error> for FromUtf8Error<Vec<u8>> {
    #[inline]
    fn from(error: std::string::FromUtf8Error) -> Self {
        let details = error.utf8_error().into();
        Self::from_parts(error.into_bytes(), details)
    }
}

// Formatting the byte view keeps generic StringBytes callers from needing an
// additional Debug bound on their storage type.
impl<T: StringBytes> fmt::Debug for FromUtf8Error<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FromUtf8Error")
            .field("bytes", &self.as_bytes())
            .field("error", &self.error)
            .finish()
    }
}

impl<T> fmt::Display for FromUtf8Error<T> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(f)
    }
}

impl<T: StringBytes> std::error::Error for FromUtf8Error<T> {
    #[inline]
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}

#[cfg(test)]
mod tests {
    use super::{FromUtf8Error, Utf8Error};
    use crate::StringBytes;

    #[test]
    fn utf8_error_from_std_preserves_positions_and_lengths() {
        for (bytes, valid_up_to, error_len) in [
            (b"ok\xff".as_slice(), 2, Some(1)),
            (b"ok\xe2\x82(", 2, Some(2)),
            (b"ok\xf0\x9f\xa6(", 2, Some(3)),
            (b"ok\xe2\x82", 2, None),
            (b"\xc3\xa9\xff", 2, Some(1)),
        ] {
            let error = Utf8Error::from(std::str::from_utf8(bytes).unwrap_err());

            assert_eq!(error.valid_up_to(), valid_up_to);
            assert_eq!(error.error_len(), error_len);
        }
    }

    #[cfg(feature = "simd")]
    #[test]
    fn utf8_error_from_simd_preserves_positions_and_lengths() {
        for prefix_len in [0, 63, 64, 127, 128] {
            for (suffix, error_len) in [
                (b"\xff".as_slice(), Some(1)),
                (b"\xe2\x82(", Some(2)),
                (b"\xf0\x9f\xa6(", Some(3)),
                (b"\xe2\x82", None),
            ] {
                let mut bytes = vec![b'a'; prefix_len];
                bytes.extend_from_slice(suffix);
                let error = Utf8Error::from(simdutf8::compat::from_utf8(&bytes).unwrap_err());

                assert_eq!(error.valid_up_to(), prefix_len);
                assert_eq!(error.error_len(), error_len);
            }
        }
    }

    #[test]
    fn utf8_error_display_reports_an_invalid_sequence() {
        let error = Utf8Error {
            valid_up_to: 2,
            error_len: Some(2),
        };

        assert_eq!(
            error.to_string(),
            "invalid utf-8 sequence of 2 bytes from index 2"
        );
    }

    #[test]
    fn utf8_error_display_reports_an_incomplete_sequence() {
        let error = Utf8Error {
            valid_up_to: 2,
            error_len: None,
        };

        assert_eq!(
            error.to_string(),
            "incomplete utf-8 byte sequence from index 2"
        );
    }

    #[test]
    fn from_utf8_error_from_std_keeps_the_buffer_and_details() {
        let mut bytes = Vec::with_capacity(32);
        bytes.extend_from_slice(b"ok\xe2\x82");
        let address = bytes.as_ptr();
        let capacity = bytes.capacity();

        let error = FromUtf8Error::from(String::from_utf8(bytes).unwrap_err());

        assert_eq!(error.as_bytes(), b"ok\xe2\x82");
        assert_eq!(error.as_bytes().as_ptr(), address);
        assert_eq!(error.utf8_error().valid_up_to(), 2);
        assert_eq!(error.utf8_error().error_len(), None);
        let bytes = error.into_bytes();
        assert_eq!(bytes, b"ok\xe2\x82");
        assert_eq!(bytes.as_ptr(), address);
        assert_eq!(bytes.capacity(), capacity);
    }

    #[test]
    fn from_utf8_error_get_ref_borrows_original_storage() {
        let storage = Box::<[u8]>::from(b"ok\xff".as_slice());
        let address = storage.as_ptr();
        let error = storage.try_into_string().unwrap_err();

        let original = error.get_ref();
        assert_eq!(&**original, b"ok\xff");
        assert_eq!(original.as_ptr(), address);
    }

    #[test]
    fn from_utf8_error_into_inner_preserves_borrowed_storage() {
        let source = b"ok\xff".to_vec();
        let error = source.as_slice().try_into_string().unwrap_err();

        let original: &[u8] = error.into_inner();
        assert!(std::ptr::eq(original, source.as_slice()));
    }

    #[test]
    fn from_utf8_error_into_bytes_converts_original_box_without_reallocating() {
        let storage = Box::<[u8]>::from(b"ok\xff".as_slice());
        let address = storage.as_ptr();
        let error = storage.try_into_string().unwrap_err();

        let bytes = error.into_bytes();
        assert_eq!(bytes, b"ok\xff");
        assert_eq!(bytes.as_ptr(), address);
        assert_eq!(bytes.capacity(), 3);
    }

    #[test]
    fn from_utf8_error_clone_owns_an_independent_buffer() {
        let source = FromUtf8Error::from(String::from_utf8(b"ok\xff".to_vec()).unwrap_err());

        let copy = source.clone();

        assert_eq!(copy.utf8_error(), source.utf8_error());
        assert_eq!(copy.as_bytes(), b"ok\xff");
        assert_ne!(copy.as_bytes().as_ptr(), source.as_bytes().as_ptr());
        let mut bytes = copy.into_bytes();
        bytes[0] = b'n';
        assert_eq!(source.as_bytes(), b"ok\xff");
    }

    #[test]
    fn from_utf8_error_clone_from_reuses_destination_capacity() {
        let mut bytes = Vec::with_capacity(64);
        bytes.extend_from_slice(b"old\xff");
        let address = bytes.as_ptr();
        let capacity = bytes.capacity();
        let mut destination = FromUtf8Error::from(String::from_utf8(bytes).unwrap_err());
        let source = FromUtf8Error::from(String::from_utf8(b"new\xe2\x82".to_vec()).unwrap_err());

        destination.clone_from(&source);

        assert_eq!(destination.as_bytes(), b"new\xe2\x82");
        assert_eq!(destination.utf8_error().valid_up_to(), 3);
        assert_eq!(destination.utf8_error().error_len(), None);
        let bytes = destination.into_bytes();
        assert_eq!(bytes.as_ptr(), address);
        assert_eq!(bytes.capacity(), capacity);
        assert_eq!(source.as_bytes(), b"new\xe2\x82");
    }

    #[test]
    fn from_utf8_error_clone_shares_reference_counted_storage() {
        use std::rc::Rc;

        let source = Rc::new(b"ok\xff".to_vec()).try_into_string().unwrap_err();
        let copy = source.clone();

        assert_eq!(copy.utf8_error(), source.utf8_error());
        assert!(Rc::ptr_eq(copy.get_ref(), source.get_ref()));
        assert_eq!(Rc::strong_count(source.get_ref()), 2);
    }

    #[test]
    fn from_utf8_error_debug_works_with_storage_without_debug() {
        struct Storage;
        impl crate::sealed::Sealed for Storage {}
        impl StringBytes for Storage {
            fn as_bytes(&self) -> &[u8] {
                b"ok\xff"
            }
        }

        let error = Storage.try_into_string().unwrap_err();
        assert_eq!(
            format!("{error:?}"),
            "FromUtf8Error { bytes: [111, 107, 255], error: Utf8Error { valid_up_to: 2, error_len: Some(1) } }"
        );
    }

    #[test]
    fn from_utf8_error_display_reports_the_utf8_error() {
        let error = FromUtf8Error::from(String::from_utf8(b"ok\xe2\x82(".to_vec()).unwrap_err());

        assert_eq!(
            error.to_string(),
            "invalid utf-8 sequence of 2 bytes from index 2"
        );
    }

    #[test]
    fn from_utf8_error_source_exposes_the_utf8_error() {
        use std::error::Error;

        let error = FromUtf8Error::from(String::from_utf8(b"ok\xff".to_vec()).unwrap_err());
        let source = error.source().unwrap().downcast_ref::<Utf8Error>().unwrap();

        assert_eq!(*source, error.utf8_error());
        assert_eq!(source.valid_up_to(), 2);
        assert_eq!(source.error_len(), Some(1));
        assert!(source.source().is_none());
    }

    #[test]
    fn from_utf8_error_source_supports_non_static_storage() {
        use std::error::Error;

        let mut bytes = *b"ok\xff";
        let error = bytes.as_mut_slice().try_into_string().unwrap_err();
        let source = error.source().unwrap().downcast_ref::<Utf8Error>().unwrap();
        assert_eq!(source.valid_up_to(), 2);
        assert_eq!(source.error_len(), Some(1));
    }
}
