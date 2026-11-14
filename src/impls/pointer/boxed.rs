use std::borrow::Cow;
use std::rc::Rc;
use std::str::Utf8Error;
use std::string::FromUtf8Error;
use std::sync::Arc;

use crate::impls::impl_pointer_conversions_for_sized;
use crate::impls::pointer::impl_string_bytes_for_pointer;
use crate::{sealed, StringBytes};

impl_string_bytes_for_pointer!(Box<T> => box_into_bytes, box_into_string);

#[cfg(test)]
mod tests {
    use std::borrow::Cow;
    use std::io::IoSlice;
    use std::io::IoSliceMut;
    use std::rc::Rc;
    use std::sync::Arc;

    use crate::impls::pointer::fixtures::{
        CheckedTextOverrides, LossyTextOverride, NoIntermediateCopy, NoPrevalidation,
    };
    use crate::StringBytes;

    #[test]
    fn as_bytes_box_vec_exposes_stable_contents() {
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
            let storage = Box::new(source);
            assert_eq!(StringBytes::as_bytes(&storage), input);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            assert_eq!(StringBytes::as_bytes(&storage), input);
            assert_eq!(StringBytes::as_bytes(&storage).as_ptr(), address);
        }
    }

    #[test]
    fn as_bytes_box_vec_matches_deref() {
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
            let storage = Box::new(source);
            let bytes = StringBytes::as_bytes(&storage);
            let other = StringBytes::as_bytes(std::ops::Deref::deref(&storage));
            assert_eq!(bytes, input);
            assert_eq!(other, input);
            assert!(std::ptr::eq(bytes, other));
        }
    }

    #[test]
    fn as_bytes_nested_exposes_stable_contents() {
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
            let storage = Box::new(Rc::new(Arc::new(Box::new(source))));
            assert_eq!(StringBytes::as_bytes(&storage), input);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            assert_eq!(StringBytes::as_bytes(&storage), input);
            assert_eq!(StringBytes::as_bytes(&storage).as_ptr(), address);
        }
    }

    #[test]
    fn as_bytes_nested_matches_deref() {
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
            let storage = Box::new(Rc::new(Arc::new(Box::new(source))));
            let bytes = StringBytes::as_bytes(&storage);
            let other = StringBytes::as_bytes(std::ops::Deref::deref(&storage));
            assert_eq!(bytes, input);
            assert_eq!(other, input);
            assert!(std::ptr::eq(bytes, other));
        }
    }

    #[test]
    fn as_bytes_boxed_slice_reflects_as_mut_edits() {
        let mut storage = Box::<[u8]>::from(b"text".as_slice());
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
    fn to_bytes_box_vec_returns_an_independent_copy() {
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
            let storage = Box::new(source);
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
    fn to_bytes_nested_returns_an_independent_copy() {
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
            let storage = Box::new(Rc::new(Arc::new(Box::new(source))));
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
    fn into_bytes_box_vec_reuses_buffer() {
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
            let storage = Box::new(source);
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
    fn into_bytes_nested_reuses_buffer() {
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
            let storage = Box::new(Rc::new(Arc::new(Box::new(source))));
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
    fn into_bytes_box_io_slice_copies_current_view() {
        let mut view = IoSlice::new(b"prefixtext");
        view.advance(6);
        let storage = Box::new(view);
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let value = StringBytes::into_bytes(storage);
        assert_eq!(value, b"text");
        assert_ne!(value.as_ptr(), address);
    }

    #[test]
    fn into_bytes_box_io_slice_mut_copies_current_view() {
        let mut source = *b"prefixtext";
        let mut view = IoSliceMut::new(&mut source);
        view.advance(6);
        let storage = Box::new(view);
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let value = StringBytes::into_bytes(storage);
        assert_eq!(value, b"text");
        assert_ne!(value.as_ptr(), address);
        assert_eq!(&source, b"prefixtext");
    }

    #[test]
    fn into_bytes_box_c_string_reuses_buffer_without_nul() {
        let storage = Box::new(std::ffi::CString::new(b"text".as_slice()).unwrap());
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let capacity = 5;
        let value = StringBytes::into_bytes(storage);
        assert_eq!(value, b"text");
        if capacity != 0 {
            assert_eq!(value.as_ptr(), address);
        }
        assert_eq!(value.capacity(), capacity);
    }

    #[test]
    fn into_bytes_boxed_array_reuses_allocation() {
        let storage = Box::new(*b"text");
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let capacity = 4;
        let value = StringBytes::into_bytes(storage);
        assert_eq!(value, b"text");
        if capacity != 0 {
            assert_eq!(value.as_ptr(), address);
        }
        assert_eq!(value.capacity(), capacity);
    }

    #[test]
    fn try_borrow_str_box_vec_borrows_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let storage = Box::new(source);
            let value = StringBytes::try_borrow_str(&storage).unwrap();
            assert_eq!(value, expected);
            assert!(std::ptr::eq(
                value.as_bytes(),
                StringBytes::as_bytes(&storage)
            ));
        }
    }

    #[test]
    fn try_borrow_str_box_vec_reports_invalid_utf8() {
        for (input, valid_up_to, error_len) in [
            (b"ok\xff".as_slice(), 2, Some(1)),
            (b"ok\xe2\x82", 2, None),
            (b"\xc0\x80", 0, Some(1)),
            (b"\xed\xa0\x80", 0, Some(1)),
        ] {
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let storage = Box::new(source);
            let error = StringBytes::try_borrow_str(&storage).unwrap_err();
            assert_eq!(error.valid_up_to(), valid_up_to);
            assert_eq!(error.error_len(), error_len);
            assert_eq!(StringBytes::as_bytes(&storage), input);
        }
    }

    #[test]
    fn try_borrow_str_nested_borrows_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let storage = Box::new(Rc::new(Arc::new(Box::new(source))));
            let value = StringBytes::try_borrow_str(&storage).unwrap();
            assert_eq!(value, expected);
            assert!(std::ptr::eq(
                value.as_bytes(),
                StringBytes::as_bytes(&storage)
            ));
        }
    }

    #[test]
    fn try_borrow_str_nested_reports_invalid_utf8() {
        for (input, valid_up_to, error_len) in [
            (b"ok\xff".as_slice(), 2, Some(1)),
            (b"ok\xe2\x82", 2, None),
            (b"\xc0\x80", 0, Some(1)),
            (b"\xed\xa0\x80", 0, Some(1)),
        ] {
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let storage = Box::new(Rc::new(Arc::new(Box::new(source))));
            let error = StringBytes::try_borrow_str(&storage).unwrap_err();
            assert_eq!(error.valid_up_to(), valid_up_to);
            assert_eq!(error.error_len(), error_len);
            assert_eq!(StringBytes::as_bytes(&storage), input);
        }
    }

    #[test]
    fn try_borrow_str_box_forwards_text_override() {
        let storage = Box::new(CheckedTextOverrides("étext"));
        let value = StringBytes::try_borrow_str(&storage).unwrap();
        assert_eq!(value, "étext");
    }

    #[test]
    fn borrow_str_unchecked_box_vec_borrows_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let storage = Box::new(source);
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
    fn borrow_str_unchecked_nested_borrows_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let storage = Box::new(Rc::new(Arc::new(Box::new(source))));
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
    fn try_to_string_box_vec_copies_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let storage = Box::new(source);
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
    fn try_to_string_box_vec_returns_a_copy_of_invalid_bytes() {
        for (input, valid_up_to, error_len) in [
            (b"ok\xff".as_slice(), 2, Some(1)),
            (b"ok\xe2\x82", 2, None),
            (b"\xc0\x80", 0, Some(1)),
            (b"\xed\xa0\x80", 0, Some(1)),
        ] {
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let storage = Box::new(source);
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
    fn try_to_string_nested_copies_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let storage = Box::new(Rc::new(Arc::new(Box::new(source))));
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
    fn try_to_string_nested_returns_a_copy_of_invalid_bytes() {
        for (input, valid_up_to, error_len) in [
            (b"ok\xff".as_slice(), 2, Some(1)),
            (b"ok\xe2\x82", 2, None),
            (b"\xc0\x80", 0, Some(1)),
            (b"\xed\xa0\x80", 0, Some(1)),
        ] {
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let storage = Box::new(Rc::new(Arc::new(Box::new(source))));
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
    fn try_to_string_box_forwards_text_override() {
        let storage = Box::new(CheckedTextOverrides("étext"));
        let value = StringBytes::try_to_string(&storage).unwrap();
        assert_eq!(value, "étext");
    }

    #[test]
    fn try_to_string_box_does_not_prevalidate() {
        let storage = Box::new(NoPrevalidation("étext".as_bytes()));
        assert_eq!(StringBytes::try_to_string(&storage).unwrap(), "étext");
    }

    #[test]
    fn to_string_lossy_box_vec_borrows_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let storage = Box::new(source);
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
    fn to_string_lossy_box_vec_owns_replacement_text() {
        for (input, expected) in [
            (b"a\xffb".as_slice(), "a�b"),
            (b"ok\xe2\x82", "ok�"),
            (b"\xc0\x80", "��"),
            (b"\xed\xa0\x80", "���"),
        ] {
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let storage = Box::new(source);
            let value = StringBytes::to_string_lossy(&storage);
            assert!(matches!(value, Cow::Owned(_)));
            assert_eq!(value, expected);
            assert_eq!(StringBytes::as_bytes(&storage), input);
        }
    }

    #[test]
    fn to_string_lossy_nested_borrows_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let storage = Box::new(Rc::new(Arc::new(Box::new(source))));
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
    fn to_string_lossy_nested_owns_replacement_text() {
        for (input, expected) in [
            (b"a\xffb".as_slice(), "a�b"),
            (b"ok\xe2\x82", "ok�"),
            (b"\xc0\x80", "��"),
            (b"\xed\xa0\x80", "���"),
        ] {
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let storage = Box::new(Rc::new(Arc::new(Box::new(source))));
            let value = StringBytes::to_string_lossy(&storage);
            assert!(matches!(value, Cow::Owned(_)));
            assert_eq!(value, expected);
            assert_eq!(StringBytes::as_bytes(&storage), input);
        }
    }

    #[test]
    fn to_string_lossy_box_forwards_text_override() {
        let storage = Box::new(LossyTextOverride("étext"));
        let value = StringBytes::to_string_lossy(&storage);
        assert_eq!(value, "étext");
    }

    #[test]
    fn to_string_unchecked_box_vec_copies_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let storage = Box::new(source);
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
    fn to_string_unchecked_nested_copies_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let storage = Box::new(Rc::new(Arc::new(Box::new(source))));
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
    fn try_into_string_box_vec_reuses_buffer() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let source_capacity = source.capacity();
            let storage = Box::new(source);
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
    fn try_into_string_box_vec_error_reuses_buffer() {
        for (input, valid_up_to, error_len) in [
            (b"ok\xff".as_slice(), 2, Some(1)),
            (b"ok\xe2\x82", 2, None),
            (b"\xc0\x80", 0, Some(1)),
            (b"\xed\xa0\x80", 0, Some(1)),
        ] {
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let source_capacity = source.capacity();
            let storage = Box::new(source);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let capacity = source_capacity;
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
    fn try_into_string_nested_reuses_buffer() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let source_capacity = source.capacity();
            let storage = Box::new(Rc::new(Arc::new(Box::new(source))));
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
    fn try_into_string_nested_error_reuses_buffer() {
        for (input, valid_up_to, error_len) in [
            (b"ok\xff".as_slice(), 2, Some(1)),
            (b"ok\xe2\x82", 2, None),
            (b"\xc0\x80", 0, Some(1)),
            (b"\xed\xa0\x80", 0, Some(1)),
        ] {
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let source_capacity = source.capacity();
            let storage = Box::new(Rc::new(Arc::new(Box::new(source))));
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let capacity = source_capacity;
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
    fn try_into_string_box_forwards_text_override() {
        let storage = Box::new(CheckedTextOverrides("étext"));
        let value = StringBytes::try_into_string(storage).unwrap();
        assert_eq!(value, "étext");
    }

    #[test]
    fn try_into_string_box_does_not_prevalidate() {
        let storage = Box::new(NoPrevalidation("étext".as_bytes()));
        assert_eq!(StringBytes::try_into_string(storage).unwrap(), "étext");
    }

    #[test]
    fn try_into_string_box_io_slice_copies_current_view() {
        let mut view = IoSlice::new(b"prefixtext");
        view.advance(6);
        let storage = Box::new(view);
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let value = StringBytes::try_into_string(storage).unwrap();
        assert_eq!(value, "text");
        assert_ne!(value.as_ptr(), address);
    }

    #[test]
    fn try_into_string_box_io_slice_mut_copies_current_view() {
        let mut source = *b"prefixtext";
        let mut view = IoSliceMut::new(&mut source);
        view.advance(6);
        let storage = Box::new(view);
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let value = StringBytes::try_into_string(storage).unwrap();
        assert_eq!(value, "text");
        assert_ne!(value.as_ptr(), address);
        assert_eq!(&source, b"prefixtext");
    }

    #[test]
    fn try_into_string_box_c_string_reuses_buffer_without_nul() {
        let storage = Box::new(std::ffi::CString::new(b"text".as_slice()).unwrap());
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let capacity = 5;
        let value = StringBytes::try_into_string(storage).unwrap();
        assert_eq!(value, "text");
        if capacity != 0 {
            assert_eq!(value.as_ptr(), address);
        }
        assert_eq!(value.capacity(), capacity);
    }

    #[test]
    fn try_into_string_boxed_array_reuses_allocation() {
        let storage = Box::new(*b"text");
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let capacity = 4;
        let value = StringBytes::try_into_string(storage).unwrap();
        assert_eq!(value, "text");
        if capacity != 0 {
            assert_eq!(value.as_ptr(), address);
        }
        assert_eq!(value.capacity(), capacity);
    }

    #[test]
    fn try_into_string_box_c_string_error_reuses_buffer_without_nul() {
        let storage = Box::new(std::ffi::CString::new(b"ok\xff".as_slice()).unwrap());
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let capacity = 4;
        let error = StringBytes::try_into_string(storage).unwrap_err();
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
    fn try_into_string_boxed_array_error_reuses_allocation() {
        let storage = Box::new(*b"ok\xff");
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let capacity = 3;
        let error = StringBytes::try_into_string(storage).unwrap_err();
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
    fn into_string_lossy_box_vec_valid_utf8_reuses_buffer() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let source_capacity = source.capacity();
            let storage = Box::new(source);
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
    fn into_string_lossy_box_vec_replaces_invalid_utf8() {
        for (input, expected) in [
            (b"a\xffb".as_slice(), "a�b"),
            (b"ok\xe2\x82", "ok�"),
            (b"\xc0\x80", "��"),
            (b"\xed\xa0\x80", "���"),
        ] {
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let storage = Box::new(source);
            let value = StringBytes::into_string_lossy(storage);
            assert_eq!(value, expected);
        }
    }

    #[test]
    fn into_string_lossy_nested_valid_utf8_reuses_buffer() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let source_capacity = source.capacity();
            let storage = Box::new(Rc::new(Arc::new(Box::new(source))));
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
    fn into_string_lossy_nested_replaces_invalid_utf8() {
        for (input, expected) in [
            (b"a\xffb".as_slice(), "a�b"),
            (b"ok\xe2\x82", "ok�"),
            (b"\xc0\x80", "��"),
            (b"\xed\xa0\x80", "���"),
        ] {
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let storage = Box::new(Rc::new(Arc::new(Box::new(source))));
            let value = StringBytes::into_string_lossy(storage);
            assert_eq!(value, expected);
        }
    }

    #[test]
    fn into_string_lossy_box_forwards_text_override() {
        let storage = Box::new(LossyTextOverride("étext"));
        let value = StringBytes::into_string_lossy(storage);
        assert_eq!(value, "étext");
    }

    #[test]
    fn into_string_lossy_box_does_not_make_an_intermediate_copy() {
        let storage = Box::new(NoIntermediateCopy(b"a\xffb"));
        assert_eq!(StringBytes::into_string_lossy(storage), "a�b");
    }

    #[test]
    fn into_string_lossy_box_c_string_reuses_buffer_without_nul() {
        let storage = Box::new(std::ffi::CString::new(b"text".as_slice()).unwrap());
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let capacity = 5;
        let value = StringBytes::into_string_lossy(storage);
        assert_eq!(value, "text");
        if capacity != 0 {
            assert_eq!(value.as_ptr(), address);
        }
        assert_eq!(value.capacity(), capacity);
    }

    #[test]
    fn into_string_lossy_boxed_array_reuses_allocation() {
        let storage = Box::new(*b"text");
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let capacity = 4;
        let value = StringBytes::into_string_lossy(storage);
        assert_eq!(value, "text");
        if capacity != 0 {
            assert_eq!(value.as_ptr(), address);
        }
        assert_eq!(value.capacity(), capacity);
    }

    #[test]
    fn into_string_unchecked_box_vec_reuses_buffer() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let source_capacity = source.capacity();
            let storage = Box::new(source);
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
    fn into_string_unchecked_nested_reuses_buffer() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let source_capacity = source.capacity();
            let storage = Box::new(Rc::new(Arc::new(Box::new(source))));
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
    fn into_string_unchecked_box_c_string_reuses_buffer_without_nul() {
        let storage = Box::new(std::ffi::CString::new(b"text".as_slice()).unwrap());
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let capacity = 5;
        // SAFETY: the storage contains the valid UTF-8 literal above.
        let value = unsafe { StringBytes::into_string_unchecked(storage) };
        assert_eq!(value, "text");
        if capacity != 0 {
            assert_eq!(value.as_ptr(), address);
        }
        assert_eq!(value.capacity(), capacity);
    }

    #[test]
    fn into_string_unchecked_boxed_array_reuses_allocation() {
        let storage = Box::new(*b"text");
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let capacity = 4;
        // SAFETY: the storage contains the valid UTF-8 literal above.
        let value = unsafe { StringBytes::into_string_unchecked(storage) };
        assert_eq!(value, "text");
        if capacity != 0 {
            assert_eq!(value.as_ptr(), address);
        }
        assert_eq!(value.capacity(), capacity);
    }

    #[test]
    fn box_into_bytes_vec_reuses_buffer() {
        let mut source = Vec::with_capacity(32);
        source.extend_from_slice(b"text");
        let source_capacity = source.capacity();
        let storage = Box::new(source);
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let capacity = source_capacity;
        let value = StringBytes::box_into_bytes(storage);
        assert_eq!(value, b"text");
        if capacity != 0 {
            assert_eq!(value.as_ptr(), address);
        }
        assert_eq!(value.capacity(), capacity);
    }

    #[test]
    fn box_into_string_vec_reuses_buffer() {
        let mut source = Vec::with_capacity(32);
        source.extend_from_slice(b"text");
        let source_capacity = source.capacity();
        let storage = Box::new(source);
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let capacity = source_capacity;
        let value = StringBytes::box_into_string(storage).unwrap();
        assert_eq!(value, "text");
        if capacity != 0 {
            assert_eq!(value.as_ptr(), address);
        }
        assert_eq!(value.capacity(), capacity);
    }

    #[test]
    fn box_into_string_vec_error_reuses_buffer() {
        let mut source = Vec::with_capacity(32);
        source.extend_from_slice(b"ok\xff");
        let source_capacity = source.capacity();
        let storage = Box::new(source);
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let capacity = source_capacity;
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
