use std::borrow::Cow;
use std::rc::Rc;
use std::sync::Arc;

use crate::impls::{impl_pointer_conversions_for_sized, impl_string_bytes};
use crate::{sealed, StringBytes};

impl_string_bytes!(
    /// Borrows the initialized bytes of the borrowed or owned vector.
    ///
    /// [`into_bytes`](StringBytes::into_bytes) returns an owned vector unchanged
    /// or clones a borrowed vector. [`to_bytes`](StringBytes::to_bytes) copies
    /// the bytes in either case.
    ///
    /// Checked and lossy string conversions validate UTF-8.
    Cow<'_, Vec<u8>> {
        /// Borrows the initialized bytes of the borrowed or owned vector.
        #[inline]
        fn as_bytes(&self) -> &[u8] {
            self.as_ref().as_slice()
        }

        /// Returns an owned vector unchanged or clones a borrowed vector.
        #[inline]
        fn into_bytes(self) -> Vec<u8> {
            // Reuse an owned Vec, including its spare capacity.
            self.into_owned()
        }

        impl_pointer_conversions_for_sized!();
    }
);

#[cfg(test)]
// Cow<Vec<u8>> has its own implementation to exercise.
#[allow(clippy::owned_cow)]
mod tests {
    use std::borrow::Cow;
    use std::rc::Rc;
    use std::sync::Arc;

    use crate::StringBytes;

