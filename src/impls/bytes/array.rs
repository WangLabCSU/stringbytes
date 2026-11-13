use std::rc::Rc;
use std::string::FromUtf8Error;
use std::sync::Arc;

use crate::impls::{impl_pointer_conversions_for_sized, impl_string_bytes};
use crate::{sealed, StringBytes};

// Arrays store bytes inline, but boxed arrays can give their allocation to
// the resulting vector or string.
impl_string_bytes!([const N: usize]; [u8; N] {
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        self
    }

    impl_pointer_conversions_for_sized! {
        #[inline]
        fn box_into_bytes(self: Box<Self>) -> Vec<u8> {
            // Passing self as an argument lets Box<[u8; N]> coerce to Box<[u8]>.
            <[u8]>::into_vec(self)
        }
    }
});

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use crate::StringBytes;

    #[test]
    fn as_bytes_array_exposes_stable_contents() {
        for input in [
            *b"text",
            *b"\0e\xcc\x81",
            *b"ok\xff\0",
            *b"ok\xe2\x82",
            *b"\xc0\x80\0\0",
            *b"\xed\xa0\x80\0",
        ] {
            let storage = input;
            assert_eq!(StringBytes::as_bytes(&storage), input);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            assert_eq!(StringBytes::as_bytes(&storage), input);
            assert_eq!(StringBytes::as_bytes(&storage).as_ptr(), address);
        }
    }

    #[test]
    fn as_bytes_array_matches_as_ref() {
        for input in [
            *b"text",
            *b"\0e\xcc\x81",
            *b"ok\xff\0",
            *b"ok\xe2\x82",
            *b"\xc0\x80\0\0",
            *b"\xed\xa0\x80\0",
        ] {
            let storage = input;
            let bytes = StringBytes::as_bytes(&storage);
            let other = AsRef::<[u8]>::as_ref(&storage);
            assert_eq!(bytes, input);
            assert_eq!(other, input);
            assert!(std::ptr::eq(bytes, other));
        }
    }

    #[test]
    fn as_bytes_array_reflects_as_mut_edits() {
        let input = *b"text";
        let mut storage = input;
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
    fn to_bytes_array_returns_an_independent_copy() {
        for input in [
            *b"text",
            *b"\0e\xcc\x81",
            *b"ok\xff\0",
            *b"ok\xe2\x82",
            *b"\xc0\x80\0\0",
            *b"\xed\xa0\x80\0",
        ] {
            let storage = input;
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
    fn into_bytes_array_copies_storage() {
        for input in [
            *b"text",
            *b"\0e\xcc\x81",
            *b"ok\xff\0",
            *b"ok\xe2\x82",
            *b"\xc0\x80\0\0",
            *b"\xed\xa0\x80\0",
        ] {
            let storage = input;
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let bytes = StringBytes::into_bytes(storage);
            assert_eq!(bytes, input);
            if !input.is_empty() {
                assert_ne!(bytes.as_ptr(), address);
            }
        }
    }

    #[test]
    fn into_bytes_empty_array() {
        let storage: [u8; 0] = [];
        let value = StringBytes::into_bytes(storage);
        assert_eq!(value, b"");
    }

    #[test]
    fn try_borrow_str_array_borrows_valid_utf8() {
        for (input, expected) in [
            (*b"text", "text"),
            (*b"\xc3\xa9ab", "éab"),
            (*b"\0e\xcc\x81", "\0e\u{301}"),
        ] {
            let storage = input;
            let value = StringBytes::try_borrow_str(&storage).unwrap();
            assert_eq!(value, expected);
            assert!(std::ptr::eq(
                value.as_bytes(),
                StringBytes::as_bytes(&storage)
            ));
        }
    }

    #[test]
    fn try_borrow_str_array_reports_invalid_utf8() {
        for (input, valid_up_to, error_len) in [
            (*b"ok\xff\0", 2, Some(1)),
            (*b"ok\xe2\x82", 2, None),
            (*b"\xc0\x80\0\0", 0, Some(1)),
            (*b"\xed\xa0\x80\0", 0, Some(1)),
        ] {
            let storage = input;
            let error = StringBytes::try_borrow_str(&storage).unwrap_err();
            assert_eq!(error.valid_up_to(), valid_up_to);
            assert_eq!(error.error_len(), error_len);
            assert_eq!(StringBytes::as_bytes(&storage), input);
        }
    }

    #[test]
    fn borrow_str_unchecked_array_borrows_valid_utf8() {
        for (input, expected) in [
            (*b"text", "text"),
            (*b"\xc3\xa9ab", "éab"),
            (*b"\0e\xcc\x81", "\0e\u{301}"),
        ] {
            let storage = input;
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
    fn try_to_string_array_copies_valid_utf8() {
        for (input, expected) in [
            (*b"text", "text"),
            (*b"\xc3\xa9ab", "éab"),
            (*b"\0e\xcc\x81", "\0e\u{301}"),
        ] {
            let storage = input;
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
    fn try_to_string_array_returns_a_copy_of_invalid_bytes() {
        for (input, valid_up_to, error_len) in [
            (*b"ok\xff\0", 2, Some(1)),
            (*b"ok\xe2\x82", 2, None),
            (*b"\xc0\x80\0\0", 0, Some(1)),
            (*b"\xed\xa0\x80\0", 0, Some(1)),
        ] {
            let storage = input;
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
    fn to_string_lossy_array_borrows_valid_utf8() {
        for (input, expected) in [
            (*b"text", "text"),
            (*b"\xc3\xa9ab", "éab"),
            (*b"\0e\xcc\x81", "\0e\u{301}"),
        ] {
            let storage = input;
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
    fn to_string_lossy_array_owns_replacement_text() {
        for (input, expected) in [
            (*b"ok\xff\0", "ok�\0"),
            (*b"ok\xe2\x82", "ok�"),
            (*b"\xc0\x80\0\0", "��\0\0"),
            (*b"\xed\xa0\x80\0", "���\0"),
        ] {
            let storage = input;
            let value = StringBytes::to_string_lossy(&storage);
            assert!(matches!(value, Cow::Owned(_)));
            assert_eq!(value, expected);
            assert_eq!(StringBytes::as_bytes(&storage), input);
        }
    }

    #[test]
    fn to_string_unchecked_array_copies_valid_utf8() {
        for (input, expected) in [
            (*b"text", "text"),
            (*b"\xc3\xa9ab", "éab"),
            (*b"\0e\xcc\x81", "\0e\u{301}"),
        ] {
            let storage = input;
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
    fn try_into_string_array_copies_storage() {
        for (input, expected) in [
            (*b"text", "text"),
            (*b"\xc3\xa9ab", "éab"),
            (*b"\0e\xcc\x81", "\0e\u{301}"),
        ] {
            let storage = input;
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let value = StringBytes::try_into_string(storage).unwrap();
            assert_eq!(value, expected);
            if !input.is_empty() {
                assert_ne!(value.as_ptr(), address);
            }
        }
    }

    #[test]
    fn try_into_string_array_error_copies_storage() {
        for (input, valid_up_to, error_len) in [
            (*b"ok\xff\0", 2, Some(1)),
            (*b"ok\xe2\x82", 2, None),
            (*b"\xc0\x80\0\0", 0, Some(1)),
            (*b"\xed\xa0\x80\0", 0, Some(1)),
        ] {
            let storage = input;
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
    fn try_into_string_empty_array() {
        let storage: [u8; 0] = [];
        let value = StringBytes::try_into_string(storage).unwrap();
        assert_eq!(value, "");
    }

    #[test]
    fn into_string_lossy_array_valid_utf8_copies_storage() {
        for (input, expected) in [
            (*b"text", "text"),
            (*b"\xc3\xa9ab", "éab"),
            (*b"\0e\xcc\x81", "\0e\u{301}"),
        ] {
            let storage = input;
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let value = StringBytes::into_string_lossy(storage);
            assert_eq!(value, expected);
            if !input.is_empty() {
                assert_ne!(value.as_ptr(), address);
            }
        }
    }

    #[test]
    fn into_string_lossy_array_replaces_invalid_utf8() {
        for (input, expected) in [
            (*b"ok\xff\0", "ok�\0"),
            (*b"ok\xe2\x82", "ok�"),
            (*b"\xc0\x80\0\0", "��\0\0"),
            (*b"\xed\xa0\x80\0", "���\0"),
        ] {
            let storage = input;
            let value = StringBytes::into_string_lossy(storage);
            assert_eq!(value, expected);
        }
    }

    #[test]
    fn into_string_lossy_empty_array() {
        let storage: [u8; 0] = [];
        let value = StringBytes::into_string_lossy(storage);
        assert_eq!(value, "");
    }

    #[test]
    fn into_string_unchecked_array_copies_storage() {
        for (input, expected) in [
            (*b"text", "text"),
            (*b"\xc3\xa9ab", "éab"),
            (*b"\0e\xcc\x81", "\0e\u{301}"),
        ] {
            let storage = input;
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
    fn into_string_unchecked_empty_array() {
        let storage: [u8; 0] = [];
        // SAFETY: the storage contains the valid UTF-8 literal above.
        let value = unsafe { StringBytes::into_string_unchecked(storage) };
        assert_eq!(value, "");
    }

    #[test]
    fn box_into_bytes_array_reuses_allocation() {
        let storage = Box::new(*b"text");
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
    fn box_into_bytes_empty_array_has_no_allocation() {
        let storage = Box::new([0_u8; 0]);
        let capacity = 0;
        let value = StringBytes::box_into_bytes(storage);
        assert_eq!(value, b"");
        assert_eq!(value.capacity(), capacity);
    }

    #[test]
    fn box_into_string_array_reuses_allocation() {
        let storage = Box::new(*b"text");
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
    fn box_into_string_array_error_keeps_allocation() {
        let storage = Box::new(*b"ok\xff");
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

    #[test]
    fn box_into_string_empty_array_has_no_allocation() {
        let storage = Box::new([0_u8; 0]);
        let capacity = 0;
        let value = StringBytes::box_into_string(storage).unwrap();
        assert_eq!(value, "");
        assert_eq!(value.capacity(), capacity);
    }
}
