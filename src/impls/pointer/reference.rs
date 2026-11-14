use std::borrow::Cow;
use std::rc::Rc;
use std::sync::Arc;

use crate::impls::impl_pointer_conversions_for_sized;
use crate::impls::pointer::impl_string_bytes_for_pointer;
use crate::{sealed, StringBytes, Utf8Error};

impl_string_bytes_for_pointer!(
    /// Borrows the value's bytes and delegates string conversions to `T`.
    ///
    /// Consuming the reference leaves the referenced value in place.
    /// Owned byte and string results copy its contents. UTF-8 validation
    /// follows `T`'s string conversions.
    &T => to_bytes
);

#[cfg(test)]
mod tests {
    use std::borrow::Cow;
    use std::cell::RefCell;

    use crate::StringBytes;

    // Records dispatch only; each test asserts its own expected calls.
    struct RecordingStorage<'a> {
        bytes: &'a [u8],
        calls: &'a RefCell<Vec<&'static str>>,
    }

    impl crate::sealed::Sealed for RecordingStorage<'_> {}

    impl StringBytes for RecordingStorage<'_> {
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

        fn try_borrow_str(&self) -> Result<&str, crate::Utf8Error> {
            self.calls.borrow_mut().push("try_borrow_str");
            std::str::from_utf8(self.bytes).map_err(crate::Utf8Error::from)
        }

        fn try_to_string(&self) -> Result<String, crate::Utf8Error> {
            self.calls.borrow_mut().push("try_to_string");
            std::str::from_utf8(self.bytes)
                .map(str::to_owned)
                .map_err(crate::Utf8Error::from)
        }

        fn to_string_lossy(&self) -> Cow<'_, str> {
            self.calls.borrow_mut().push("to_string_lossy");
            String::from_utf8_lossy(self.bytes)
        }
    }

    #[test]
    fn as_bytes_reference_exposes_stable_contents() {
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
            let storage = &source;
            assert_eq!(StringBytes::as_bytes(&storage), input);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            assert_eq!(StringBytes::as_bytes(&storage), input);
            assert_eq!(StringBytes::as_bytes(&storage).as_ptr(), address);
        }
    }

    #[test]
    fn as_bytes_reference_matches_as_ref() {
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
            let storage = &source;
            let bytes = StringBytes::as_bytes(&storage);
            let other = AsRef::<[u8]>::as_ref(&storage);
            assert_eq!(bytes, input);
            assert_eq!(other, input);
            assert!(std::ptr::eq(bytes, other));
        }
    }

    #[test]
    fn as_bytes_reference_matches_deref() {
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
            let storage = &source;
            let bytes = StringBytes::as_bytes(&storage);
            let other = StringBytes::as_bytes(std::ops::Deref::deref(&storage));
            assert_eq!(bytes, input);
            assert_eq!(other, input);
            assert!(std::ptr::eq(bytes, other));
        }
    }

    #[test]
    fn as_bytes_str_reference_exposes_stable_contents() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let storage = expected;
            assert_eq!(StringBytes::as_bytes(&storage), input);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            assert_eq!(StringBytes::as_bytes(&storage), input);
            assert_eq!(StringBytes::as_bytes(&storage).as_ptr(), address);
        }
    }

    #[test]
    fn as_bytes_str_reference_matches_as_ref() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let storage = expected;
            let bytes = StringBytes::as_bytes(&storage);
            let other = AsRef::<[u8]>::as_ref(&storage);
            assert_eq!(bytes, input);
            assert_eq!(other, input);
            assert!(std::ptr::eq(bytes, other));
        }
    }

    #[test]
    fn as_bytes_str_reference_matches_deref() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let storage = expected;
            let bytes = StringBytes::as_bytes(&storage);
            let other = StringBytes::as_bytes(std::ops::Deref::deref(&storage));
            assert_eq!(bytes, input);
            assert_eq!(other, input);
            assert!(std::ptr::eq(bytes, other));
        }
    }

    #[test]
    fn to_bytes_reference_returns_an_independent_copy() {
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
            let storage = &source;
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
    fn to_bytes_str_reference_returns_an_independent_copy() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let storage = expected;
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
    fn into_bytes_reference_copies_storage() {
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
            let storage = &source;
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let bytes = StringBytes::into_bytes(storage);
            assert_eq!(bytes, input);
            if !input.is_empty() {
                assert_ne!(bytes.as_ptr(), address);
            }
        }
    }

    #[test]
    fn into_bytes_str_reference_copies_storage() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let storage = expected;
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let bytes = StringBytes::into_bytes(storage);
            assert_eq!(bytes, input);
            if !input.is_empty() {
                assert_ne!(bytes.as_ptr(), address);
            }
        }
    }

    #[test]
    fn try_borrow_str_reference_borrows_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let source = input.to_vec();
            let storage = &source;
            let value = StringBytes::try_borrow_str(&storage).unwrap();
            assert_eq!(value, expected);
            assert!(std::ptr::eq(
                value.as_bytes(),
                StringBytes::as_bytes(&storage)
            ));
        }
    }

    #[test]
    fn try_borrow_str_reference_reports_invalid_utf8() {
        for (input, valid_up_to, error_len) in [
            (b"ok\xff".as_slice(), 2, Some(1)),
            (b"ok\xe2\x82", 2, None),
            (b"\xc0\x80", 0, Some(1)),
            (b"\xed\xa0\x80", 0, Some(1)),
        ] {
            let source = input.to_vec();
            let storage = &source;
            let error = StringBytes::try_borrow_str(&storage).unwrap_err();
            assert_eq!(error.valid_up_to(), valid_up_to);
            assert_eq!(error.error_len(), error_len);
            assert_eq!(StringBytes::as_bytes(&storage), input);
        }
    }

    #[test]
    fn try_borrow_str_str_reference_borrows_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let storage = expected;
            let value = StringBytes::try_borrow_str(&storage).unwrap();
            assert_eq!(value, expected);
            assert!(std::ptr::eq(
                value.as_bytes(),
                StringBytes::as_bytes(&storage)
            ));
        }
    }

    #[test]
    fn try_borrow_str_reference_forwards_text_override() {
        let input = "étext".as_bytes();
        let calls = RefCell::new(Vec::new());
        let source = RecordingStorage {
            bytes: input,
            calls: &calls,
        };
        let storage = &source;
        let value = StringBytes::try_borrow_str(&storage).unwrap();
        assert_eq!(*calls.borrow(), ["try_borrow_str"]);
        assert_eq!(value, "étext");
    }

    #[test]
    fn borrow_str_unchecked_reference_borrows_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let source = input.to_vec();
            let storage = &source;
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
    fn borrow_str_unchecked_str_reference_borrows_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let storage = expected;
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
    fn try_to_string_reference_copies_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let source = input.to_vec();
            let storage = &source;
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
    fn try_to_string_reference_reports_invalid_utf8_without_changing_storage() {
        for (input, valid_up_to, error_len) in [
            (b"ok\xff".as_slice(), 2, Some(1)),
            (b"ok\xe2\x82", 2, None),
            (b"\xc0\x80", 0, Some(1)),
            (b"\xed\xa0\x80", 0, Some(1)),
        ] {
            let source = input.to_vec();
            let storage = &source;
            let error = StringBytes::try_to_string(&storage).unwrap_err();
            assert_eq!(error.valid_up_to(), valid_up_to);
            assert_eq!(error.error_len(), error_len);
            assert_eq!(StringBytes::as_bytes(&storage), input);
        }
    }

    #[test]
    fn try_to_string_str_reference_copies_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let storage = expected;
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
    fn try_to_string_reference_calls_only_inner_override() {
        let input = "étext".as_bytes();
        let calls = RefCell::new(Vec::new());
        let source = RecordingStorage {
            bytes: input,
            calls: &calls,
        };
        let storage = &source;
        let value = StringBytes::try_to_string(&storage).unwrap();
        assert_eq!(*calls.borrow(), ["try_to_string"]);
        assert_eq!(value, "étext");
    }

    #[test]
    fn to_string_lossy_reference_borrows_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let source = input.to_vec();
            let storage = &source;
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
    fn to_string_lossy_reference_owns_replacement_text() {
        for (input, expected) in [
            (b"a\xffb".as_slice(), "a�b"),
            (b"ok\xe2\x82", "ok�"),
            (b"\xc0\x80", "��"),
            (b"\xed\xa0\x80", "���"),
        ] {
            let source = input.to_vec();
            let storage = &source;
            let value = StringBytes::to_string_lossy(&storage);
            assert!(matches!(value, Cow::Owned(_)));
            assert_eq!(value, expected);
            assert_eq!(StringBytes::as_bytes(&storage), input);
        }
    }

    #[test]
    fn to_string_lossy_str_reference_borrows_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let storage = expected;
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
    fn to_string_lossy_reference_forwards_text_override() {
        let input = "étext".as_bytes();
        let calls = RefCell::new(Vec::new());
        let source = RecordingStorage {
            bytes: input,
            calls: &calls,
        };
        let storage = &source;
        let value = StringBytes::to_string_lossy(&storage);
        assert_eq!(*calls.borrow(), ["to_string_lossy"]);
        assert_eq!(value, "étext");
    }

    #[test]
    fn to_string_unchecked_reference_copies_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let source = input.to_vec();
            let storage = &source;
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
    fn to_string_unchecked_str_reference_copies_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let storage = expected;
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
    fn try_into_string_reference_copies_storage() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let source = input.to_vec();
            let storage = &source;
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let value = StringBytes::try_into_string(storage).unwrap();
            assert_eq!(value, expected);
            if !input.is_empty() {
                assert_ne!(value.as_ptr(), address);
            }
        }
    }

    #[test]
    fn try_into_string_reference_error_returns_original() {
        for (input, valid_up_to, error_len) in [
            (b"ok\xff".as_slice(), 2, Some(1)),
            (b"ok\xe2\x82", 2, None),
            (b"\xc0\x80", 0, Some(1)),
            (b"\xed\xa0\x80", 0, Some(1)),
        ] {
            let source = input.to_vec();
            let storage = &source;
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
    fn try_into_string_str_reference_copies_storage() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let storage = expected;
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let value = StringBytes::try_into_string(storage).unwrap();
            assert_eq!(value, expected);
            if !input.is_empty() {
                assert_ne!(value.as_ptr(), address);
            }
        }
    }

    #[test]
    fn try_into_string_reference_forwards_text_override() {
        let input = "étext".as_bytes();
        let calls = RefCell::new(Vec::new());
        let source = RecordingStorage {
            bytes: input,
            calls: &calls,
        };
        let storage = &source;
        let value = StringBytes::try_into_string(storage).unwrap();
        assert_eq!(*calls.borrow(), ["try_borrow_str", "to_bytes"]);
        assert_eq!(value, "étext");
    }

    #[test]
    fn try_into_string_reference_returns_original_without_byte_conversion() {
        let input = b"ok\xff";
        let calls = RefCell::new(Vec::new());
        let source = RecordingStorage {
            bytes: input,
            calls: &calls,
        };
        let storage = &source;
        let address = input.as_ptr();
        let error = StringBytes::try_into_string(storage).unwrap_err();
        assert_eq!(*calls.borrow(), ["try_borrow_str"]);
        assert_eq!(error.utf8_error().valid_up_to(), 2);
        let recovered = error.into_inner();
        assert_eq!(StringBytes::as_bytes(&recovered), b"ok\xff");
        assert_eq!(StringBytes::as_bytes(&recovered).as_ptr(), address);
    }

    #[test]
    fn into_string_lossy_reference_valid_utf8_copies_storage() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let source = input.to_vec();
            let storage = &source;
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let value = StringBytes::into_string_lossy(storage);
            assert_eq!(value, expected);
            if !input.is_empty() {
                assert_ne!(value.as_ptr(), address);
            }
        }
    }

    #[test]
    fn into_string_lossy_reference_replaces_invalid_utf8() {
        for (input, expected) in [
            (b"a\xffb".as_slice(), "a�b"),
            (b"ok\xe2\x82", "ok�"),
            (b"\xc0\x80", "��"),
            (b"\xed\xa0\x80", "���"),
        ] {
            let source = input.to_vec();
            let storage = &source;
            let value = StringBytes::into_string_lossy(storage);
            assert_eq!(value, expected);
        }
    }

    #[test]
    fn into_string_lossy_str_reference_valid_utf8_copies_storage() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let storage = expected;
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let value = StringBytes::into_string_lossy(storage);
            assert_eq!(value, expected);
            if !input.is_empty() {
                assert_ne!(value.as_ptr(), address);
            }
        }
    }

    #[test]
    fn into_string_lossy_reference_forwards_text_override() {
        let input = "étext".as_bytes();
        let calls = RefCell::new(Vec::new());
        let source = RecordingStorage {
            bytes: input,
            calls: &calls,
        };
        let storage = &source;
        let value = StringBytes::into_string_lossy(storage);
        assert_eq!(*calls.borrow(), ["to_string_lossy", "to_bytes"]);
        assert_eq!(value, "étext");
    }

    #[test]
    fn into_string_lossy_reference_does_not_make_an_intermediate_copy() {
        let input = b"a\xffb";
        let calls = RefCell::new(Vec::new());
        let source = RecordingStorage {
            bytes: input,
            calls: &calls,
        };
        let storage = &source;
        assert_eq!(StringBytes::into_string_lossy(storage), "a�b");
        assert_eq!(*calls.borrow(), ["to_string_lossy"]);
    }

    #[test]
    fn into_string_unchecked_reference_copies_storage() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let source = input.to_vec();
            let storage = &source;
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
    fn into_string_unchecked_str_reference_copies_storage() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let storage = expected;
            let address = StringBytes::as_bytes(&storage).as_ptr();
            // SAFETY: these test literals are valid UTF-8.
            let value = unsafe { StringBytes::into_string_unchecked(storage) };
            assert_eq!(value, expected);
            if !input.is_empty() {
                assert_ne!(value.as_ptr(), address);
            }
        }
    }
}
