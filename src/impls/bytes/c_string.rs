use std::ffi::CString;
use std::rc::Rc;
use std::string::FromUtf8Error;
use std::sync::Arc;

use crate::impls::{impl_pointer_conversions_for_sized, impl_string_bytes};
use crate::{sealed, StringBytes};

impl_string_bytes!(CString {
    /// Returns the bytes without the trailing NUL, as in [`CString::as_bytes`].
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        CString::as_bytes(self)
    }

    /// Converts the contents into bytes, removing the trailing NUL and reusing
    /// the allocation.
    #[inline]
    fn into_bytes(self) -> Vec<u8> {
        CString::into_bytes(self)
    }

    impl_pointer_conversions_for_sized!();
});

#[cfg(test)]
mod tests {
    use std::borrow::Cow;
    use std::ffi::CString;
    use std::rc::Rc;
    use std::sync::Arc;

    use crate::StringBytes;

    #[test]
    fn as_bytes_c_string_exposes_stable_contents() {
        for input in [
            b"".as_slice(),
            b"text",
            "é说明".as_bytes(),
            b"e\xcc\x81",
            b"ok\xff",
            b"ok\xe2\x82",
            b"\xc0\x80",
            b"\xed\xa0\x80",
        ] {
            let storage = CString::new(input).unwrap();
            assert_eq!(StringBytes::as_bytes(&storage), input);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            assert_eq!(StringBytes::as_bytes(&storage), input);
            assert_eq!(StringBytes::as_bytes(&storage).as_ptr(), address);
        }
    }

    #[test]
    fn as_bytes_c_string_matches_deref() {
        for input in [
            b"".as_slice(),
            b"text",
            "é说明".as_bytes(),
            b"e\xcc\x81",
            b"ok\xff",
            b"ok\xe2\x82",
            b"\xc0\x80",
            b"\xed\xa0\x80",
        ] {
            let storage = CString::new(input).unwrap();
            let bytes = StringBytes::as_bytes(&storage);
            let other = StringBytes::as_bytes(std::ops::Deref::deref(&storage));
            assert_eq!(bytes, input);
            assert_eq!(other, input);
            assert!(std::ptr::eq(bytes, other));
        }
    }

