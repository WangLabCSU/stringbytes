use std::borrow::Cow;
use std::rc::Rc;
use std::string::FromUtf8Error;
use std::sync::Arc;

use crate::impls::{impl_pointer_conversions_for_sized, impl_string_bytes};
use crate::{sealed, StringBytes};

// Neither variant owns a transferable allocation; copy directly from the
// array without first cloning it through Cow::into_owned.
impl_string_bytes!([const N: usize]; Cow<'_, [u8; N]> {
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        self.as_ref()
    }

    impl_pointer_conversions_for_sized! {
        #[inline]
        fn box_into_bytes(self: Box<Self>) -> Vec<u8> {
            // An owned array is part of the Cow enum, whose allocation cannot
            // be transferred to Vec. Copy without first moving the Cow out.
            (*self).to_bytes()
        }
    }
});

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use crate::StringBytes;

    #[test]
    fn as_bytes_cow_array_borrowed_exposes_stable_contents() {
        for input in [
            *b"text",
            *b"\0e\xcc\x81",
            *b"ok\xff\0",
            *b"ok\xe2\x82",
            *b"\xc0\x80\0\0",
            *b"\xed\xa0\x80\0",
        ] {
            let source = input;
            let storage: Cow<'_, [u8; 4]> = Cow::Borrowed(&source);
            assert_eq!(StringBytes::as_bytes(&storage), input);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            assert_eq!(StringBytes::as_bytes(&storage), input);
            assert_eq!(StringBytes::as_bytes(&storage).as_ptr(), address);
        }
    }

    #[test]
    fn as_bytes_cow_array_borrowed_matches_deref() {
        for input in [
            *b"text",
            *b"\0e\xcc\x81",
            *b"ok\xff\0",
            *b"ok\xe2\x82",
            *b"\xc0\x80\0\0",
            *b"\xed\xa0\x80\0",
        ] {
            let source = input;
            let storage: Cow<'_, [u8; 4]> = Cow::Borrowed(&source);
            let bytes = StringBytes::as_bytes(&storage);
            let other = StringBytes::as_bytes(std::ops::Deref::deref(&storage));
            assert_eq!(bytes, input);
            assert_eq!(other, input);
            assert!(std::ptr::eq(bytes, other));
        }
    }

    #[test]
    fn as_bytes_cow_array_owned_exposes_stable_contents() {
        for input in [
            *b"text",
            *b"\0e\xcc\x81",
            *b"ok\xff\0",
            *b"ok\xe2\x82",
            *b"\xc0\x80\0\0",
            *b"\xed\xa0\x80\0",
        ] {
            let storage: Cow<'_, [u8; 4]> = Cow::Owned(input);
            assert_eq!(StringBytes::as_bytes(&storage), input);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            assert_eq!(StringBytes::as_bytes(&storage), input);
            assert_eq!(StringBytes::as_bytes(&storage).as_ptr(), address);
        }
    }

    #[test]
    fn as_bytes_cow_array_owned_matches_deref() {
        for input in [
            *b"text",
            *b"\0e\xcc\x81",
            *b"ok\xff\0",
            *b"ok\xe2\x82",
            *b"\xc0\x80\0\0",
            *b"\xed\xa0\x80\0",
        ] {
            let storage: Cow<'_, [u8; 4]> = Cow::Owned(input);
            let bytes = StringBytes::as_bytes(&storage);
            let other = StringBytes::as_bytes(std::ops::Deref::deref(&storage));
            assert_eq!(bytes, input);
            assert_eq!(other, input);
            assert!(std::ptr::eq(bytes, other));
        }
    }

    #[test]
    fn to_bytes_cow_array_borrowed_returns_an_independent_copy() {
        for input in [
            *b"text",
            *b"\0e\xcc\x81",
            *b"ok\xff\0",
            *b"ok\xe2\x82",
            *b"\xc0\x80\0\0",
            *b"\xed\xa0\x80\0",
        ] {
            let source = input;
            let storage: Cow<'_, [u8; 4]> = Cow::Borrowed(&source);
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
    fn to_bytes_cow_array_owned_returns_an_independent_copy() {
        for input in [
            *b"text",
            *b"\0e\xcc\x81",
            *b"ok\xff\0",
            *b"ok\xe2\x82",
            *b"\xc0\x80\0\0",
            *b"\xed\xa0\x80\0",
        ] {
            let storage: Cow<'_, [u8; 4]> = Cow::Owned(input);
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
    fn into_bytes_cow_array_borrowed_copies_storage() {
        for input in [
            *b"text",
            *b"\0e\xcc\x81",
            *b"ok\xff\0",
            *b"ok\xe2\x82",
            *b"\xc0\x80\0\0",
            *b"\xed\xa0\x80\0",
        ] {
            let source = input;
            let storage: Cow<'_, [u8; 4]> = Cow::Borrowed(&source);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let bytes = StringBytes::into_bytes(storage);
            assert_eq!(bytes, input);
            if !input.is_empty() {
                assert_ne!(bytes.as_ptr(), address);
            }
        }
    }

    #[test]
    fn into_bytes_cow_array_owned_copies_storage() {
        for input in [
            *b"text",
            *b"\0e\xcc\x81",
            *b"ok\xff\0",
            *b"ok\xe2\x82",
            *b"\xc0\x80\0\0",
            *b"\xed\xa0\x80\0",
        ] {
            let storage: Cow<'_, [u8; 4]> = Cow::Owned(input);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let bytes = StringBytes::into_bytes(storage);
            assert_eq!(bytes, input);
            if !input.is_empty() {
                assert_ne!(bytes.as_ptr(), address);
            }
        }
    }

    #[test]
    fn into_bytes_empty_borrowed_cow_array() {
        let source = [];
        let storage: Cow<'_, [u8; 0]> = Cow::Borrowed(&source);
        let value = StringBytes::into_bytes(storage);
        assert_eq!(value, b"");
    }

    #[test]
    fn into_bytes_empty_owned_cow_array() {
        let storage: Cow<'_, [u8; 0]> = Cow::Owned([]);
        let value = StringBytes::into_bytes(storage);
        assert_eq!(value, b"");
    }

    #[test]
    fn try_borrow_str_cow_array_borrowed_borrows_valid_utf8() {
        for (input, expected) in [
            (*b"text", "text"),
            (*b"\xc3\xa9ab", "éab"),
            (*b"\0e\xcc\x81", "\0e\u{301}"),
        ] {
            let source = input;
            let storage: Cow<'_, [u8; 4]> = Cow::Borrowed(&source);
            let value = StringBytes::try_borrow_str(&storage).unwrap();
            assert_eq!(value, expected);
            assert!(std::ptr::eq(
                value.as_bytes(),
                StringBytes::as_bytes(&storage)
            ));
        }
    }

    #[test]
    fn try_borrow_str_cow_array_borrowed_reports_invalid_utf8() {
        for (input, valid_up_to, error_len) in [
            (*b"ok\xff\0", 2, Some(1)),
            (*b"ok\xe2\x82", 2, None),
            (*b"\xc0\x80\0\0", 0, Some(1)),
            (*b"\xed\xa0\x80\0", 0, Some(1)),
        ] {
            let source = input;
            let storage: Cow<'_, [u8; 4]> = Cow::Borrowed(&source);
            let error = StringBytes::try_borrow_str(&storage).unwrap_err();
            assert_eq!(error.valid_up_to(), valid_up_to);
            assert_eq!(error.error_len(), error_len);
            assert_eq!(StringBytes::as_bytes(&storage), input);
        }
    }

    #[test]
    fn try_borrow_str_cow_array_owned_borrows_valid_utf8() {
        for (input, expected) in [
            (*b"text", "text"),
            (*b"\xc3\xa9ab", "éab"),
            (*b"\0e\xcc\x81", "\0e\u{301}"),
        ] {
            let storage: Cow<'_, [u8; 4]> = Cow::Owned(input);
            let value = StringBytes::try_borrow_str(&storage).unwrap();
            assert_eq!(value, expected);
            assert!(std::ptr::eq(
                value.as_bytes(),
                StringBytes::as_bytes(&storage)
            ));
        }
    }

    #[test]
    fn try_borrow_str_cow_array_owned_reports_invalid_utf8() {
        for (input, valid_up_to, error_len) in [
            (*b"ok\xff\0", 2, Some(1)),
            (*b"ok\xe2\x82", 2, None),
            (*b"\xc0\x80\0\0", 0, Some(1)),
            (*b"\xed\xa0\x80\0", 0, Some(1)),
        ] {
            let storage: Cow<'_, [u8; 4]> = Cow::Owned(input);
            let error = StringBytes::try_borrow_str(&storage).unwrap_err();
            assert_eq!(error.valid_up_to(), valid_up_to);
            assert_eq!(error.error_len(), error_len);
            assert_eq!(StringBytes::as_bytes(&storage), input);
        }
    }

    #[test]
    fn borrow_str_unchecked_cow_array_borrowed_borrows_valid_utf8() {
        for (input, expected) in [
            (*b"text", "text"),
            (*b"\xc3\xa9ab", "éab"),
            (*b"\0e\xcc\x81", "\0e\u{301}"),
        ] {
            let source = input;
            let storage: Cow<'_, [u8; 4]> = Cow::Borrowed(&source);
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
    fn borrow_str_unchecked_cow_array_owned_borrows_valid_utf8() {
        for (input, expected) in [
            (*b"text", "text"),
            (*b"\xc3\xa9ab", "éab"),
            (*b"\0e\xcc\x81", "\0e\u{301}"),
        ] {
            let storage: Cow<'_, [u8; 4]> = Cow::Owned(input);
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
    fn try_to_string_cow_array_borrowed_copies_valid_utf8() {
        for (input, expected) in [
            (*b"text", "text"),
            (*b"\xc3\xa9ab", "éab"),
            (*b"\0e\xcc\x81", "\0e\u{301}"),
        ] {
            let source = input;
            let storage: Cow<'_, [u8; 4]> = Cow::Borrowed(&source);
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
    fn try_to_string_cow_array_borrowed_returns_a_copy_of_invalid_bytes() {
        for (input, valid_up_to, error_len) in [
            (*b"ok\xff\0", 2, Some(1)),
            (*b"ok\xe2\x82", 2, None),
            (*b"\xc0\x80\0\0", 0, Some(1)),
            (*b"\xed\xa0\x80\0", 0, Some(1)),
        ] {
            let source = input;
            let storage: Cow<'_, [u8; 4]> = Cow::Borrowed(&source);
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
    fn try_to_string_cow_array_owned_copies_valid_utf8() {
        for (input, expected) in [
            (*b"text", "text"),
            (*b"\xc3\xa9ab", "éab"),
            (*b"\0e\xcc\x81", "\0e\u{301}"),
        ] {
            let storage: Cow<'_, [u8; 4]> = Cow::Owned(input);
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
    fn try_to_string_cow_array_owned_returns_a_copy_of_invalid_bytes() {
        for (input, valid_up_to, error_len) in [
            (*b"ok\xff\0", 2, Some(1)),
            (*b"ok\xe2\x82", 2, None),
            (*b"\xc0\x80\0\0", 0, Some(1)),
            (*b"\xed\xa0\x80\0", 0, Some(1)),
        ] {
            let storage: Cow<'_, [u8; 4]> = Cow::Owned(input);
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
    fn to_string_lossy_cow_array_borrowed_borrows_valid_utf8() {
        for (input, expected) in [
            (*b"text", "text"),
            (*b"\xc3\xa9ab", "éab"),
            (*b"\0e\xcc\x81", "\0e\u{301}"),
        ] {
            let source = input;
            let storage: Cow<'_, [u8; 4]> = Cow::Borrowed(&source);
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
    fn to_string_lossy_cow_array_borrowed_owns_replacement_text() {
        for (input, expected) in [
            (*b"ok\xff\0", "ok�\0"),
            (*b"ok\xe2\x82", "ok�"),
            (*b"\xc0\x80\0\0", "��\0\0"),
            (*b"\xed\xa0\x80\0", "���\0"),
        ] {
            let source = input;
            let storage: Cow<'_, [u8; 4]> = Cow::Borrowed(&source);
            let value = StringBytes::to_string_lossy(&storage);
            assert!(matches!(value, Cow::Owned(_)));
            assert_eq!(value, expected);
            assert_eq!(StringBytes::as_bytes(&storage), input);
        }
    }

    #[test]
    fn to_string_lossy_cow_array_owned_borrows_valid_utf8() {
        for (input, expected) in [
            (*b"text", "text"),
            (*b"\xc3\xa9ab", "éab"),
            (*b"\0e\xcc\x81", "\0e\u{301}"),
        ] {
            let storage: Cow<'_, [u8; 4]> = Cow::Owned(input);
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
    fn to_string_lossy_cow_array_owned_owns_replacement_text() {
        for (input, expected) in [
            (*b"ok\xff\0", "ok�\0"),
            (*b"ok\xe2\x82", "ok�"),
            (*b"\xc0\x80\0\0", "��\0\0"),
            (*b"\xed\xa0\x80\0", "���\0"),
        ] {
            let storage: Cow<'_, [u8; 4]> = Cow::Owned(input);
            let value = StringBytes::to_string_lossy(&storage);
            assert!(matches!(value, Cow::Owned(_)));
            assert_eq!(value, expected);
            assert_eq!(StringBytes::as_bytes(&storage), input);
        }
    }

    #[test]
    fn to_string_unchecked_cow_array_borrowed_copies_valid_utf8() {
        for (input, expected) in [
            (*b"text", "text"),
            (*b"\xc3\xa9ab", "éab"),
            (*b"\0e\xcc\x81", "\0e\u{301}"),
        ] {
            let source = input;
            let storage: Cow<'_, [u8; 4]> = Cow::Borrowed(&source);
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
    fn to_string_unchecked_cow_array_owned_copies_valid_utf8() {
        for (input, expected) in [
            (*b"text", "text"),
            (*b"\xc3\xa9ab", "éab"),
            (*b"\0e\xcc\x81", "\0e\u{301}"),
        ] {
            let storage: Cow<'_, [u8; 4]> = Cow::Owned(input);
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
    fn try_into_string_cow_array_borrowed_copies_storage() {
        for (input, expected) in [
            (*b"text", "text"),
            (*b"\xc3\xa9ab", "éab"),
            (*b"\0e\xcc\x81", "\0e\u{301}"),
        ] {
            let source = input;
            let storage: Cow<'_, [u8; 4]> = Cow::Borrowed(&source);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let value = StringBytes::try_into_string(storage).unwrap();
            assert_eq!(value, expected);
            if !input.is_empty() {
                assert_ne!(value.as_ptr(), address);
            }
        }
    }

    #[test]
    fn try_into_string_cow_array_borrowed_error_copies_storage() {
        for (input, valid_up_to, error_len) in [
            (*b"ok\xff\0", 2, Some(1)),
            (*b"ok\xe2\x82", 2, None),
            (*b"\xc0\x80\0\0", 0, Some(1)),
            (*b"\xed\xa0\x80\0", 0, Some(1)),
        ] {
            let source = input;
            let storage: Cow<'_, [u8; 4]> = Cow::Borrowed(&source);
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
    fn try_into_string_cow_array_owned_copies_storage() {
        for (input, expected) in [
            (*b"text", "text"),
            (*b"\xc3\xa9ab", "éab"),
            (*b"\0e\xcc\x81", "\0e\u{301}"),
        ] {
            let storage: Cow<'_, [u8; 4]> = Cow::Owned(input);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let value = StringBytes::try_into_string(storage).unwrap();
            assert_eq!(value, expected);
            if !input.is_empty() {
                assert_ne!(value.as_ptr(), address);
            }
        }
    }

    #[test]
    fn try_into_string_cow_array_owned_error_copies_storage() {
        for (input, valid_up_to, error_len) in [
            (*b"ok\xff\0", 2, Some(1)),
            (*b"ok\xe2\x82", 2, None),
            (*b"\xc0\x80\0\0", 0, Some(1)),
            (*b"\xed\xa0\x80\0", 0, Some(1)),
        ] {
            let storage: Cow<'_, [u8; 4]> = Cow::Owned(input);
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
    fn try_into_string_empty_borrowed_cow_array() {
        let source = [];
        let storage: Cow<'_, [u8; 0]> = Cow::Borrowed(&source);
        let value = StringBytes::try_into_string(storage).unwrap();
        assert_eq!(value, "");
    }

    #[test]
    fn try_into_string_empty_owned_cow_array() {
        let storage: Cow<'_, [u8; 0]> = Cow::Owned([]);
        let value = StringBytes::try_into_string(storage).unwrap();
        assert_eq!(value, "");
    }

    #[test]
    fn into_string_lossy_cow_array_borrowed_valid_utf8_copies_storage() {
        for (input, expected) in [
            (*b"text", "text"),
            (*b"\xc3\xa9ab", "éab"),
            (*b"\0e\xcc\x81", "\0e\u{301}"),
        ] {
            let source = input;
            let storage: Cow<'_, [u8; 4]> = Cow::Borrowed(&source);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let value = StringBytes::into_string_lossy(storage);
            assert_eq!(value, expected);
            if !input.is_empty() {
                assert_ne!(value.as_ptr(), address);
            }
        }
    }

    #[test]
    fn into_string_lossy_cow_array_borrowed_replaces_invalid_utf8() {
        for (input, expected) in [
            (*b"ok\xff\0", "ok�\0"),
            (*b"ok\xe2\x82", "ok�"),
            (*b"\xc0\x80\0\0", "��\0\0"),
            (*b"\xed\xa0\x80\0", "���\0"),
        ] {
            let source = input;
            let storage: Cow<'_, [u8; 4]> = Cow::Borrowed(&source);
            let value = StringBytes::into_string_lossy(storage);
            assert_eq!(value, expected);
        }
    }

    #[test]
    fn into_string_lossy_cow_array_owned_valid_utf8_copies_storage() {
        for (input, expected) in [
            (*b"text", "text"),
            (*b"\xc3\xa9ab", "éab"),
            (*b"\0e\xcc\x81", "\0e\u{301}"),
        ] {
            let storage: Cow<'_, [u8; 4]> = Cow::Owned(input);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let value = StringBytes::into_string_lossy(storage);
            assert_eq!(value, expected);
            if !input.is_empty() {
                assert_ne!(value.as_ptr(), address);
            }
        }
    }

    #[test]
    fn into_string_lossy_cow_array_owned_replaces_invalid_utf8() {
        for (input, expected) in [
            (*b"ok\xff\0", "ok�\0"),
            (*b"ok\xe2\x82", "ok�"),
            (*b"\xc0\x80\0\0", "��\0\0"),
            (*b"\xed\xa0\x80\0", "���\0"),
        ] {
            let storage: Cow<'_, [u8; 4]> = Cow::Owned(input);
            let value = StringBytes::into_string_lossy(storage);
            assert_eq!(value, expected);
        }
    }

    #[test]
    fn into_string_lossy_empty_borrowed_cow_array() {
        let source = [];
        let storage: Cow<'_, [u8; 0]> = Cow::Borrowed(&source);
        let value = StringBytes::into_string_lossy(storage);
        assert_eq!(value, "");
    }

    #[test]
    fn into_string_lossy_empty_owned_cow_array() {
        let storage: Cow<'_, [u8; 0]> = Cow::Owned([]);
        let value = StringBytes::into_string_lossy(storage);
        assert_eq!(value, "");
    }

    #[test]
    fn into_string_unchecked_cow_array_borrowed_copies_storage() {
        for (input, expected) in [
            (*b"text", "text"),
            (*b"\xc3\xa9ab", "éab"),
            (*b"\0e\xcc\x81", "\0e\u{301}"),
        ] {
            let source = input;
            let storage: Cow<'_, [u8; 4]> = Cow::Borrowed(&source);
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
    fn into_string_unchecked_cow_array_owned_copies_storage() {
        for (input, expected) in [
            (*b"text", "text"),
            (*b"\xc3\xa9ab", "éab"),
            (*b"\0e\xcc\x81", "\0e\u{301}"),
        ] {
            let storage: Cow<'_, [u8; 4]> = Cow::Owned(input);
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
    fn into_string_unchecked_empty_borrowed_cow_array() {
        let source = [];
        let storage: Cow<'_, [u8; 0]> = Cow::Borrowed(&source);
        // SAFETY: the storage contains the valid UTF-8 literal above.
        let value = unsafe { StringBytes::into_string_unchecked(storage) };
        assert_eq!(value, "");
    }

    #[test]
    fn into_string_unchecked_empty_owned_cow_array() {
        let storage: Cow<'_, [u8; 0]> = Cow::Owned([]);
        // SAFETY: the storage contains the valid UTF-8 literal above.
        let value = unsafe { StringBytes::into_string_unchecked(storage) };
        assert_eq!(value, "");
    }

    #[test]
    fn box_into_bytes_borrowed_cow_array_copies() {
        let source = *b"text";
        let storage = Box::new(Cow::<'_, [u8; 4]>::Borrowed(&source));
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let value = StringBytes::box_into_bytes(storage);
        assert_eq!(value, b"text");
        assert_ne!(value.as_ptr(), address);
    }

    #[test]
    fn box_into_bytes_owned_cow_array_copies() {
        let source = *b"text";
        let storage = Box::new(Cow::<'_, [u8; 4]>::Owned(source));
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let value = StringBytes::box_into_bytes(storage);
        assert_eq!(value, b"text");
        assert_ne!(value.as_ptr(), address);
    }

    #[test]
    fn box_into_string_borrowed_cow_array_copies() {
        let source = *b"text";
        let storage = Box::new(Cow::<'_, [u8; 4]>::Borrowed(&source));
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let value = StringBytes::box_into_string(storage).unwrap();
        assert_eq!(value, "text");
        assert_ne!(value.as_ptr(), address);
    }

    #[test]
    fn box_into_string_borrowed_cow_array_error_copies() {
        let source = *b"ok\xff";
        let storage = Box::new(Cow::<'_, [u8; 3]>::Borrowed(&source));
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let error = StringBytes::box_into_string(storage).unwrap_err();
        assert_eq!(error.utf8_error().valid_up_to(), 2);
        assert_eq!(error.utf8_error().error_len(), Some(1));
        let value = error.into_bytes();
        assert_eq!(value, b"ok\xff");
        assert_ne!(value.as_ptr(), address);
    }

    #[test]
    fn box_into_string_owned_cow_array_copies() {
        let source = *b"text";
        let storage = Box::new(Cow::<'_, [u8; 4]>::Owned(source));
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let value = StringBytes::box_into_string(storage).unwrap();
        assert_eq!(value, "text");
        assert_ne!(value.as_ptr(), address);
    }

    #[test]
    fn box_into_string_owned_cow_array_error_copies() {
        let source = *b"ok\xff";
        let storage = Box::new(Cow::<'_, [u8; 3]>::Owned(source));
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let error = StringBytes::box_into_string(storage).unwrap_err();
        assert_eq!(error.utf8_error().valid_up_to(), 2);
        assert_eq!(error.utf8_error().error_len(), Some(1));
        let value = error.into_bytes();
        assert_eq!(value, b"ok\xff");
        assert_ne!(value.as_ptr(), address);
    }
}