    #[test]
    fn as_bytes_cow_vec_borrowed_exposes_stable_contents() {
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
            let source = input.to_vec();
            let storage: Cow<'_, Vec<u8>> = Cow::Borrowed(&source);
            assert_eq!(StringBytes::as_bytes(&storage), input);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            assert_eq!(StringBytes::as_bytes(&storage), input);
            assert_eq!(StringBytes::as_bytes(&storage).as_ptr(), address);
        }
    }

    #[test]
    fn as_bytes_cow_vec_borrowed_matches_deref() {
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
            let source = input.to_vec();
            let storage: Cow<'_, Vec<u8>> = Cow::Borrowed(&source);
            let bytes = StringBytes::as_bytes(&storage);
            let other = StringBytes::as_bytes(std::ops::Deref::deref(&storage));
            assert_eq!(bytes, input);
            assert_eq!(other, input);
            assert!(std::ptr::eq(bytes, other));
        }
    }

    #[test]
    fn as_bytes_cow_vec_owned_exposes_stable_contents() {
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
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let storage: Cow<'_, Vec<u8>> = Cow::Owned(source);
            assert_eq!(StringBytes::as_bytes(&storage), input);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            assert_eq!(StringBytes::as_bytes(&storage), input);
            assert_eq!(StringBytes::as_bytes(&storage).as_ptr(), address);
        }
    }

    #[test]
    fn as_bytes_cow_vec_owned_matches_deref() {
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
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let storage: Cow<'_, Vec<u8>> = Cow::Owned(source);
            let bytes = StringBytes::as_bytes(&storage);
            let other = StringBytes::as_bytes(std::ops::Deref::deref(&storage));
            assert_eq!(bytes, input);
            assert_eq!(other, input);
            assert!(std::ptr::eq(bytes, other));
        }
    }

    #[test]
    fn to_bytes_cow_vec_borrowed_returns_an_independent_copy() {
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
            let source = input.to_vec();
            let storage: Cow<'_, Vec<u8>> = Cow::Borrowed(&source);
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
    fn to_bytes_cow_vec_owned_returns_an_independent_copy() {
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
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let storage: Cow<'_, Vec<u8>> = Cow::Owned(source);
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
    fn into_bytes_cow_vec_borrowed_copies_storage() {
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
            let source = input.to_vec();
            let storage: Cow<'_, Vec<u8>> = Cow::Borrowed(&source);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let bytes = StringBytes::into_bytes(storage);
            assert_eq!(bytes, input);
            if !input.is_empty() {
                assert_ne!(bytes.as_ptr(), address);
            }
        }
    }

    #[test]
    fn into_bytes_cow_vec_owned_reuses_buffer() {
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
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let source_capacity = source.capacity();
            let storage: Cow<'_, Vec<u8>> = Cow::Owned(source);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let capacity = source_capacity;
            let bytes = StringBytes::into_bytes(storage);
            assert_eq!(bytes, input);
            assert_eq!(bytes.capacity(), capacity);
            if capacity != 0 {
                assert_eq!(bytes.as_ptr(), address);
            }
        }
    }

    #[test]
    fn try_borrow_str_cow_vec_borrowed_borrows_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let source = input.to_vec();
            let storage: Cow<'_, Vec<u8>> = Cow::Borrowed(&source);
            let value = StringBytes::try_borrow_str(&storage).unwrap();
            assert_eq!(value, expected);
            assert!(std::ptr::eq(
                value.as_bytes(),
                StringBytes::as_bytes(&storage)
            ));
        }
    }

    #[test]
    fn try_borrow_str_cow_vec_borrowed_reports_invalid_utf8() {
        for (input, valid_up_to, error_len) in [
            (b"ok\xff".as_slice(), 2, Some(1)),
            (b"ok\xe2\x82", 2, None),
            (b"\xc0\x80", 0, Some(1)),
            (b"\xed\xa0\x80", 0, Some(1)),
        ] {
            let source = input.to_vec();
            let storage: Cow<'_, Vec<u8>> = Cow::Borrowed(&source);
            let error = StringBytes::try_borrow_str(&storage).unwrap_err();
            assert_eq!(error.valid_up_to(), valid_up_to);
            assert_eq!(error.error_len(), error_len);
            assert_eq!(StringBytes::as_bytes(&storage), input);
        }
    }

    #[test]
    fn try_borrow_str_cow_vec_owned_borrows_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let storage: Cow<'_, Vec<u8>> = Cow::Owned(source);
            let value = StringBytes::try_borrow_str(&storage).unwrap();
            assert_eq!(value, expected);
            assert!(std::ptr::eq(
                value.as_bytes(),
                StringBytes::as_bytes(&storage)
            ));
        }
    }

    #[test]
    fn try_borrow_str_cow_vec_owned_reports_invalid_utf8() {
        for (input, valid_up_to, error_len) in [
            (b"ok\xff".as_slice(), 2, Some(1)),
            (b"ok\xe2\x82", 2, None),
            (b"\xc0\x80", 0, Some(1)),
            (b"\xed\xa0\x80", 0, Some(1)),
        ] {
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let storage: Cow<'_, Vec<u8>> = Cow::Owned(source);
            let error = StringBytes::try_borrow_str(&storage).unwrap_err();
            assert_eq!(error.valid_up_to(), valid_up_to);
            assert_eq!(error.error_len(), error_len);
            assert_eq!(StringBytes::as_bytes(&storage), input);
        }
    }

    #[test]
    fn borrow_str_unchecked_cow_vec_borrowed_borrows_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let source = input.to_vec();
            let storage: Cow<'_, Vec<u8>> = Cow::Borrowed(&source);
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
    fn borrow_str_unchecked_cow_vec_owned_borrows_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let storage: Cow<'_, Vec<u8>> = Cow::Owned(source);
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
    fn try_to_string_cow_vec_borrowed_copies_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let source = input.to_vec();
            let storage: Cow<'_, Vec<u8>> = Cow::Borrowed(&source);
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
    fn try_to_string_cow_vec_borrowed_reports_invalid_utf8_without_changing_storage() {
        for (input, valid_up_to, error_len) in [
            (b"ok\xff".as_slice(), 2, Some(1)),
            (b"ok\xe2\x82", 2, None),
            (b"\xc0\x80", 0, Some(1)),
            (b"\xed\xa0\x80", 0, Some(1)),
        ] {
            let source = input.to_vec();
            let storage: Cow<'_, Vec<u8>> = Cow::Borrowed(&source);
            let error = StringBytes::try_to_string(&storage).unwrap_err();
            assert_eq!(error.valid_up_to(), valid_up_to);
            assert_eq!(error.error_len(), error_len);
            assert_eq!(StringBytes::as_bytes(&storage), input);
        }
    }

    #[test]
    fn try_to_string_cow_vec_owned_copies_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let storage: Cow<'_, Vec<u8>> = Cow::Owned(source);
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
    fn try_to_string_cow_vec_owned_reports_invalid_utf8_without_changing_storage() {
        for (input, valid_up_to, error_len) in [
            (b"ok\xff".as_slice(), 2, Some(1)),
            (b"ok\xe2\x82", 2, None),
            (b"\xc0\x80", 0, Some(1)),
            (b"\xed\xa0\x80", 0, Some(1)),
        ] {
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let storage: Cow<'_, Vec<u8>> = Cow::Owned(source);
            let error = StringBytes::try_to_string(&storage).unwrap_err();
            assert_eq!(error.valid_up_to(), valid_up_to);
            assert_eq!(error.error_len(), error_len);
            assert_eq!(StringBytes::as_bytes(&storage), input);
        }
    }

    #[test]
    fn to_string_lossy_cow_vec_borrowed_borrows_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let source = input.to_vec();
            let storage: Cow<'_, Vec<u8>> = Cow::Borrowed(&source);
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
    fn to_string_lossy_cow_vec_borrowed_owns_replacement_text() {
        for (input, expected) in [
            (b"a\xffb".as_slice(), "a�b"),
            (b"ok\xe2\x82", "ok�"),
            (b"\xc0\x80", "��"),
            (b"\xed\xa0\x80", "���"),
        ] {
            let source = input.to_vec();
            let storage: Cow<'_, Vec<u8>> = Cow::Borrowed(&source);
            let value = StringBytes::to_string_lossy(&storage);
            assert!(matches!(value, Cow::Owned(_)));
            assert_eq!(value, expected);
            assert_eq!(StringBytes::as_bytes(&storage), input);
        }
    }

    #[test]
    fn to_string_lossy_cow_vec_owned_borrows_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let storage: Cow<'_, Vec<u8>> = Cow::Owned(source);
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
    fn to_string_lossy_cow_vec_owned_owns_replacement_text() {
        for (input, expected) in [
            (b"a\xffb".as_slice(), "a�b"),
            (b"ok\xe2\x82", "ok�"),
            (b"\xc0\x80", "��"),
            (b"\xed\xa0\x80", "���"),
        ] {
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let storage: Cow<'_, Vec<u8>> = Cow::Owned(source);
            let value = StringBytes::to_string_lossy(&storage);
            assert!(matches!(value, Cow::Owned(_)));
            assert_eq!(value, expected);
            assert_eq!(StringBytes::as_bytes(&storage), input);
        }
    }

    #[test]
    fn to_string_unchecked_cow_vec_borrowed_copies_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let source = input.to_vec();
            let storage: Cow<'_, Vec<u8>> = Cow::Borrowed(&source);
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
    fn to_string_unchecked_cow_vec_owned_copies_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let storage: Cow<'_, Vec<u8>> = Cow::Owned(source);
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
    fn try_into_string_cow_vec_borrowed_copies_storage() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let source = input.to_vec();
            let storage: Cow<'_, Vec<u8>> = Cow::Borrowed(&source);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let value = StringBytes::try_into_string(storage).unwrap();
            assert_eq!(value, expected);
            if !input.is_empty() {
                assert_ne!(value.as_ptr(), address);
            }
        }
    }

    #[test]
    fn try_into_string_cow_vec_borrowed_error_returns_original() {
        for (input, valid_up_to, error_len) in [
            (b"ok\xff".as_slice(), 2, Some(1)),
            (b"ok\xe2\x82", 2, None),
            (b"\xc0\x80", 0, Some(1)),
            (b"\xed\xa0\x80", 0, Some(1)),
        ] {
            let source = input.to_vec();
            let storage: Cow<'_, Vec<u8>> = Cow::Borrowed(&source);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let error = StringBytes::try_into_string(storage).unwrap_err();
            assert_eq!(error.utf8_error().valid_up_to(), valid_up_to);
            assert_eq!(error.utf8_error().error_len(), error_len);
            let bytes = error.into_inner();
            assert!(matches!(&bytes, Cow::Borrowed(_)));
            assert_eq!(StringBytes::as_bytes(&bytes), input);
            if !input.is_empty() {
                assert_eq!(StringBytes::as_bytes(&bytes).as_ptr(), address);
            }
        }
    }

    #[test]
    fn try_into_string_cow_vec_owned_reuses_buffer() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let source_capacity = source.capacity();
            let storage: Cow<'_, Vec<u8>> = Cow::Owned(source);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let capacity = source_capacity;
            let value = StringBytes::try_into_string(storage).unwrap();
            assert_eq!(value, expected);
            assert_eq!(value.capacity(), capacity);
            if capacity != 0 {
                assert_eq!(value.as_ptr(), address);
            }
        }
    }

    #[test]
    fn try_into_string_cow_vec_owned_error_keeps_buffer() {
        for (input, valid_up_to, error_len) in [
            (b"ok\xff".as_slice(), 2, Some(1)),
            (b"ok\xe2\x82", 2, None),
            (b"\xc0\x80", 0, Some(1)),
            (b"\xed\xa0\x80", 0, Some(1)),
        ] {
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let source_capacity = source.capacity();
            let storage: Cow<'_, Vec<u8>> = Cow::Owned(source);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let capacity = source_capacity;
            let error = StringBytes::try_into_string(storage).unwrap_err();
            assert_eq!(error.utf8_error().valid_up_to(), valid_up_to);
            assert_eq!(error.utf8_error().error_len(), error_len);
            let bytes = error.into_inner();
            assert!(matches!(&bytes, Cow::Owned(_)));
            assert_eq!(StringBytes::as_bytes(&bytes), input);
            assert_eq!(bytes.capacity(), capacity);
            if capacity != 0 {
                assert_eq!(StringBytes::as_bytes(&bytes).as_ptr(), address);
            }
        }
    }

    #[test]
    fn into_string_lossy_cow_vec_borrowed_valid_utf8_copies_storage() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let source = input.to_vec();
            let storage: Cow<'_, Vec<u8>> = Cow::Borrowed(&source);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let value = StringBytes::into_string_lossy(storage);
            assert_eq!(value, expected);
            if !input.is_empty() {
                assert_ne!(value.as_ptr(), address);
            }
        }
    }

    #[test]
    fn into_string_lossy_cow_vec_borrowed_replaces_invalid_utf8() {
        for (input, expected) in [
            (b"a\xffb".as_slice(), "a�b"),
            (b"ok\xe2\x82", "ok�"),
            (b"\xc0\x80", "��"),
            (b"\xed\xa0\x80", "���"),
        ] {
            let source = input.to_vec();
            let storage: Cow<'_, Vec<u8>> = Cow::Borrowed(&source);
            let value = StringBytes::into_string_lossy(storage);
            assert_eq!(value, expected);
        }
    }

    #[test]
    fn into_string_lossy_cow_vec_owned_valid_utf8_reuses_buffer() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let source_capacity = source.capacity();
            let storage: Cow<'_, Vec<u8>> = Cow::Owned(source);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let capacity = source_capacity;
            let value = StringBytes::into_string_lossy(storage);
            assert_eq!(value, expected);
            assert_eq!(value.capacity(), capacity);
            if capacity != 0 {
                assert_eq!(value.as_ptr(), address);
            }
        }
    }

    #[test]
    fn into_string_lossy_cow_vec_owned_replaces_invalid_utf8() {
        for (input, expected) in [
            (b"a\xffb".as_slice(), "a�b"),
            (b"ok\xe2\x82", "ok�"),
            (b"\xc0\x80", "��"),
            (b"\xed\xa0\x80", "���"),
        ] {
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let storage: Cow<'_, Vec<u8>> = Cow::Owned(source);
            let value = StringBytes::into_string_lossy(storage);
            assert_eq!(value, expected);
        }
    }

    #[test]
    fn into_string_unchecked_cow_vec_borrowed_copies_storage() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let source = input.to_vec();
            let storage: Cow<'_, Vec<u8>> = Cow::Borrowed(&source);
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
    fn into_string_unchecked_cow_vec_owned_reuses_buffer() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let source_capacity = source.capacity();
            let storage: Cow<'_, Vec<u8>> = Cow::Owned(source);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let capacity = source_capacity;
            // SAFETY: these test literals are valid UTF-8.
            let value = unsafe { StringBytes::into_string_unchecked(storage) };
            assert_eq!(value, expected);
            assert_eq!(value.capacity(), capacity);
            if capacity != 0 {
                assert_eq!(value.as_ptr(), address);
            }
        }
    }

    #[test]
    fn box_into_bytes_owned_cow_vec_reuses_inner_buffer() {
        let storage: Box<Cow<'static, Vec<u8>>> =
            Box::new(Cow::Owned(Vec::from(b"text".as_slice())));
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let capacity = 4;
        let value = StringBytes::box_into_bytes(storage);
        assert_eq!(value, b"text");
        if capacity != 0 {
            assert_eq!(value.as_ptr(), address);
        }
        assert_eq!(value.capacity(), capacity);
    }

    #[test]
    fn rc_into_bytes_owned_cow_vec_reuses_inner_buffer() {
        let storage: Rc<Cow<'static, Vec<u8>>> = Rc::new(Cow::Owned(Vec::from(b"text".as_slice())));
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let capacity = 4;
        let value = StringBytes::rc_into_bytes(storage);
        assert_eq!(value, b"text");
        if capacity != 0 {
            assert_eq!(value.as_ptr(), address);
        }
        assert_eq!(value.capacity(), capacity);
    }

    #[test]
    fn arc_into_bytes_owned_cow_vec_reuses_inner_buffer() {
        let storage: Arc<Cow<'static, Vec<u8>>> =
            Arc::new(Cow::Owned(Vec::from(b"text".as_slice())));
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let capacity = 4;
        let value = StringBytes::arc_into_bytes(storage);
        assert_eq!(value, b"text");
        if capacity != 0 {
            assert_eq!(value.as_ptr(), address);
        }
        assert_eq!(value.capacity(), capacity);
    }

    #[test]
    fn box_into_string_owned_cow_vec_reuses_inner_buffer() {
        let storage: Box<Cow<'static, Vec<u8>>> =
            Box::new(Cow::Owned(Vec::from(b"text".as_slice())));
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let capacity = 4;
        let value = StringBytes::box_into_string(storage).unwrap();
        assert_eq!(value, "text");
        if capacity != 0 {
            assert_eq!(value.as_ptr(), address);
        }
        assert_eq!(value.capacity(), capacity);
    }

    #[test]
    fn rc_into_string_owned_cow_vec_reuses_inner_buffer() {
        let storage: Rc<Cow<'static, Vec<u8>>> = Rc::new(Cow::Owned(Vec::from(b"text".as_slice())));
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let capacity = 4;
        let value = StringBytes::rc_into_string(storage).unwrap();
        assert_eq!(value, "text");
        if capacity != 0 {
            assert_eq!(value.as_ptr(), address);
        }
        assert_eq!(value.capacity(), capacity);
    }

    #[test]
    fn arc_into_string_owned_cow_vec_reuses_inner_buffer() {
        let storage: Arc<Cow<'static, Vec<u8>>> =
            Arc::new(Cow::Owned(Vec::from(b"text".as_slice())));
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let capacity = 4;
        let value = StringBytes::arc_into_string(storage).unwrap();
        assert_eq!(value, "text");
        if capacity != 0 {
            assert_eq!(value.as_ptr(), address);
        }
        assert_eq!(value.capacity(), capacity);
    }
}
