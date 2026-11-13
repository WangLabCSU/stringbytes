use std::io::IoSliceMut;

use crate::impls::impl_string_bytes;
use crate::{sealed, StringBytes};

impl_string_bytes!(IoSliceMut<'_> {
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        self
    }
});

#[cfg(test)]
mod tests {
    use std::borrow::Cow;
    use std::io::IoSliceMut;

    use crate::StringBytes;

    #[test]
    fn as_bytes_io_slice_mut_exposes_stable_contents() {
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
            let mut source = input.to_vec();
            let storage = IoSliceMut::new(&mut source);
            assert_eq!(StringBytes::as_bytes(&storage), input);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            assert_eq!(StringBytes::as_bytes(&storage), input);
            assert_eq!(StringBytes::as_bytes(&storage).as_ptr(), address);
        }
    }

    #[test]
    fn as_bytes_io_slice_mut_matches_deref() {
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
            let mut source = input.to_vec();
            let storage = IoSliceMut::new(&mut source);
            let bytes = StringBytes::as_bytes(&storage);
            let other = StringBytes::as_bytes(std::ops::Deref::deref(&storage));
            assert_eq!(bytes, input);
            assert_eq!(other, input);
            assert!(std::ptr::eq(bytes, other));
        }
    }

    #[test]
    fn as_bytes_io_slice_mut_uses_advanced_view() {
        let mut source = *b"prefixtext";
        let mut storage = IoSliceMut::new(&mut source);
        storage.advance(6);
        let value = StringBytes::as_bytes(&storage);
        assert_eq!(value, b"text");
    }

    #[test]
    fn as_bytes_io_slice_mut_exhausted_view_is_empty() {
        let mut source = *b"prefixtext";
        let mut storage = IoSliceMut::new(&mut source);
        storage.advance(6);
        storage.advance(4);
        assert_eq!(StringBytes::as_bytes(&storage), b"");
    }

    #[test]
    fn as_bytes_io_slice_mut_reflects_edits_to_current_view() {
        let mut source = *b"prefixtext";
        let address = source[6..].as_ptr();
        {
            let mut storage = IoSliceMut::new(&mut source);
            storage.advance(6);
            storage.copy_from_slice(b"a\xffb\0");
            assert_eq!(StringBytes::as_bytes(&storage), b"a\xffb\0");
            assert_eq!(StringBytes::as_bytes(&storage).as_ptr(), address);
        }
        assert_eq!(&source, b"prefixa\xffb\0");
    }

    #[test]
    fn to_bytes_io_slice_mut_returns_an_independent_copy() {
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
            let mut source = input.to_vec();
            let storage = IoSliceMut::new(&mut source);
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
    fn to_bytes_io_slice_mut_uses_advanced_view() {
        let mut source = *b"prefixtext";
        let mut storage = IoSliceMut::new(&mut source);
        storage.advance(6);
        let value = StringBytes::to_bytes(&storage);
        assert_eq!(value, b"text");
    }

    #[test]
    fn into_bytes_io_slice_mut_copies_storage() {
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
            let mut source = input.to_vec();
            let storage = IoSliceMut::new(&mut source);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let bytes = StringBytes::into_bytes(storage);
            assert_eq!(bytes, input);
            if !input.is_empty() {
                assert_ne!(bytes.as_ptr(), address);
            }
        }
    }

    #[test]
    fn into_bytes_io_slice_mut_uses_advanced_view() {
        let mut source = *b"prefixtext";
        let mut storage = IoSliceMut::new(&mut source);
        storage.advance(6);
        let value = StringBytes::into_bytes(storage);
        assert_eq!(value, b"text");
        assert_eq!(&source, b"prefixtext");
    }

    #[test]
    fn try_borrow_str_io_slice_mut_borrows_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut source = input.to_vec();
            let storage = IoSliceMut::new(&mut source);
            let value = StringBytes::try_borrow_str(&storage).unwrap();
            assert_eq!(value, expected);
            assert!(std::ptr::eq(
                value.as_bytes(),
                StringBytes::as_bytes(&storage)
            ));
        }
    }

    #[test]
    fn try_borrow_str_io_slice_mut_reports_invalid_utf8() {
        for (input, valid_up_to, error_len) in [
            (b"ok\xff".as_slice(), 2, Some(1)),
            (b"ok\xe2\x82", 2, None),
            (b"\xc0\x80", 0, Some(1)),
            (b"\xed\xa0\x80", 0, Some(1)),
        ] {
            let mut source = input.to_vec();
            let storage = IoSliceMut::new(&mut source);
            let error = StringBytes::try_borrow_str(&storage).unwrap_err();
            assert_eq!(error.valid_up_to(), valid_up_to);
            assert_eq!(error.error_len(), error_len);
            assert_eq!(StringBytes::as_bytes(&storage), input);
        }
    }

    #[test]
    fn try_borrow_str_io_slice_mut_uses_advanced_view() {
        let mut source = *b"prefixtext";
        let mut storage = IoSliceMut::new(&mut source);
        storage.advance(6);
        let value = StringBytes::try_borrow_str(&storage).unwrap();
        assert_eq!(value, "text");
    }

    #[test]
    fn try_borrow_str_io_slice_mut_rejects_view_starting_inside_utf8() {
        let mut source = "étext".as_bytes().to_vec();
        let mut storage = IoSliceMut::new(&mut source);
        storage.advance(1);
        let error = StringBytes::try_borrow_str(&storage).unwrap_err();
        assert_eq!(error.valid_up_to(), 0);
        assert_eq!(error.error_len(), Some(1));
    }

    #[test]
    fn borrow_str_unchecked_io_slice_mut_borrows_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut source = input.to_vec();
            let storage = IoSliceMut::new(&mut source);
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
    fn borrow_str_unchecked_io_slice_mut_uses_advanced_view() {
        let mut source = *b"prefixtext";
        let mut storage = IoSliceMut::new(&mut source);
        storage.advance(6);
        // SAFETY: advancing past the ASCII prefix leaves the UTF-8 literal "text".
        let value = unsafe { StringBytes::borrow_str_unchecked(&storage) };
        assert_eq!(value, "text");
    }

    #[test]
    fn try_to_string_io_slice_mut_copies_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut source = input.to_vec();
            let storage = IoSliceMut::new(&mut source);
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
    fn try_to_string_io_slice_mut_returns_a_copy_of_invalid_bytes() {
        for (input, valid_up_to, error_len) in [
            (b"ok\xff".as_slice(), 2, Some(1)),
            (b"ok\xe2\x82", 2, None),
            (b"\xc0\x80", 0, Some(1)),
            (b"\xed\xa0\x80", 0, Some(1)),
        ] {
            let mut source = input.to_vec();
            let storage = IoSliceMut::new(&mut source);
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
    fn try_to_string_io_slice_mut_uses_advanced_view() {
        let mut source = *b"prefixtext";
        let mut storage = IoSliceMut::new(&mut source);
        storage.advance(6);
        let value = StringBytes::try_to_string(&storage).unwrap();
        assert_eq!(value, "text");
    }

    #[test]
    fn try_to_string_io_slice_mut_rejects_view_starting_inside_utf8() {
        let mut source = "étext".as_bytes().to_vec();
        let mut storage = IoSliceMut::new(&mut source);
        storage.advance(1);
        let error = StringBytes::try_to_string(&storage).unwrap_err();
        assert_eq!(error.utf8_error().valid_up_to(), 0);
        assert_eq!(error.utf8_error().error_len(), Some(1));
        assert_eq!(error.into_bytes(), b"\xa9text");
    }

    #[test]
    fn to_string_lossy_io_slice_mut_borrows_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut source = input.to_vec();
            let storage = IoSliceMut::new(&mut source);
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
    fn to_string_lossy_io_slice_mut_owns_replacement_text() {
        for (input, expected) in [
            (b"a\xffb".as_slice(), "a�b"),
            (b"ok\xe2\x82", "ok�"),
            (b"\xc0\x80", "��"),
            (b"\xed\xa0\x80", "���"),
        ] {
            let mut source = input.to_vec();
            let storage = IoSliceMut::new(&mut source);
            let value = StringBytes::to_string_lossy(&storage);
            assert!(matches!(value, Cow::Owned(_)));
            assert_eq!(value, expected);
            assert_eq!(StringBytes::as_bytes(&storage), input);
        }
    }

    #[test]
    fn to_string_lossy_io_slice_mut_uses_advanced_view() {
        let mut source = *b"prefixtext";
        let mut storage = IoSliceMut::new(&mut source);
        storage.advance(6);
        let value = StringBytes::to_string_lossy(&storage);
        assert_eq!(value, "text");
    }

    #[test]
    fn to_string_unchecked_io_slice_mut_copies_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut source = input.to_vec();
            let storage = IoSliceMut::new(&mut source);
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
    fn to_string_unchecked_io_slice_mut_uses_advanced_view() {
        let mut source = *b"prefixtext";
        let mut storage = IoSliceMut::new(&mut source);
        storage.advance(6);
        // SAFETY: advancing past the ASCII prefix leaves the UTF-8 literal "text".
        let value = unsafe { StringBytes::to_string_unchecked(&storage) };
        assert_eq!(value, "text");
    }

    #[test]
    fn try_into_string_io_slice_mut_copies_storage() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut source = input.to_vec();
            let storage = IoSliceMut::new(&mut source);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let value = StringBytes::try_into_string(storage).unwrap();
            assert_eq!(value, expected);
            if !input.is_empty() {
                assert_ne!(value.as_ptr(), address);
            }
        }
    }

    #[test]
    fn try_into_string_io_slice_mut_error_copies_storage() {
        for (input, valid_up_to, error_len) in [
            (b"ok\xff".as_slice(), 2, Some(1)),
            (b"ok\xe2\x82", 2, None),
            (b"\xc0\x80", 0, Some(1)),
            (b"\xed\xa0\x80", 0, Some(1)),
        ] {
            let mut source = input.to_vec();
            let storage = IoSliceMut::new(&mut source);
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
    fn try_into_string_io_slice_mut_uses_advanced_view() {
        let mut source = *b"prefixtext";
        let mut storage = IoSliceMut::new(&mut source);
        storage.advance(6);
        let value = StringBytes::try_into_string(storage).unwrap();
        assert_eq!(value, "text");
        assert_eq!(&source, b"prefixtext");
    }

    #[test]
    fn try_into_string_io_slice_mut_rejects_view_starting_inside_utf8() {
        let mut source = "étext".as_bytes().to_vec();
        let mut storage = IoSliceMut::new(&mut source);
        storage.advance(1);
        let error = StringBytes::try_into_string(storage).unwrap_err();
        assert_eq!(error.utf8_error().valid_up_to(), 0);
        assert_eq!(error.utf8_error().error_len(), Some(1));
        assert_eq!(error.into_bytes(), b"\xa9text");
    }

    #[test]
    fn into_string_lossy_io_slice_mut_valid_utf8_copies_storage() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut source = input.to_vec();
            let storage = IoSliceMut::new(&mut source);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let value = StringBytes::into_string_lossy(storage);
            assert_eq!(value, expected);
            if !input.is_empty() {
                assert_ne!(value.as_ptr(), address);
            }
        }
    }

    #[test]
    fn into_string_lossy_io_slice_mut_replaces_invalid_utf8() {
        for (input, expected) in [
            (b"a\xffb".as_slice(), "a�b"),
            (b"ok\xe2\x82", "ok�"),
            (b"\xc0\x80", "��"),
            (b"\xed\xa0\x80", "���"),
        ] {
            let mut source = input.to_vec();
            let storage = IoSliceMut::new(&mut source);
            let value = StringBytes::into_string_lossy(storage);
            assert_eq!(value, expected);
        }
    }

    #[test]
    fn into_string_lossy_io_slice_mut_uses_advanced_view() {
        let mut source = *b"prefixtext";
        let mut storage = IoSliceMut::new(&mut source);
        storage.advance(6);
        let value = StringBytes::into_string_lossy(storage);
        assert_eq!(value, "text");
        assert_eq!(&source, b"prefixtext");
    }

    #[test]
    fn into_string_unchecked_io_slice_mut_copies_storage() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut source = input.to_vec();
            let storage = IoSliceMut::new(&mut source);
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
    fn into_string_unchecked_io_slice_mut_uses_advanced_view() {
        let mut source = *b"prefixtext";
        let mut storage = IoSliceMut::new(&mut source);
        storage.advance(6);
        // SAFETY: advancing past the ASCII prefix leaves the UTF-8 literal "text".
        let value = unsafe { StringBytes::into_string_unchecked(storage) };
        assert_eq!(value, "text");
        assert_eq!(&source, b"prefixtext");
    }
}
