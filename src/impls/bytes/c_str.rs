use std::ffi::CStr;

use crate::impls::impl_string_bytes;
use crate::{sealed, StringBytes};

// Exclude the trailing NUL, matching CStr::to_bytes. C strings need not be
// UTF-8, so the default string conversions still validate their contents.
impl_string_bytes!(CStr {
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        CStr::to_bytes(self)
    }

    #[inline]
    fn box_into_bytes(self: Box<Self>) -> Vec<u8> {
        // Both conversions transfer the allocation. Removing the terminator
        // changes the length, leaving its byte available as spare capacity.
        self.into_c_string().into_bytes()
    }
});

#[cfg(test)]
mod tests {
    use std::borrow::Cow;
    use std::ffi::CStr;
    use std::ffi::CString;

    use crate::StringBytes;

    #[test]
    fn as_bytes_c_str_exposes_stable_contents() {
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
            let source = CString::new(input).unwrap();
            let storage = source.as_c_str();
            assert_eq!(StringBytes::as_bytes(storage), input);
            let address = StringBytes::as_bytes(storage).as_ptr();
            assert_eq!(StringBytes::as_bytes(storage), input);
            assert_eq!(StringBytes::as_bytes(storage).as_ptr(), address);
        }
    }

    #[test]
    fn to_bytes_c_str_returns_an_independent_copy() {
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
            let source = CString::new(input).unwrap();
            let storage = source.as_c_str();
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
    fn try_borrow_str_c_str_borrows_valid_utf8() {
        for expected in ["", "text", "é说明", "e\u{301}"] {
            let input = expected.as_bytes();
            let source = CString::new(input).unwrap();
            let storage = source.as_c_str();
            let value = StringBytes::try_borrow_str(storage).unwrap();
            assert_eq!(value, expected);
            assert!(std::ptr::eq(
                value.as_bytes(),
                StringBytes::as_bytes(storage)
            ));
        }
    }

    #[test]
    fn try_borrow_str_c_str_reports_invalid_utf8() {
        for (input, valid_up_to, error_len) in [
            (b"ok\xff".as_slice(), 2, Some(1)),
            (b"ok\xe2\x82", 2, None),
            (b"\xc0\x80", 0, Some(1)),
            (b"\xed\xa0\x80", 0, Some(1)),
        ] {
            let source = CString::new(input).unwrap();
            let storage = source.as_c_str();
            let error = StringBytes::try_borrow_str(storage).unwrap_err();
            assert_eq!(error.valid_up_to(), valid_up_to);
            assert_eq!(error.error_len(), error_len);
            assert_eq!(StringBytes::as_bytes(storage), input);
        }
    }

    #[test]
    fn borrow_str_unchecked_c_str_borrows_valid_utf8() {
        for expected in ["", "text", "é说明", "e\u{301}"] {
            let input = expected.as_bytes();
            let source = CString::new(input).unwrap();
            let storage = source.as_c_str();
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
    fn try_to_string_c_str_copies_valid_utf8() {
        for expected in ["", "text", "é说明", "e\u{301}"] {
            let input = expected.as_bytes();
            let source = CString::new(input).unwrap();
            let storage = source.as_c_str();
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
    fn try_to_string_c_str_returns_a_copy_of_invalid_bytes() {
        for (input, valid_up_to, error_len) in [
            (b"ok\xff".as_slice(), 2, Some(1)),
            (b"ok\xe2\x82", 2, None),
            (b"\xc0\x80", 0, Some(1)),
            (b"\xed\xa0\x80", 0, Some(1)),
        ] {
            let source = CString::new(input).unwrap();
            let storage = source.as_c_str();
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
    fn to_string_lossy_c_str_borrows_valid_utf8() {
        for expected in ["", "text", "é说明", "e\u{301}"] {
            let input = expected.as_bytes();
            let source = CString::new(input).unwrap();
            let storage = source.as_c_str();
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
    fn to_string_lossy_c_str_owns_replacement_text() {
        for (input, expected) in [
            (b"a\xffb".as_slice(), "a�b"),
            (b"ok\xe2\x82", "ok�"),
            (b"\xc0\x80", "��"),
            (b"\xed\xa0\x80", "���"),
        ] {
            let source = CString::new(input).unwrap();
            let storage = source.as_c_str();
            let value = StringBytes::to_string_lossy(storage);
            assert!(matches!(value, Cow::Owned(_)));
            assert_eq!(value, expected);
            assert_eq!(StringBytes::as_bytes(storage), input);
        }
    }

    #[test]
    fn to_string_unchecked_c_str_copies_valid_utf8() {
        for expected in ["", "text", "é说明", "e\u{301}"] {
            let input = expected.as_bytes();
            let source = CString::new(input).unwrap();
            let storage = source.as_c_str();
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
    fn box_into_bytes_c_str_reuses_allocation() {
        for (input, expected) in [
            (b"\0".as_slice(), ""),
            (b"text\0", "text"),
            ("é说明\0".as_bytes(), "é说明"),
        ] {
            let storage: Box<CStr> = Box::<CStr>::from(CStr::from_bytes_with_nul(input).unwrap());
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let capacity = input.len();
            let value = <CStr as StringBytes>::box_into_bytes(storage);
            assert_eq!(value, expected.as_bytes());
            assert_eq!(value.capacity(), capacity);
            if capacity != 0 {
                assert_eq!(value.as_ptr(), address);
            }
        }
    }

    #[test]
    fn box_into_string_c_str_reuses_allocation() {
        for (input, expected) in [
            (b"\0".as_slice(), ""),
            (b"text\0", "text"),
            ("é说明\0".as_bytes(), "é说明"),
        ] {
            let storage: Box<CStr> = Box::<CStr>::from(CStr::from_bytes_with_nul(input).unwrap());
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let capacity = input.len();
            let value = <CStr as StringBytes>::box_into_string(storage).unwrap();
            assert_eq!(value, expected);
            assert_eq!(value.capacity(), capacity);
            if capacity != 0 {
                assert_eq!(value.as_ptr(), address);
            }
        }
    }

    #[test]
    fn box_into_string_c_str_error_keeps_allocation() {
        let storage = Box::<CStr>::from(c"ok\xff");
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let capacity = 4;
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