    #[test]
    fn to_bytes_c_string_returns_an_independent_copy() {
        for input in [
            b"".as_slice(),
            b"text",
            "é说明".as_bytes(),
            b"e\xcc\x81",
            b"ok\xff",
            b"ok\xe2\x82",
            b"\xc0\x80",
            b"\xed\xa0\x80",
        ] {
            let storage = CString::new(input).unwrap();
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
    fn into_bytes_c_string_reuses_buffer() {
        for input in [
            b"".as_slice(),
            b"text",
            "é说明".as_bytes(),
            b"e\xcc\x81",
            b"ok\xff",
            b"ok\xe2\x82",
            b"\xc0\x80",
            b"\xed\xa0\x80",
        ] {
            let storage = CString::new(input).unwrap();
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let capacity = storage.as_bytes_with_nul().len();
            let bytes = StringBytes::into_bytes(storage);
            assert_eq!(bytes, input);
            assert_eq!(bytes.capacity(), capacity);
            if capacity != 0 {
                assert_eq!(bytes.as_ptr(), address);
            }
        }
    }

    #[test]
    fn try_borrow_str_c_string_borrows_valid_utf8() {
        for expected in ["", "text", "é说明", "e\u{301}"] {
            let input = expected.as_bytes();
            let storage = CString::new(input).unwrap();
            let value = StringBytes::try_borrow_str(&storage).unwrap();
            assert_eq!(value, expected);
            assert!(std::ptr::eq(
                value.as_bytes(),
                StringBytes::as_bytes(&storage)
            ));
        }
    }

    #[test]
    fn try_borrow_str_c_string_reports_invalid_utf8() {
        for (input, valid_up_to, error_len) in [
            (b"ok\xff".as_slice(), 2, Some(1)),
            (b"ok\xe2\x82", 2, None),
            (b"\xc0\x80", 0, Some(1)),
            (b"\xed\xa0\x80", 0, Some(1)),
        ] {
            let storage = CString::new(input).unwrap();
            let error = StringBytes::try_borrow_str(&storage).unwrap_err();
            assert_eq!(error.valid_up_to(), valid_up_to);
            assert_eq!(error.error_len(), error_len);
            assert_eq!(StringBytes::as_bytes(&storage), input);
        }
    }

    #[test]
    fn borrow_str_unchecked_c_string_borrows_valid_utf8() {
        for expected in ["", "text", "é说明", "e\u{301}"] {
            let input = expected.as_bytes();
            let storage = CString::new(input).unwrap();
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
    fn try_to_string_c_string_copies_valid_utf8() {
        for expected in ["", "text", "é说明", "e\u{301}"] {
            let input = expected.as_bytes();
            let storage = CString::new(input).unwrap();
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
    fn try_to_string_c_string_returns_a_copy_of_invalid_bytes() {
        for (input, valid_up_to, error_len) in [
            (b"ok\xff".as_slice(), 2, Some(1)),
            (b"ok\xe2\x82", 2, None),
            (b"\xc0\x80", 0, Some(1)),
            (b"\xed\xa0\x80", 0, Some(1)),
        ] {
            let storage = CString::new(input).unwrap();
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
    fn to_string_lossy_c_string_borrows_valid_utf8() {
        for expected in ["", "text", "é说明", "e\u{301}"] {
            let input = expected.as_bytes();
            let storage = CString::new(input).unwrap();
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
    fn to_string_lossy_c_string_owns_replacement_text() {
        for (input, expected) in [
            (b"a\xffb".as_slice(), "a�b"),
            (b"ok\xe2\x82", "ok�"),
            (b"\xc0\x80", "��"),
            (b"\xed\xa0\x80", "���"),
        ] {
            let storage = CString::new(input).unwrap();
            let value = StringBytes::to_string_lossy(&storage);
            assert!(matches!(value, Cow::Owned(_)));
            assert_eq!(value, expected);
            assert_eq!(StringBytes::as_bytes(&storage), input);
        }
    }

    #[test]
    fn to_string_unchecked_c_string_copies_valid_utf8() {
        for expected in ["", "text", "é说明", "e\u{301}"] {
            let input = expected.as_bytes();
            let storage = CString::new(input).unwrap();
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
    fn try_into_string_c_string_reuses_buffer() {
        for expected in ["", "text", "é说明", "e\u{301}"] {
            let input = expected.as_bytes();
            let storage = CString::new(input).unwrap();
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let capacity = storage.as_bytes_with_nul().len();
            let value = StringBytes::try_into_string(storage).unwrap();
            assert_eq!(value, expected);
            assert_eq!(value.capacity(), capacity);
            if capacity != 0 {
                assert_eq!(value.as_ptr(), address);
            }
        }
    }

    #[test]
    fn try_into_string_c_string_error_reuses_buffer() {
        for (input, valid_up_to, error_len) in [
            (b"ok\xff".as_slice(), 2, Some(1)),
            (b"ok\xe2\x82", 2, None),
            (b"\xc0\x80", 0, Some(1)),
            (b"\xed\xa0\x80", 0, Some(1)),
        ] {
            let storage = CString::new(input).unwrap();
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let capacity = storage.as_bytes_with_nul().len();
            let error = StringBytes::try_into_string(storage).unwrap_err();
            assert_eq!(error.utf8_error().valid_up_to(), valid_up_to);
            assert_eq!(error.utf8_error().error_len(), error_len);
            let bytes = error.into_bytes();
            assert_eq!(bytes, input);
            assert_eq!(bytes.capacity(), capacity);
            if capacity != 0 {
                assert_eq!(bytes.as_ptr(), address);
            }
        }
    }

    #[test]
    fn into_string_lossy_c_string_valid_utf8_reuses_buffer() {
        for expected in ["", "text", "é说明", "e\u{301}"] {
            let input = expected.as_bytes();
            let storage = CString::new(input).unwrap();
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let capacity = storage.as_bytes_with_nul().len();
            let value = StringBytes::into_string_lossy(storage);
            assert_eq!(value, expected);
            assert_eq!(value.capacity(), capacity);
            if capacity != 0 {
                assert_eq!(value.as_ptr(), address);
            }
        }
    }

    #[test]
    fn into_string_lossy_c_string_replaces_invalid_utf8() {
        for (input, expected) in [
            (b"a\xffb".as_slice(), "a�b"),
            (b"ok\xe2\x82", "ok�"),
            (b"\xc0\x80", "��"),
            (b"\xed\xa0\x80", "���"),
        ] {
            let storage = CString::new(input).unwrap();
            let value = StringBytes::into_string_lossy(storage);
            assert_eq!(value, expected);
        }
    }

    #[test]
    fn into_string_unchecked_c_string_reuses_buffer() {
        for expected in ["", "text", "é说明", "e\u{301}"] {
            let input = expected.as_bytes();
            let storage = CString::new(input).unwrap();
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let capacity = storage.as_bytes_with_nul().len();
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
    fn box_into_bytes_c_string_reuses_inner_buffer() {
        let storage: Box<CString> = Box::new(CString::new("text").unwrap());
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let capacity = 5;
        let value = StringBytes::box_into_bytes(storage);
        assert_eq!(value, b"text");
        if capacity != 0 {
            assert_eq!(value.as_ptr(), address);
        }
        assert_eq!(value.capacity(), capacity);
    }

    #[test]
    fn rc_into_bytes_c_string_reuses_inner_buffer() {
        let storage: Rc<CString> = Rc::new(CString::new("text").unwrap());
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let capacity = 5;
        let value = StringBytes::rc_into_bytes(storage);
        assert_eq!(value, b"text");
        if capacity != 0 {
            assert_eq!(value.as_ptr(), address);
        }
        assert_eq!(value.capacity(), capacity);
    }

    #[test]
    fn arc_into_bytes_c_string_reuses_inner_buffer() {
        let storage: Arc<CString> = Arc::new(CString::new("text").unwrap());
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let capacity = 5;
        let value = StringBytes::arc_into_bytes(storage);
        assert_eq!(value, b"text");
        if capacity != 0 {
            assert_eq!(value.as_ptr(), address);
        }
        assert_eq!(value.capacity(), capacity);
    }

    #[test]
    fn box_into_string_c_string_reuses_inner_buffer() {
        let storage: Box<CString> = Box::new(CString::new("text").unwrap());
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let capacity = 5;
        let value = StringBytes::box_into_string(storage).unwrap();
        assert_eq!(value, "text");
        if capacity != 0 {
            assert_eq!(value.as_ptr(), address);
        }
        assert_eq!(value.capacity(), capacity);
    }

    #[test]
    fn rc_into_string_c_string_reuses_inner_buffer() {
        let storage: Rc<CString> = Rc::new(CString::new("text").unwrap());
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let capacity = 5;
        let value = StringBytes::rc_into_string(storage).unwrap();
        assert_eq!(value, "text");
        if capacity != 0 {
            assert_eq!(value.as_ptr(), address);
        }
        assert_eq!(value.capacity(), capacity);
    }

    #[test]
    fn arc_into_string_c_string_reuses_inner_buffer() {
        let storage: Arc<CString> = Arc::new(CString::new("text").unwrap());
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let capacity = 5;
        let value = StringBytes::arc_into_string(storage).unwrap();
        assert_eq!(value, "text");
        if capacity != 0 {
            assert_eq!(value.as_ptr(), address);
        }
        assert_eq!(value.capacity(), capacity);
    }
}
