use crate::impls::impl_string_bytes;
use crate::{sealed, StringBytes};

impl_string_bytes!(
    /// Borrows the bytes of a slice.
    ///
    /// [`into_bytes`](StringBytes::into_bytes) reuses the allocation of a
    /// [`Box<[u8]>`](Box). References, [`Rc<[u8]>`](std::rc::Rc), and
    /// [`Arc<[u8]>`](std::sync::Arc) copy the bytes when consumed.
    ///
    /// Checked and lossy string conversions validate UTF-8.
    [u8] {
        /// Returns this byte slice.
        #[inline]
        fn as_bytes(&self) -> &[u8] {
            self
        }

        /// Converts the boxed slice into a vector, reusing its allocation.
        #[inline]
        fn box_into_bytes(self: Box<Self>) -> Vec<u8> {
            // Use the slice's inherent method to transfer the allocation.
            <[u8]>::into_vec(self)
        }
    }
);

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use crate::StringBytes;

    #[test]
    fn as_bytes_slice_exposes_stable_contents() {
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
            let storage: &[u8] = input;
            assert_eq!(StringBytes::as_bytes(storage), input);
            let address = StringBytes::as_bytes(storage).as_ptr();
            assert_eq!(StringBytes::as_bytes(storage), input);
            assert_eq!(StringBytes::as_bytes(storage).as_ptr(), address);
        }
    }

    #[test]
    fn as_bytes_slice_matches_as_ref() {
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
            let storage: &[u8] = input;
            let bytes = StringBytes::as_bytes(storage);
            let other = AsRef::<[u8]>::as_ref(storage);
            assert_eq!(bytes, input);
            assert_eq!(other, input);
            assert!(std::ptr::eq(bytes, other));
        }
    }

    #[test]
    fn as_bytes_slice_reflects_as_mut_edits() {
        let mut source = *b"text";
        let storage = &mut source[..];
        let address = storage.as_ptr();
        {
            let bytes = AsMut::<[u8]>::as_mut(storage);
            assert_eq!(bytes, b"text");
            assert_eq!(bytes.as_ptr(), address);
            bytes[0] = b'n';
        }
        assert_eq!(StringBytes::as_bytes(storage), b"next");
        assert_eq!(StringBytes::as_bytes(storage).as_ptr(), address);
    }

    #[test]
    fn to_bytes_slice_returns_an_independent_copy() {
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
            let storage: &[u8] = input;
            let mut bytes = StringBytes::to_bytes(storage);
            assert_eq!(bytes, input);
            if !bytes.is_empty() {
                assert_ne!(bytes.as_ptr(), StringBytes::as_bytes(storage).as_ptr());
                bytes[0] ^= 0xff;
            }
            assert_eq!(StringBytes::as_bytes(storage), input);
        }
    }

    #[test]
    fn try_borrow_str_slice_borrows_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let storage: &[u8] = input;
            let value = StringBytes::try_borrow_str(storage).unwrap();
            assert_eq!(value, expected);
            assert!(std::ptr::eq(
                value.as_bytes(),
                StringBytes::as_bytes(storage)
            ));
        }
    }

    #[test]
    fn try_borrow_str_slice_reports_invalid_utf8() {
        for (input, valid_up_to, error_len) in [
            (b"ok\xff".as_slice(), 2, Some(1)),
            (b"ok\xe2\x82", 2, None),
            (b"\xc0\x80", 0, Some(1)),
            (b"\xed\xa0\x80", 0, Some(1)),
        ] {
            let storage: &[u8] = input;
            let error = StringBytes::try_borrow_str(storage).unwrap_err();
            assert_eq!(error.valid_up_to(), valid_up_to);
            assert_eq!(error.error_len(), error_len);
            assert_eq!(StringBytes::as_bytes(storage), input);
        }
    }

    #[test]
    fn borrow_str_unchecked_slice_borrows_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let storage: &[u8] = input;
            // SAFETY: these test literals are valid UTF-8.
            let value = unsafe { StringBytes::borrow_str_unchecked(storage) };
            assert_eq!(value, expected);
            assert!(std::ptr::eq(
                value.as_bytes(),
                StringBytes::as_bytes(storage)
            ));
        }
    }

    #[test]
    fn try_to_string_slice_copies_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let storage: &[u8] = input;
            let mut value = StringBytes::try_to_string(storage).unwrap();
            assert_eq!(value, expected);
            if !value.is_empty() {
                assert_ne!(value.as_ptr(), StringBytes::as_bytes(storage).as_ptr());
            }
            value.make_ascii_uppercase();
            assert_eq!(StringBytes::as_bytes(storage), input);
        }
    }

    #[test]
    fn try_to_string_slice_returns_a_copy_of_invalid_bytes() {
        for (input, valid_up_to, error_len) in [
            (b"ok\xff".as_slice(), 2, Some(1)),
            (b"ok\xe2\x82", 2, None),
            (b"\xc0\x80", 0, Some(1)),
            (b"\xed\xa0\x80", 0, Some(1)),
        ] {
            let storage: &[u8] = input;
            let error = StringBytes::try_to_string(storage).unwrap_err();
            assert_eq!(error.utf8_error().valid_up_to(), valid_up_to);
            assert_eq!(error.utf8_error().error_len(), error_len);
            assert_eq!(error.as_bytes(), input);
            assert_ne!(
                error.as_bytes().as_ptr(),
                StringBytes::as_bytes(storage).as_ptr()
            );
            assert_eq!(StringBytes::as_bytes(storage), input);
        }
    }

    #[test]
    fn to_string_lossy_slice_borrows_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let storage: &[u8] = input;
            let value = StringBytes::to_string_lossy(storage);
            assert!(matches!(value, Cow::Borrowed(_)));
            assert_eq!(value, expected);
            // Empty buffers may borrow the standard library's static empty string.
            if !value.is_empty() {
                assert!(std::ptr::eq(
                    value.as_bytes(),
                    StringBytes::as_bytes(storage)
                ));
            }
        }
    }

    #[test]
    fn to_string_lossy_slice_owns_replacement_text() {
        for (input, expected) in [
            (b"a\xffb".as_slice(), "a�b"),
            (b"ok\xe2\x82", "ok�"),
            (b"\xc0\x80", "��"),
            (b"\xed\xa0\x80", "���"),
        ] {
            let storage: &[u8] = input;
            let value = StringBytes::to_string_lossy(storage);
            assert!(matches!(value, Cow::Owned(_)));
            assert_eq!(value, expected);
            assert_eq!(StringBytes::as_bytes(storage), input);
        }
    }

    #[test]
    fn to_string_unchecked_slice_copies_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let storage: &[u8] = input;
            // SAFETY: these test literals are valid UTF-8.
            let mut value = unsafe { StringBytes::to_string_unchecked(storage) };
            assert_eq!(value, expected);
            if !value.is_empty() {
                assert_ne!(value.as_ptr(), StringBytes::as_bytes(storage).as_ptr());
            }
            value.make_ascii_uppercase();
            assert_eq!(StringBytes::as_bytes(storage), input);
        }
    }

    #[test]
    fn box_into_bytes_slice_reuses_allocation() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let storage: Box<[u8]> = Box::<[u8]>::from(input);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let capacity = input.len();
            let value = <[u8] as StringBytes>::box_into_bytes(storage);
            assert_eq!(value, expected.as_bytes());
            assert_eq!(value.capacity(), capacity);
            if capacity != 0 {
                assert_eq!(value.as_ptr(), address);
            }
        }
    }

    #[test]
    fn box_into_string_slice_reuses_allocation() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let storage: Box<[u8]> = Box::<[u8]>::from(input);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let capacity = input.len();
            let value = <[u8] as StringBytes>::box_into_string(storage).unwrap();
            assert_eq!(value, expected);
            assert_eq!(value.capacity(), capacity);
            if capacity != 0 {
                assert_eq!(value.as_ptr(), address);
            }
        }
    }

    #[test]
    fn box_into_string_slice_error_keeps_allocation() {
        let storage = Box::<[u8]>::from(b"ok\xff".as_slice());
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let capacity = 3;
        let error = StringBytes::box_into_string(storage).unwrap_err();
        assert_eq!(error.utf8_error().valid_up_to(), 2);
        assert_eq!(error.utf8_error().error_len(), Some(1));
        let value = error.into_bytes();
        assert_eq!(value, b"ok\xff");
        if capacity != 0 {
            assert_eq!(value.as_ptr(), address);
        }
        assert_eq!(value.capacity(), capacity);
    }
}
