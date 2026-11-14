use std::rc::Rc;
use std::sync::Arc;

use crate::impls::{impl_pointer_conversions_for_sized, impl_string_bytes};
use crate::{sealed, StringBytes};

impl_string_bytes!(
    /// Uses the vector's initialized bytes, excluding spare capacity.
    ///
    /// [`into_bytes`](StringBytes::into_bytes) returns the vector unchanged.
    /// [`try_into_string`](StringBytes::try_into_string) validates UTF-8 and
    /// keeps the buffer and capacity, including when conversion returns an error.
    /// Checked borrowing and copying conversions also validate UTF-8.
    Vec<u8> {
        /// Borrows the initialized bytes of the vector.
        #[inline]
        fn as_bytes(&self) -> &[u8] {
            self.as_slice()
        }

        /// Returns this vector unchanged, preserving its buffer and capacity.
        #[inline]
        fn into_bytes(self) -> Vec<u8> {
            self
        }

        impl_pointer_conversions_for_sized!();
    }
);

#[cfg(test)]
mod tests {
    use std::borrow::Cow;
    use std::rc::Rc;
    use std::sync::Arc;

    use crate::StringBytes;

    #[test]
    fn as_bytes_vec_exposes_stable_contents() {
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
            let mut storage = Vec::with_capacity(input.len() + 32);
            storage.extend_from_slice(input);
            assert_eq!(StringBytes::as_bytes(&storage), input);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            assert_eq!(StringBytes::as_bytes(&storage), input);
            assert_eq!(StringBytes::as_bytes(&storage).as_ptr(), address);
        }
    }

    #[test]
    fn as_bytes_vec_matches_as_ref() {
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
            let mut storage = Vec::with_capacity(input.len() + 32);
            storage.extend_from_slice(input);
            let bytes = StringBytes::as_bytes(&storage);
            let other = AsRef::<[u8]>::as_ref(&storage);
            assert_eq!(bytes, input);
            assert_eq!(other, input);
            assert!(std::ptr::eq(bytes, other));
        }
    }

    #[test]
    fn as_bytes_vec_matches_deref() {
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
            let mut storage = Vec::with_capacity(input.len() + 32);
            storage.extend_from_slice(input);
            let bytes = StringBytes::as_bytes(&storage);
            let other = StringBytes::as_bytes(std::ops::Deref::deref(&storage));
            assert_eq!(bytes, input);
            assert_eq!(other, input);
            assert!(std::ptr::eq(bytes, other));
        }
    }

    #[test]
    fn as_bytes_vec_reflects_as_mut_edits() {
        let input = *b"text";
        let mut storage = Vec::with_capacity(input.len() + 32);
        storage.extend_from_slice(&input);
        let address = StringBytes::as_bytes(&storage).as_ptr();
        {
            let bytes = AsMut::<[u8]>::as_mut(&mut storage);
            assert_eq!(bytes, b"text");
            assert_eq!(bytes.as_ptr(), address);
            bytes[0] = b'n';
        }
        assert_eq!(StringBytes::as_bytes(&storage), b"next");
        assert_eq!(StringBytes::as_bytes(&storage).as_ptr(), address);
    }

    #[test]
    fn to_bytes_vec_returns_an_independent_copy() {
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
            let mut storage = Vec::with_capacity(input.len() + 32);
            storage.extend_from_slice(input);
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
    fn into_bytes_vec_reuses_buffer() {
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
            let mut storage = Vec::with_capacity(input.len() + 32);
            storage.extend_from_slice(input);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let capacity = storage.capacity();
            let bytes = StringBytes::into_bytes(storage);
            assert_eq!(bytes, input);
            assert_eq!(bytes.capacity(), capacity);
            if capacity != 0 {
                assert_eq!(bytes.as_ptr(), address);
            }
        }
    }

    #[test]
    fn into_bytes_empty_vec_without_capacity() {
        let storage = Vec::<u8>::new();
        let capacity = 0;
        let value = StringBytes::into_bytes(storage);
        assert_eq!(value, b"");
        assert_eq!(value.capacity(), capacity);
    }

    #[test]
    fn try_borrow_str_vec_borrows_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut storage = Vec::with_capacity(input.len() + 32);
            storage.extend_from_slice(input);
            let value = StringBytes::try_borrow_str(&storage).unwrap();
            assert_eq!(value, expected);
            assert!(std::ptr::eq(
                value.as_bytes(),
                StringBytes::as_bytes(&storage)
            ));
        }
    }

    #[test]
    fn try_borrow_str_vec_reports_invalid_utf8() {
        for (input, valid_up_to, error_len) in [
            (b"ok\xff".as_slice(), 2, Some(1)),
            (b"ok\xe2\x82", 2, None),
            (b"\xc0\x80", 0, Some(1)),
            (b"\xed\xa0\x80", 0, Some(1)),
        ] {
            let mut storage = Vec::with_capacity(input.len() + 32);
            storage.extend_from_slice(input);
            let error = StringBytes::try_borrow_str(&storage).unwrap_err();
            assert_eq!(error.valid_up_to(), valid_up_to);
            assert_eq!(error.error_len(), error_len);
            assert_eq!(StringBytes::as_bytes(&storage), input);
        }
    }

    #[test]
    fn borrow_str_unchecked_vec_borrows_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut storage = Vec::with_capacity(input.len() + 32);
            storage.extend_from_slice(input);
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
    fn try_to_string_vec_copies_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut storage = Vec::with_capacity(input.len() + 32);
            storage.extend_from_slice(input);
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
    fn try_to_string_vec_reports_invalid_utf8_without_changing_storage() {
        for (input, valid_up_to, error_len) in [
            (b"ok\xff".as_slice(), 2, Some(1)),
            (b"ok\xe2\x82", 2, None),
            (b"\xc0\x80", 0, Some(1)),
            (b"\xed\xa0\x80", 0, Some(1)),
        ] {
            let mut storage = Vec::with_capacity(input.len() + 32);
            storage.extend_from_slice(input);
            let error = StringBytes::try_to_string(&storage).unwrap_err();
            assert_eq!(error.valid_up_to(), valid_up_to);
            assert_eq!(error.error_len(), error_len);
            assert_eq!(StringBytes::as_bytes(&storage), input);
        }
    }

    #[test]
    fn to_string_lossy_vec_borrows_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut storage = Vec::with_capacity(input.len() + 32);
            storage.extend_from_slice(input);
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
    fn to_string_lossy_vec_owns_replacement_text() {
        for (input, expected) in [
            (b"a\xffb".as_slice(), "a�b"),
            (b"ok\xe2\x82", "ok�"),
            (b"\xc0\x80", "��"),
            (b"\xed\xa0\x80", "���"),
        ] {
            let mut storage = Vec::with_capacity(input.len() + 32);
            storage.extend_from_slice(input);
            let value = StringBytes::to_string_lossy(&storage);
            assert!(matches!(value, Cow::Owned(_)));
            assert_eq!(value, expected);
            assert_eq!(StringBytes::as_bytes(&storage), input);
        }
    }

    #[test]
    fn to_string_unchecked_vec_copies_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut storage = Vec::with_capacity(input.len() + 32);
            storage.extend_from_slice(input);
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
    fn try_into_string_vec_reuses_buffer() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut storage = Vec::with_capacity(input.len() + 32);
            storage.extend_from_slice(input);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let capacity = storage.capacity();
            let value = StringBytes::try_into_string(storage).unwrap();
            assert_eq!(value, expected);
            assert_eq!(value.capacity(), capacity);
            if capacity != 0 {
                assert_eq!(value.as_ptr(), address);
            }
        }
    }

    #[test]
    fn try_into_string_vec_error_keeps_buffer() {
        for (input, valid_up_to, error_len) in [
            (b"ok\xff".as_slice(), 2, Some(1)),
            (b"ok\xe2\x82", 2, None),
            (b"\xc0\x80", 0, Some(1)),
            (b"\xed\xa0\x80", 0, Some(1)),
        ] {
            let mut storage = Vec::with_capacity(input.len() + 32);
            storage.extend_from_slice(input);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let capacity = storage.capacity();
            let error = StringBytes::try_into_string(storage).unwrap_err();
            assert_eq!(error.utf8_error().valid_up_to(), valid_up_to);
            assert_eq!(error.utf8_error().error_len(), error_len);
            let bytes = error.into_inner();
            assert_eq!(StringBytes::as_bytes(&bytes), input);
            assert_eq!(bytes.capacity(), capacity);
            if capacity != 0 {
                assert_eq!(StringBytes::as_bytes(&bytes).as_ptr(), address);
            }
        }
    }

    #[test]
    fn try_into_string_empty_vec_without_capacity() {
        let storage = Vec::<u8>::new();
        let capacity = 0;
        let value = StringBytes::try_into_string(storage).unwrap();
        assert_eq!(value, "");
        assert_eq!(value.capacity(), capacity);
    }

    #[test]
    fn into_string_lossy_vec_valid_utf8_reuses_buffer() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut storage = Vec::with_capacity(input.len() + 32);
            storage.extend_from_slice(input);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let capacity = storage.capacity();
            let value = StringBytes::into_string_lossy(storage);
            assert_eq!(value, expected);
            assert_eq!(value.capacity(), capacity);
            if capacity != 0 {
                assert_eq!(value.as_ptr(), address);
            }
        }
    }

    #[test]
    fn into_string_lossy_vec_replaces_invalid_utf8() {
        for (input, expected) in [
            (b"a\xffb".as_slice(), "a�b"),
            (b"ok\xe2\x82", "ok�"),
            (b"\xc0\x80", "��"),
            (b"\xed\xa0\x80", "���"),
        ] {
            let mut storage = Vec::with_capacity(input.len() + 32);
            storage.extend_from_slice(input);
            let value = StringBytes::into_string_lossy(storage);
            assert_eq!(value, expected);
        }
    }

    #[test]
    fn into_string_lossy_empty_vec_without_capacity() {
        let storage = Vec::<u8>::new();
        let capacity = 0;
        let value = StringBytes::into_string_lossy(storage);
        assert_eq!(value, "");
        assert_eq!(value.capacity(), capacity);
    }

    #[test]
    fn into_string_unchecked_vec_reuses_buffer() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut storage = Vec::with_capacity(input.len() + 32);
            storage.extend_from_slice(input);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let capacity = storage.capacity();
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
    fn into_string_unchecked_empty_vec_without_capacity() {
        let storage = Vec::<u8>::new();
        let capacity = 0;
        // SAFETY: the storage contains the valid UTF-8 literal above.
        let value = unsafe { StringBytes::into_string_unchecked(storage) };
        assert_eq!(value, "");
        assert_eq!(value.capacity(), capacity);
    }

    #[test]
    fn box_into_bytes_vec_reuses_inner_buffer() {
        let storage: Box<Vec<u8>> = Box::new(vec![b't', b'e', b'x', b't']);
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
    fn rc_into_bytes_vec_reuses_inner_buffer() {
        let storage: Rc<Vec<u8>> = Rc::new(vec![b't', b'e', b'x', b't']);
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
    fn arc_into_bytes_vec_reuses_inner_buffer() {
        let storage: Arc<Vec<u8>> = Arc::new(vec![b't', b'e', b'x', b't']);
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
    fn box_into_string_vec_reuses_inner_buffer() {
        let storage: Box<Vec<u8>> = Box::new(vec![b't', b'e', b'x', b't']);
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
    fn rc_into_string_vec_reuses_inner_buffer() {
        let storage: Rc<Vec<u8>> = Rc::new(vec![b't', b'e', b'x', b't']);
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
    fn arc_into_string_vec_reuses_inner_buffer() {
        let storage: Arc<Vec<u8>> = Arc::new(vec![b't', b'e', b'x', b't']);
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
