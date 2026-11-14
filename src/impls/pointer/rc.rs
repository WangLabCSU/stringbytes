use std::borrow::Cow;
use std::rc::Rc;
use std::sync::Arc;

use crate::impls::impl_pointer_conversions_for_sized;
use crate::impls::pointer::impl_string_bytes_for_pointer;
use crate::{sealed, FromUtf8Error, StringBytes, Utf8Error};

impl_string_bytes_for_pointer!(
    /// Borrows the shared value's bytes and delegates string conversions to `T`.
    ///
    /// [`into_bytes`](StringBytes::into_bytes) and successful
    /// [`try_into_string`](StringBytes::try_into_string) calls reuse an inner
    /// buffer when there is only one strong owner and `T` can transfer it.
    /// Additional strong owners require a copy; [`Weak`](std::rc::Weak) pointers
    /// do not prevent reuse.
    /// Inline and unsized contents are copied.
    ///
    /// UTF-8 validation follows `T`'s string conversions.
    Rc<T> => rc_into_bytes, rc_into_string
);

#[cfg(test)]
mod tests {
    use std::borrow::Cow;
    use std::cell::RefCell;
    use std::ffi::CStr;
    use std::io::IoSlice;
    use std::io::IoSliceMut;
    use std::rc::Rc;
    use std::sync::Arc;

    use crate::impls::impl_pointer_conversions_for_sized;
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

        impl_pointer_conversions_for_sized!();
    }

    #[test]
    fn as_bytes_rc_vec_exposes_stable_contents() {
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
            let storage = Rc::new(source);
            assert_eq!(StringBytes::as_bytes(&storage), input);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            assert_eq!(StringBytes::as_bytes(&storage), input);
            assert_eq!(StringBytes::as_bytes(&storage).as_ptr(), address);
        }
    }

    #[test]
    fn as_bytes_rc_vec_matches_deref() {
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
            let storage = Rc::new(source);
            let bytes = StringBytes::as_bytes(&storage);
            let other = StringBytes::as_bytes(std::ops::Deref::deref(&storage));
            assert_eq!(bytes, input);
            assert_eq!(other, input);
            assert!(std::ptr::eq(bytes, other));
        }
    }

    #[test]
    fn to_bytes_rc_vec_returns_an_independent_copy() {
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
            let storage = Rc::new(source);
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
    fn into_bytes_rc_vec_reuses_buffer() {
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
            let storage = Rc::new(source);
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
    fn into_bytes_rc_vec_reuses_buffer_with_weak_reference() {
        let mut source = Vec::with_capacity(32);
        source.extend_from_slice(b"text");
        let capacity = source.capacity();
        let storage = Rc::new(source);
        let weak = Rc::downgrade(&storage);
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let value = StringBytes::into_bytes(storage);
        assert_eq!(value, b"text");
        if capacity != 0 {
            assert_eq!(value.as_ptr(), address);
        }
        assert_eq!(value.capacity(), capacity);
        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn into_bytes_rc_vec_copies_with_another_strong_owner() {
        let owner = Rc::new(Vec::from(b"text".as_slice()));
        let storage = Rc::clone(&owner);
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let value = StringBytes::into_bytes(storage);
        assert_eq!(value, b"text");
        assert_ne!(value.as_ptr(), address);
        assert_eq!(StringBytes::as_bytes(&owner), b"text");
    }

    #[test]
    fn into_bytes_nested_copies_with_another_strong_owner() {
        let owner = Rc::new(Arc::new(Box::new(Vec::from(b"text".as_slice()))));
        let storage = Rc::clone(&owner);
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let value = StringBytes::into_bytes(storage);
        assert_eq!(value, b"text");
        assert_ne!(value.as_ptr(), address);
        assert_eq!(StringBytes::as_bytes(&owner), b"text");
    }

    #[test]
    fn into_bytes_rc_slice_unique_copies() {
        let storage = Rc::<[u8]>::from(b"text".as_slice());
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let value = StringBytes::into_bytes(storage);
        assert_eq!(value, b"text");
        assert_ne!(value.as_ptr(), address);
    }

    #[test]
    fn into_bytes_rc_slice_shared_copies() {
        let storage = Rc::<[u8]>::from(b"text".as_slice());
        let other = Rc::clone(&storage);
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let value = StringBytes::into_bytes(storage);
        assert_eq!(value, b"text");
        assert_ne!(value.as_ptr(), address);
        assert_eq!(StringBytes::as_bytes(&other), b"text");
    }

    #[test]
    fn into_bytes_rc_str_unique_copies() {
        let storage = Rc::<str>::from("text");
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let value = StringBytes::into_bytes(storage);
        assert_eq!(value, b"text");
        assert_ne!(value.as_ptr(), address);
    }

    #[test]
    fn into_bytes_rc_str_shared_copies() {
        let storage = Rc::<str>::from("text");
        let other = Rc::clone(&storage);
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let value = StringBytes::into_bytes(storage);
        assert_eq!(value, b"text");
        assert_ne!(value.as_ptr(), address);
        assert_eq!(StringBytes::as_bytes(&other), b"text");
    }

    #[test]
    fn into_bytes_rc_c_str_unique_copies() {
        let storage = Rc::<CStr>::from(c"text");
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let value = StringBytes::into_bytes(storage);
        assert_eq!(value, b"text");
        assert_ne!(value.as_ptr(), address);
    }

    #[test]
    fn into_bytes_rc_c_str_shared_copies() {
        let storage = Rc::<CStr>::from(c"text");
        let other = Rc::clone(&storage);
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let value = StringBytes::into_bytes(storage);
        assert_eq!(value, b"text");
        assert_ne!(value.as_ptr(), address);
        assert_eq!(StringBytes::as_bytes(&other), b"text");
    }

    #[test]
    fn into_bytes_rc_io_slice_copies_current_view() {
        let mut view = IoSlice::new(b"prefixtext");
        view.advance(6);
        let storage = Rc::new(view);
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let value = StringBytes::into_bytes(storage);
        assert_eq!(value, b"text");
        assert_ne!(value.as_ptr(), address);
    }

    #[test]
    fn into_bytes_rc_io_slice_mut_copies_current_view() {
        let mut source = *b"prefixtext";
        let mut view = IoSliceMut::new(&mut source);
        view.advance(6);
        let storage = Rc::new(view);
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let value = StringBytes::into_bytes(storage);
        assert_eq!(value, b"text");
        assert_ne!(value.as_ptr(), address);
        assert_eq!(&source, b"prefixtext");
    }

    #[test]
    fn into_bytes_rc_c_string_reuses_buffer_without_nul() {
        let storage = Rc::new(std::ffi::CString::new(b"text".as_slice()).unwrap());
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
    fn try_borrow_str_rc_vec_borrows_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let storage = Rc::new(source);
            let value = StringBytes::try_borrow_str(&storage).unwrap();
            assert_eq!(value, expected);
            assert!(std::ptr::eq(
                value.as_bytes(),
                StringBytes::as_bytes(&storage)
            ));
        }
    }

    #[test]
    fn try_borrow_str_rc_vec_reports_invalid_utf8() {
        for (input, valid_up_to, error_len) in [
            (b"ok\xff".as_slice(), 2, Some(1)),
            (b"ok\xe2\x82", 2, None),
            (b"\xc0\x80", 0, Some(1)),
            (b"\xed\xa0\x80", 0, Some(1)),
        ] {
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let storage = Rc::new(source);
            let error = StringBytes::try_borrow_str(&storage).unwrap_err();
            assert_eq!(error.valid_up_to(), valid_up_to);
            assert_eq!(error.error_len(), error_len);
            assert_eq!(StringBytes::as_bytes(&storage), input);
        }
    }

    #[test]
    fn try_borrow_str_rc_forwards_text_override() {
        let input = "étext".as_bytes();
        let calls = RefCell::new(Vec::new());
        let storage = Rc::new(RecordingStorage {
            bytes: input,
            calls: &calls,
        });
        let value = StringBytes::try_borrow_str(&storage).unwrap();
        assert_eq!(*calls.borrow(), ["try_borrow_str"]);
        assert_eq!(value, "étext");
    }

    #[test]
    fn try_borrow_str_shared_rc_forwards_text_override() {
        let input = "étext".as_bytes();
        let calls = RefCell::new(Vec::new());
        let owner = Rc::new(RecordingStorage {
            bytes: input,
            calls: &calls,
        });
        let storage = Rc::clone(&owner);
        let value = StringBytes::try_borrow_str(&storage).unwrap();
        assert_eq!(*calls.borrow(), ["try_borrow_str"]);
        assert_eq!(value, "étext");
    }

    #[test]
    #[expect(
        clippy::arc_with_non_send_sync,
        reason = "This test exercises Arc on one thread; its call log need not be Sync."
    )]
    fn try_borrow_str_nested_forwards_text_override() {
        let input = "étext".as_bytes();
        let calls = RefCell::new(Vec::new());
        let storage = Rc::new(Arc::new(Box::new(RecordingStorage {
            bytes: input,
            calls: &calls,
        })));
        let value = StringBytes::try_borrow_str(&storage).unwrap();
        assert_eq!(*calls.borrow(), ["try_borrow_str"]);
        assert_eq!(value, "étext");
    }

    #[test]
    #[expect(
        clippy::arc_with_non_send_sync,
        reason = "This test exercises Arc on one thread; its call log need not be Sync."
    )]
    fn try_borrow_str_shared_nested_forwards_text_override() {
        let input = "étext".as_bytes();
        let calls = RefCell::new(Vec::new());
        let owner = Rc::new(Arc::new(Box::new(RecordingStorage {
            bytes: input,
            calls: &calls,
        })));
        let storage = Rc::clone(&owner);
        let value = StringBytes::try_borrow_str(&storage).unwrap();
        assert_eq!(*calls.borrow(), ["try_borrow_str"]);
        assert_eq!(value, "étext");
    }

    #[test]
    fn borrow_str_unchecked_rc_vec_borrows_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let storage = Rc::new(source);
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
    fn try_to_string_rc_vec_copies_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let storage = Rc::new(source);
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
    fn try_to_string_rc_vec_reports_invalid_utf8_without_changing_storage() {
        for (input, valid_up_to, error_len) in [
            (b"ok\xff".as_slice(), 2, Some(1)),
            (b"ok\xe2\x82", 2, None),
            (b"\xc0\x80", 0, Some(1)),
            (b"\xed\xa0\x80", 0, Some(1)),
        ] {
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let storage = Rc::new(source);
            let error = StringBytes::try_to_string(&storage).unwrap_err();
            assert_eq!(error.valid_up_to(), valid_up_to);
            assert_eq!(error.error_len(), error_len);
            assert_eq!(StringBytes::as_bytes(&storage), input);
        }
    }

    #[test]
    fn try_to_string_rc_calls_only_inner_override() {
        let input = "étext".as_bytes();
        let calls = RefCell::new(Vec::new());
        let storage = Rc::new(RecordingStorage {
            bytes: input,
            calls: &calls,
        });
        let value = StringBytes::try_to_string(&storage).unwrap();
        assert_eq!(*calls.borrow(), ["try_to_string"]);
        assert_eq!(value, "étext");
    }

    #[test]
    fn try_to_string_shared_rc_calls_only_inner_override() {
        let input = "étext".as_bytes();
        let calls = RefCell::new(Vec::new());
        let owner = Rc::new(RecordingStorage {
            bytes: input,
            calls: &calls,
        });
        let storage = Rc::clone(&owner);
        let value = StringBytes::try_to_string(&storage).unwrap();
        assert_eq!(*calls.borrow(), ["try_to_string"]);
        assert_eq!(value, "étext");
    }

    #[test]
    #[expect(
        clippy::arc_with_non_send_sync,
        reason = "This test exercises Arc on one thread; its call log need not be Sync."
    )]
    fn try_to_string_nested_calls_only_inner_override() {
        let input = "étext".as_bytes();
        let calls = RefCell::new(Vec::new());
        let storage = Rc::new(Arc::new(Box::new(RecordingStorage {
            bytes: input,
            calls: &calls,
        })));
        let value = StringBytes::try_to_string(&storage).unwrap();
        assert_eq!(*calls.borrow(), ["try_to_string"]);
        assert_eq!(value, "étext");
    }

    #[test]
    #[expect(
        clippy::arc_with_non_send_sync,
        reason = "This test exercises Arc on one thread; its call log need not be Sync."
    )]
    fn try_to_string_shared_nested_calls_only_inner_override() {
        let input = "étext".as_bytes();
        let calls = RefCell::new(Vec::new());
        let owner = Rc::new(Arc::new(Box::new(RecordingStorage {
            bytes: input,
            calls: &calls,
        })));
        let storage = Rc::clone(&owner);
        let value = StringBytes::try_to_string(&storage).unwrap();
        assert_eq!(*calls.borrow(), ["try_to_string"]);
        assert_eq!(value, "étext");
    }

    #[test]
    fn to_string_lossy_rc_vec_borrows_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let storage = Rc::new(source);
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
    fn to_string_lossy_rc_vec_owns_replacement_text() {
        for (input, expected) in [
            (b"a\xffb".as_slice(), "a�b"),
            (b"ok\xe2\x82", "ok�"),
            (b"\xc0\x80", "��"),
            (b"\xed\xa0\x80", "���"),
        ] {
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let storage = Rc::new(source);
            let value = StringBytes::to_string_lossy(&storage);
            assert!(matches!(value, Cow::Owned(_)));
            assert_eq!(value, expected);
            assert_eq!(StringBytes::as_bytes(&storage), input);
        }
    }

    #[test]
    fn to_string_lossy_rc_forwards_text_override() {
        let input = "étext".as_bytes();
        let calls = RefCell::new(Vec::new());
        let storage = Rc::new(RecordingStorage {
            bytes: input,
            calls: &calls,
        });
        let value = StringBytes::to_string_lossy(&storage);
        assert_eq!(*calls.borrow(), ["to_string_lossy"]);
        assert_eq!(value, "étext");
    }

    #[test]
    fn to_string_lossy_shared_rc_forwards_text_override() {
        let input = "étext".as_bytes();
        let calls = RefCell::new(Vec::new());
        let owner = Rc::new(RecordingStorage {
            bytes: input,
            calls: &calls,
        });
        let storage = Rc::clone(&owner);
        let value = StringBytes::to_string_lossy(&storage);
        assert_eq!(*calls.borrow(), ["to_string_lossy"]);
        assert_eq!(value, "étext");
    }

    #[test]
    #[expect(
        clippy::arc_with_non_send_sync,
        reason = "This test exercises Arc on one thread; its call log need not be Sync."
    )]
    fn to_string_lossy_nested_forwards_text_override() {
        let input = "étext".as_bytes();
        let calls = RefCell::new(Vec::new());
        let storage = Rc::new(Arc::new(Box::new(RecordingStorage {
            bytes: input,
            calls: &calls,
        })));
        let value = StringBytes::to_string_lossy(&storage);
        assert_eq!(*calls.borrow(), ["to_string_lossy"]);
        assert_eq!(value, "étext");
    }

    #[test]
    #[expect(
        clippy::arc_with_non_send_sync,
        reason = "This test exercises Arc on one thread; its call log need not be Sync."
    )]
    fn to_string_lossy_shared_nested_forwards_text_override() {
        let input = "étext".as_bytes();
        let calls = RefCell::new(Vec::new());
        let owner = Rc::new(Arc::new(Box::new(RecordingStorage {
            bytes: input,
            calls: &calls,
        })));
        let storage = Rc::clone(&owner);
        let value = StringBytes::to_string_lossy(&storage);
        assert_eq!(*calls.borrow(), ["to_string_lossy"]);
        assert_eq!(value, "étext");
    }

    #[test]
    fn to_string_unchecked_rc_vec_copies_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let storage = Rc::new(source);
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
    fn try_into_string_rc_vec_reuses_buffer() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let source_capacity = source.capacity();
            let storage = Rc::new(source);
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
    fn try_into_string_rc_vec_error_keeps_buffer() {
        for (input, valid_up_to, error_len) in [
            (b"ok\xff".as_slice(), 2, Some(1)),
            (b"ok\xe2\x82", 2, None),
            (b"\xc0\x80", 0, Some(1)),
            (b"\xed\xa0\x80", 0, Some(1)),
        ] {
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let source_capacity = source.capacity();
            let storage = Rc::new(source);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let capacity = source_capacity;
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
    fn try_into_string_rc_vec_reuses_buffer_with_weak_reference() {
        let mut source = Vec::with_capacity(32);
        source.extend_from_slice(b"text");
        let capacity = source.capacity();
        let storage = Rc::new(source);
        let weak = Rc::downgrade(&storage);
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let value = StringBytes::try_into_string(storage).unwrap();
        assert_eq!(value, "text");
        if capacity != 0 {
            assert_eq!(value.as_ptr(), address);
        }
        assert_eq!(value.capacity(), capacity);
        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn try_into_string_rc_vec_copies_with_another_strong_owner() {
        let owner = Rc::new(Vec::from(b"text".as_slice()));
        let storage = Rc::clone(&owner);
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let value = StringBytes::try_into_string(storage).unwrap();
        assert_eq!(value, "text");
        assert_ne!(value.as_ptr(), address);
        assert_eq!(StringBytes::as_bytes(&owner), b"text");
    }

    #[test]
    fn try_into_string_nested_copies_with_another_strong_owner() {
        let owner = Rc::new(Arc::new(Box::new(Vec::from(b"text".as_slice()))));
        let storage = Rc::clone(&owner);
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let value = StringBytes::try_into_string(storage).unwrap();
        assert_eq!(value, "text");
        assert_ne!(value.as_ptr(), address);
        assert_eq!(StringBytes::as_bytes(&owner), b"text");
    }

    #[test]
    fn try_into_string_rc_slice_unique_copies() {
        let storage = Rc::<[u8]>::from(b"text".as_slice());
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let value = StringBytes::try_into_string(storage).unwrap();
        assert_eq!(value, "text");
        assert_ne!(value.as_ptr(), address);
    }

    #[test]
    fn try_into_string_rc_slice_shared_copies() {
        let storage = Rc::<[u8]>::from(b"text".as_slice());
        let other = Rc::clone(&storage);
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let value = StringBytes::try_into_string(storage).unwrap();
        assert_eq!(value, "text");
        assert_ne!(value.as_ptr(), address);
        assert_eq!(StringBytes::as_bytes(&other), b"text");
    }

    #[test]
    fn try_into_string_rc_str_unique_copies() {
        let storage = Rc::<str>::from("text");
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let value = StringBytes::try_into_string(storage).unwrap();
        assert_eq!(value, "text");
        assert_ne!(value.as_ptr(), address);
    }

    #[test]
    fn try_into_string_rc_str_shared_copies() {
        let storage = Rc::<str>::from("text");
        let other = Rc::clone(&storage);
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let value = StringBytes::try_into_string(storage).unwrap();
        assert_eq!(value, "text");
        assert_ne!(value.as_ptr(), address);
        assert_eq!(StringBytes::as_bytes(&other), b"text");
    }

    #[test]
    fn try_into_string_rc_c_str_unique_copies() {
        let storage = Rc::<CStr>::from(c"text");
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let value = StringBytes::try_into_string(storage).unwrap();
        assert_eq!(value, "text");
        assert_ne!(value.as_ptr(), address);
    }

    #[test]
    fn try_into_string_rc_c_str_shared_copies() {
        let storage = Rc::<CStr>::from(c"text");
        let other = Rc::clone(&storage);
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let value = StringBytes::try_into_string(storage).unwrap();
        assert_eq!(value, "text");
        assert_ne!(value.as_ptr(), address);
        assert_eq!(StringBytes::as_bytes(&other), b"text");
    }

    #[test]
    fn try_into_string_rc_vec_error_keeps_buffer_with_weak_reference() {
        let mut source = Vec::with_capacity(32);
        source.extend_from_slice(b"ok\xff");
        let capacity = source.capacity();
        let storage = Rc::new(source);
        let weak = Rc::downgrade(&storage);
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let storage_address = std::ptr::from_ref(&*storage);
        let error = StringBytes::try_into_string(storage).unwrap_err();
        assert_eq!(error.utf8_error().valid_up_to(), 2);
        assert_eq!(error.utf8_error().error_len(), Some(1));
        let value = error.into_inner();
        assert_eq!(std::ptr::from_ref(&*value), storage_address);
        assert_eq!(StringBytes::as_bytes(&value), b"ok\xff");
        if capacity != 0 {
            assert_eq!(StringBytes::as_bytes(&value).as_ptr(), address);
        }
        assert_eq!(value.capacity(), capacity);
        assert!(Rc::ptr_eq(&weak.upgrade().unwrap(), &value));
        assert_eq!(Rc::strong_count(&value), 1);
    }

    #[test]
    fn try_into_string_rc_vec_error_preserves_shared_ownership() {
        let owner = Rc::new(Vec::from(b"ok\xff".as_slice()));
        let storage = Rc::clone(&owner);
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let storage_address = std::ptr::from_ref(&*storage);
        let error = StringBytes::try_into_string(storage).unwrap_err();
        assert_eq!(error.utf8_error().valid_up_to(), 2);
        assert_eq!(error.utf8_error().error_len(), Some(1));
        let value = error.into_inner();
        assert!(Rc::ptr_eq(&value, &owner));
        assert_eq!(Rc::strong_count(&value), 2);
        assert_eq!(std::ptr::from_ref(&*value), storage_address);
        assert_eq!(StringBytes::as_bytes(&value), b"ok\xff");
        assert_eq!(StringBytes::as_bytes(&value).as_ptr(), address);
        assert_eq!(StringBytes::as_bytes(&owner), b"ok\xff");
    }

    #[test]
    fn try_into_string_nested_error_preserves_shared_ownership() {
        let owner = Rc::new(Arc::new(Box::new(Vec::from(b"ok\xff".as_slice()))));
        let storage = Rc::clone(&owner);
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let storage_address = std::ptr::from_ref(&*storage);
        let error = StringBytes::try_into_string(storage).unwrap_err();
        assert_eq!(error.utf8_error().valid_up_to(), 2);
        assert_eq!(error.utf8_error().error_len(), Some(1));
        let value = error.into_inner();
        assert!(Rc::ptr_eq(&value, &owner));
        assert_eq!(Rc::strong_count(&value), 2);
        assert_eq!(std::ptr::from_ref(&*value), storage_address);
        assert_eq!(StringBytes::as_bytes(&value), b"ok\xff");
        assert_eq!(StringBytes::as_bytes(&value).as_ptr(), address);
        assert_eq!(StringBytes::as_bytes(&owner), b"ok\xff");
    }

    #[test]
    fn try_into_string_rc_slice_error_preserves_unique_pointer() {
        let storage = Rc::<[u8]>::from(b"ok\xff".as_slice());
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let storage_address = std::ptr::from_ref(&*storage);
        let error = StringBytes::try_into_string(storage).unwrap_err();
        assert_eq!(error.utf8_error().valid_up_to(), 2);
        assert_eq!(error.utf8_error().error_len(), Some(1));
        let value = error.into_inner();
        assert_eq!(std::ptr::from_ref(&*value), storage_address);
        assert_eq!(StringBytes::as_bytes(&value), b"ok\xff");
        assert_eq!(StringBytes::as_bytes(&value).as_ptr(), address);
    }

    #[test]
    fn try_into_string_rc_slice_error_preserves_shared_pointer() {
        let storage = Rc::<[u8]>::from(b"ok\xff".as_slice());
        let other = Rc::clone(&storage);
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let storage_address = std::ptr::from_ref(&*storage);
        let error = StringBytes::try_into_string(storage).unwrap_err();
        assert_eq!(error.utf8_error().valid_up_to(), 2);
        assert_eq!(error.utf8_error().error_len(), Some(1));
        let value = error.into_inner();
        assert!(Rc::ptr_eq(&value, &other));
        assert_eq!(Rc::strong_count(&value), 2);
        assert_eq!(std::ptr::from_ref(&*value), storage_address);
        assert_eq!(StringBytes::as_bytes(&value), b"ok\xff");
        assert_eq!(StringBytes::as_bytes(&value).as_ptr(), address);
        assert_eq!(StringBytes::as_bytes(&other), b"ok\xff");
    }

    #[test]
    fn try_into_string_rc_c_str_error_preserves_unique_pointer() {
        let storage = Rc::<CStr>::from(c"ok\xff");
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let storage_address = std::ptr::from_ref(&*storage);
        let error = StringBytes::try_into_string(storage).unwrap_err();
        assert_eq!(error.utf8_error().valid_up_to(), 2);
        assert_eq!(error.utf8_error().error_len(), Some(1));
        let value = error.into_inner();
        assert_eq!(value.to_bytes_with_nul().last(), Some(&0));
        assert_eq!(
            value.to_bytes_with_nul().len(),
            StringBytes::as_bytes(&value).len() + 1
        );
        assert_eq!(std::ptr::from_ref(&*value), storage_address);
        assert_eq!(StringBytes::as_bytes(&value), b"ok\xff");
        assert_eq!(StringBytes::as_bytes(&value).as_ptr(), address);
    }

    #[test]
    fn try_into_string_rc_c_str_error_preserves_shared_pointer() {
        let storage = Rc::<CStr>::from(c"ok\xff");
        let other = Rc::clone(&storage);
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let storage_address = std::ptr::from_ref(&*storage);
        let error = StringBytes::try_into_string(storage).unwrap_err();
        assert_eq!(error.utf8_error().valid_up_to(), 2);
        assert_eq!(error.utf8_error().error_len(), Some(1));
        let value = error.into_inner();
        assert_eq!(value.to_bytes_with_nul().last(), Some(&0));
        assert_eq!(
            value.to_bytes_with_nul().len(),
            StringBytes::as_bytes(&value).len() + 1
        );
        assert!(Rc::ptr_eq(&value, &other));
        assert_eq!(Rc::strong_count(&value), 2);
        assert_eq!(std::ptr::from_ref(&*value), storage_address);
        assert_eq!(StringBytes::as_bytes(&value), b"ok\xff");
        assert_eq!(StringBytes::as_bytes(&value).as_ptr(), address);
        assert_eq!(StringBytes::as_bytes(&other), b"ok\xff");
    }

    #[test]
    fn try_into_string_rc_forwards_text_override() {
        let input = "étext".as_bytes();
        let calls = RefCell::new(Vec::new());
        let storage = Rc::new(RecordingStorage {
            bytes: input,
            calls: &calls,
        });
        let value = StringBytes::try_into_string(storage).unwrap();
        assert_eq!(*calls.borrow(), ["try_borrow_str", "into_bytes"]);
        assert_eq!(value, "étext");
    }

    #[test]
    fn try_into_string_rc_returns_original_without_byte_conversion() {
        let input = b"ok\xff";
        let calls = RefCell::new(Vec::new());
        let storage = Rc::new(RecordingStorage {
            bytes: input,
            calls: &calls,
        });
        let address = input.as_ptr();
        let error = StringBytes::try_into_string(storage).unwrap_err();
        assert_eq!(*calls.borrow(), ["try_borrow_str"]);
        assert_eq!(error.utf8_error().valid_up_to(), 2);
        let recovered = error.into_inner();
        assert_eq!(StringBytes::as_bytes(&recovered), b"ok\xff");
        assert_eq!(StringBytes::as_bytes(&recovered).as_ptr(), address);
    }

    #[test]
    fn try_into_string_shared_rc_forwards_text_override() {
        let input = "étext".as_bytes();
        let calls = RefCell::new(Vec::new());
        let owner = Rc::new(RecordingStorage {
            bytes: input,
            calls: &calls,
        });
        let storage = Rc::clone(&owner);
        let value = StringBytes::try_into_string(storage).unwrap();
        assert_eq!(*calls.borrow(), ["try_borrow_str", "to_bytes"]);
        assert_eq!(value, "étext");
    }

    #[test]
    fn try_into_string_shared_rc_returns_original_without_byte_conversion() {
        let input = b"ok\xff";
        let calls = RefCell::new(Vec::new());
        let owner = Rc::new(RecordingStorage {
            bytes: input,
            calls: &calls,
        });
        let storage = Rc::clone(&owner);
        let address = input.as_ptr();
        let error = StringBytes::try_into_string(storage).unwrap_err();
        assert_eq!(*calls.borrow(), ["try_borrow_str"]);
        assert_eq!(error.utf8_error().valid_up_to(), 2);
        let recovered = error.into_inner();
        assert_eq!(StringBytes::as_bytes(&recovered), b"ok\xff");
        assert_eq!(StringBytes::as_bytes(&recovered).as_ptr(), address);
    }

    #[test]
    #[expect(
        clippy::arc_with_non_send_sync,
        reason = "This test exercises Arc on one thread; its call log need not be Sync."
    )]
    fn try_into_string_nested_forwards_text_override() {
        let input = "étext".as_bytes();
        let calls = RefCell::new(Vec::new());
        let storage = Rc::new(Arc::new(Box::new(RecordingStorage {
            bytes: input,
            calls: &calls,
        })));
        let value = StringBytes::try_into_string(storage).unwrap();
        assert_eq!(*calls.borrow(), ["try_borrow_str", "into_bytes"]);
        assert_eq!(value, "étext");
    }

    #[test]
    #[expect(
        clippy::arc_with_non_send_sync,
        reason = "This test exercises Arc on one thread; its call log need not be Sync."
    )]
    fn try_into_string_nested_returns_original_without_byte_conversion() {
        let input = b"ok\xff";
        let calls = RefCell::new(Vec::new());
        let storage = Rc::new(Arc::new(Box::new(RecordingStorage {
            bytes: input,
            calls: &calls,
        })));
        let address = input.as_ptr();
        let error = StringBytes::try_into_string(storage).unwrap_err();
        assert_eq!(*calls.borrow(), ["try_borrow_str"]);
        assert_eq!(error.utf8_error().valid_up_to(), 2);
        let recovered = error.into_inner();
        assert_eq!(StringBytes::as_bytes(&recovered), b"ok\xff");
        assert_eq!(StringBytes::as_bytes(&recovered).as_ptr(), address);
    }

    #[test]
    #[expect(
        clippy::arc_with_non_send_sync,
        reason = "This test exercises Arc on one thread; its call log need not be Sync."
    )]
    fn try_into_string_shared_nested_forwards_text_override() {
        let input = "étext".as_bytes();
        let calls = RefCell::new(Vec::new());
        let owner = Rc::new(Arc::new(Box::new(RecordingStorage {
            bytes: input,
            calls: &calls,
        })));
        let storage = Rc::clone(&owner);
        let value = StringBytes::try_into_string(storage).unwrap();
        assert_eq!(*calls.borrow(), ["try_borrow_str", "to_bytes"]);
        assert_eq!(value, "étext");
    }

    #[test]
    #[expect(
        clippy::arc_with_non_send_sync,
        reason = "This test exercises Arc on one thread; its call log need not be Sync."
    )]
    fn try_into_string_shared_nested_returns_original_without_byte_conversion() {
        let input = b"ok\xff";
        let calls = RefCell::new(Vec::new());
        let owner = Rc::new(Arc::new(Box::new(RecordingStorage {
            bytes: input,
            calls: &calls,
        })));
        let storage = Rc::clone(&owner);
        let address = input.as_ptr();
        let error = StringBytes::try_into_string(storage).unwrap_err();
        assert_eq!(*calls.borrow(), ["try_borrow_str"]);
        assert_eq!(error.utf8_error().valid_up_to(), 2);
        let recovered = error.into_inner();
        assert_eq!(StringBytes::as_bytes(&recovered), b"ok\xff");
        assert_eq!(StringBytes::as_bytes(&recovered).as_ptr(), address);
    }

    #[test]
    fn try_into_string_rc_io_slice_copies_current_view() {
        let mut view = IoSlice::new(b"prefixtext");
        view.advance(6);
        let storage = Rc::new(view);
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let value = StringBytes::try_into_string(storage).unwrap();
        assert_eq!(value, "text");
        assert_ne!(value.as_ptr(), address);
    }

    #[test]
    fn try_into_string_rc_io_slice_mut_copies_current_view() {
        let mut source = *b"prefixtext";
        let mut view = IoSliceMut::new(&mut source);
        view.advance(6);
        let storage = Rc::new(view);
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let value = StringBytes::try_into_string(storage).unwrap();
        assert_eq!(value, "text");
        assert_ne!(value.as_ptr(), address);
        assert_eq!(&source, b"prefixtext");
    }

    #[test]
    fn try_into_string_rc_c_string_reuses_buffer_without_nul() {
        let storage = Rc::new(std::ffi::CString::new(b"text".as_slice()).unwrap());
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
    fn try_into_string_rc_c_string_error_preserves_c_string() {
        let storage = Rc::new(std::ffi::CString::new(b"ok\xff".as_slice()).unwrap());
        let address = StringBytes::as_bytes(&storage).as_ptr();

        let storage_address = std::ptr::from_ref(&*storage);
        let error = StringBytes::try_into_string(storage).unwrap_err();
        assert_eq!(error.utf8_error().valid_up_to(), 2);
        assert_eq!(error.utf8_error().error_len(), Some(1));
        let value = error.into_inner();
        assert_eq!(value.as_bytes_with_nul().last(), Some(&0));
        assert_eq!(
            value.as_bytes_with_nul().len(),
            StringBytes::as_bytes(&value).len() + 1
        );
        assert_eq!(std::ptr::from_ref(&*value), storage_address);
        assert_eq!(StringBytes::as_bytes(&value), b"ok\xff");
        assert_eq!(StringBytes::as_bytes(&value).as_ptr(), address);
    }

    #[test]
    fn into_string_lossy_rc_vec_valid_utf8_reuses_buffer() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let source_capacity = source.capacity();
            let storage = Rc::new(source);
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
    fn into_string_lossy_rc_vec_replaces_invalid_utf8() {
        for (input, expected) in [
            (b"a\xffb".as_slice(), "a�b"),
            (b"ok\xe2\x82", "ok�"),
            (b"\xc0\x80", "��"),
            (b"\xed\xa0\x80", "���"),
        ] {
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let storage = Rc::new(source);
            let value = StringBytes::into_string_lossy(storage);
            assert_eq!(value, expected);
        }
    }

    #[test]
    fn into_string_lossy_rc_vec_reuses_buffer_with_weak_reference() {
        let mut source = Vec::with_capacity(32);
        source.extend_from_slice(b"text");
        let capacity = source.capacity();
        let storage = Rc::new(source);
        let weak = Rc::downgrade(&storage);
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let value = StringBytes::into_string_lossy(storage);
        assert_eq!(value, "text");
        if capacity != 0 {
            assert_eq!(value.as_ptr(), address);
        }
        assert_eq!(value.capacity(), capacity);
        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn into_string_lossy_rc_vec_copies_with_another_strong_owner() {
        let owner = Rc::new(Vec::from(b"text".as_slice()));
        let storage = Rc::clone(&owner);
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let value = StringBytes::into_string_lossy(storage);
        assert_eq!(value, "text");
        assert_ne!(value.as_ptr(), address);
        assert_eq!(StringBytes::as_bytes(&owner), b"text");
    }

    #[test]
    fn into_string_lossy_nested_copies_with_another_strong_owner() {
        let owner = Rc::new(Arc::new(Box::new(Vec::from(b"text".as_slice()))));
        let storage = Rc::clone(&owner);
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let value = StringBytes::into_string_lossy(storage);
        assert_eq!(value, "text");
        assert_ne!(value.as_ptr(), address);
        assert_eq!(StringBytes::as_bytes(&owner), b"text");
    }

    #[test]
    fn into_string_lossy_rc_slice_unique_copies() {
        let storage = Rc::<[u8]>::from(b"text".as_slice());
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let value = StringBytes::into_string_lossy(storage);
        assert_eq!(value, "text");
        assert_ne!(value.as_ptr(), address);
    }

    #[test]
    fn into_string_lossy_rc_slice_shared_copies() {
        let storage = Rc::<[u8]>::from(b"text".as_slice());
        let other = Rc::clone(&storage);
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let value = StringBytes::into_string_lossy(storage);
        assert_eq!(value, "text");
        assert_ne!(value.as_ptr(), address);
        assert_eq!(StringBytes::as_bytes(&other), b"text");
    }

    #[test]
    fn into_string_lossy_rc_str_unique_copies() {
        let storage = Rc::<str>::from("text");
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let value = StringBytes::into_string_lossy(storage);
        assert_eq!(value, "text");
        assert_ne!(value.as_ptr(), address);
    }

    #[test]
    fn into_string_lossy_rc_str_shared_copies() {
        let storage = Rc::<str>::from("text");
        let other = Rc::clone(&storage);
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let value = StringBytes::into_string_lossy(storage);
        assert_eq!(value, "text");
        assert_ne!(value.as_ptr(), address);
        assert_eq!(StringBytes::as_bytes(&other), b"text");
    }

    #[test]
    fn into_string_lossy_rc_c_str_unique_copies() {
        let storage = Rc::<CStr>::from(c"text");
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let value = StringBytes::into_string_lossy(storage);
        assert_eq!(value, "text");
        assert_ne!(value.as_ptr(), address);
    }

    #[test]
    fn into_string_lossy_rc_c_str_shared_copies() {
        let storage = Rc::<CStr>::from(c"text");
        let other = Rc::clone(&storage);
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let value = StringBytes::into_string_lossy(storage);
        assert_eq!(value, "text");
        assert_ne!(value.as_ptr(), address);
        assert_eq!(StringBytes::as_bytes(&other), b"text");
    }

    #[test]
    fn into_string_lossy_rc_forwards_text_override() {
        let input = "étext".as_bytes();
        let calls = RefCell::new(Vec::new());
        let storage = Rc::new(RecordingStorage {
            bytes: input,
            calls: &calls,
        });
        let value = StringBytes::into_string_lossy(storage);
        assert_eq!(*calls.borrow(), ["to_string_lossy", "into_bytes"]);
        assert_eq!(value, "étext");
    }

    #[test]
    fn into_string_lossy_rc_does_not_make_an_intermediate_copy() {
        let input = b"a\xffb";
        let calls = RefCell::new(Vec::new());
        let storage = Rc::new(RecordingStorage {
            bytes: input,
            calls: &calls,
        });
        assert_eq!(StringBytes::into_string_lossy(storage), "a�b");
        assert_eq!(*calls.borrow(), ["to_string_lossy"]);
    }

    #[test]
    fn into_string_lossy_shared_rc_forwards_text_override() {
        let input = "étext".as_bytes();
        let calls = RefCell::new(Vec::new());
        let owner = Rc::new(RecordingStorage {
            bytes: input,
            calls: &calls,
        });
        let storage = Rc::clone(&owner);
        let value = StringBytes::into_string_lossy(storage);
        assert_eq!(*calls.borrow(), ["to_string_lossy", "to_bytes"]);
        assert_eq!(value, "étext");
    }

    #[test]
    fn into_string_lossy_shared_rc_does_not_make_an_intermediate_copy() {
        let input = b"a\xffb";
        let calls = RefCell::new(Vec::new());
        let owner = Rc::new(RecordingStorage {
            bytes: input,
            calls: &calls,
        });
        let storage = Rc::clone(&owner);
        assert_eq!(StringBytes::into_string_lossy(storage), "a�b");
        assert_eq!(*calls.borrow(), ["to_string_lossy"]);
    }

    #[test]
    #[expect(
        clippy::arc_with_non_send_sync,
        reason = "This test exercises Arc on one thread; its call log need not be Sync."
    )]
    fn into_string_lossy_nested_forwards_text_override() {
        let input = "étext".as_bytes();
        let calls = RefCell::new(Vec::new());
        let storage = Rc::new(Arc::new(Box::new(RecordingStorage {
            bytes: input,
            calls: &calls,
        })));
        let value = StringBytes::into_string_lossy(storage);
        assert_eq!(*calls.borrow(), ["to_string_lossy", "into_bytes"]);
        assert_eq!(value, "étext");
    }

    #[test]
    #[expect(
        clippy::arc_with_non_send_sync,
        reason = "This test exercises Arc on one thread; its call log need not be Sync."
    )]
    fn into_string_lossy_nested_does_not_make_an_intermediate_copy() {
        let input = b"a\xffb";
        let calls = RefCell::new(Vec::new());
        let storage = Rc::new(Arc::new(Box::new(RecordingStorage {
            bytes: input,
            calls: &calls,
        })));
        assert_eq!(StringBytes::into_string_lossy(storage), "a�b");
        assert_eq!(*calls.borrow(), ["to_string_lossy"]);
    }

    #[test]
    #[expect(
        clippy::arc_with_non_send_sync,
        reason = "This test exercises Arc on one thread; its call log need not be Sync."
    )]
    fn into_string_lossy_shared_nested_forwards_text_override() {
        let input = "étext".as_bytes();
        let calls = RefCell::new(Vec::new());
        let owner = Rc::new(Arc::new(Box::new(RecordingStorage {
            bytes: input,
            calls: &calls,
        })));
        let storage = Rc::clone(&owner);
        let value = StringBytes::into_string_lossy(storage);
        assert_eq!(*calls.borrow(), ["to_string_lossy", "to_bytes"]);
        assert_eq!(value, "étext");
    }

    #[test]
    #[expect(
        clippy::arc_with_non_send_sync,
        reason = "This test exercises Arc on one thread; its call log need not be Sync."
    )]
    fn into_string_lossy_shared_nested_does_not_make_an_intermediate_copy() {
        let input = b"a\xffb";
        let calls = RefCell::new(Vec::new());
        let owner = Rc::new(Arc::new(Box::new(RecordingStorage {
            bytes: input,
            calls: &calls,
        })));
        let storage = Rc::clone(&owner);
        assert_eq!(StringBytes::into_string_lossy(storage), "a�b");
        assert_eq!(*calls.borrow(), ["to_string_lossy"]);
    }

    #[test]
    fn into_string_lossy_rc_c_string_reuses_buffer_without_nul() {
        let storage = Rc::new(std::ffi::CString::new(b"text".as_slice()).unwrap());
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
    fn into_string_unchecked_rc_vec_reuses_buffer() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let source_capacity = source.capacity();
            let storage = Rc::new(source);
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
    fn into_string_unchecked_rc_vec_reuses_buffer_with_weak_reference() {
        let mut source = Vec::with_capacity(32);
        source.extend_from_slice(b"text");
        let capacity = source.capacity();
        let storage = Rc::new(source);
        let weak = Rc::downgrade(&storage);
        let address = StringBytes::as_bytes(&storage).as_ptr();
        // SAFETY: the storage contains the valid UTF-8 literal above.
        let value = unsafe { StringBytes::into_string_unchecked(storage) };
        assert_eq!(value, "text");
        if capacity != 0 {
            assert_eq!(value.as_ptr(), address);
        }
        assert_eq!(value.capacity(), capacity);
        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn into_string_unchecked_rc_vec_copies_with_another_strong_owner() {
        let owner = Rc::new(Vec::from(b"text".as_slice()));
        let storage = Rc::clone(&owner);
        let address = StringBytes::as_bytes(&storage).as_ptr();
        // SAFETY: the storage contains the valid UTF-8 literal above.
        let value = unsafe { StringBytes::into_string_unchecked(storage) };
        assert_eq!(value, "text");
        assert_ne!(value.as_ptr(), address);
        assert_eq!(StringBytes::as_bytes(&owner), b"text");
    }

    #[test]
    fn into_string_unchecked_nested_copies_with_another_strong_owner() {
        let owner = Rc::new(Arc::new(Box::new(Vec::from(b"text".as_slice()))));
        let storage = Rc::clone(&owner);
        let address = StringBytes::as_bytes(&storage).as_ptr();
        // SAFETY: the storage contains the valid UTF-8 literal above.
        let value = unsafe { StringBytes::into_string_unchecked(storage) };
        assert_eq!(value, "text");
        assert_ne!(value.as_ptr(), address);
        assert_eq!(StringBytes::as_bytes(&owner), b"text");
    }

    #[test]
    fn into_string_unchecked_rc_slice_unique_copies() {
        let storage = Rc::<[u8]>::from(b"text".as_slice());
        let address = StringBytes::as_bytes(&storage).as_ptr();
        // SAFETY: the storage contains the valid UTF-8 literal above.
        let value = unsafe { StringBytes::into_string_unchecked(storage) };
        assert_eq!(value, "text");
        assert_ne!(value.as_ptr(), address);
    }

    #[test]
    fn into_string_unchecked_rc_slice_shared_copies() {
        let storage = Rc::<[u8]>::from(b"text".as_slice());
        let other = Rc::clone(&storage);
        let address = StringBytes::as_bytes(&storage).as_ptr();
        // SAFETY: the storage contains the valid UTF-8 literal above.
        let value = unsafe { StringBytes::into_string_unchecked(storage) };
        assert_eq!(value, "text");
        assert_ne!(value.as_ptr(), address);
        assert_eq!(StringBytes::as_bytes(&other), b"text");
    }

    #[test]
    fn into_string_unchecked_rc_str_unique_copies() {
        let storage = Rc::<str>::from("text");
        let address = StringBytes::as_bytes(&storage).as_ptr();
        // SAFETY: the storage contains the valid UTF-8 literal above.
        let value = unsafe { StringBytes::into_string_unchecked(storage) };
        assert_eq!(value, "text");
        assert_ne!(value.as_ptr(), address);
    }

    #[test]
    fn into_string_unchecked_rc_str_shared_copies() {
        let storage = Rc::<str>::from("text");
        let other = Rc::clone(&storage);
        let address = StringBytes::as_bytes(&storage).as_ptr();
        // SAFETY: the storage contains the valid UTF-8 literal above.
        let value = unsafe { StringBytes::into_string_unchecked(storage) };
        assert_eq!(value, "text");
        assert_ne!(value.as_ptr(), address);
        assert_eq!(StringBytes::as_bytes(&other), b"text");
    }

    #[test]
    fn into_string_unchecked_rc_c_str_unique_copies() {
        let storage = Rc::<CStr>::from(c"text");
        let address = StringBytes::as_bytes(&storage).as_ptr();
        // SAFETY: the storage contains the valid UTF-8 literal above.
        let value = unsafe { StringBytes::into_string_unchecked(storage) };
        assert_eq!(value, "text");
        assert_ne!(value.as_ptr(), address);
    }

    #[test]
    fn into_string_unchecked_rc_c_str_shared_copies() {
        let storage = Rc::<CStr>::from(c"text");
        let other = Rc::clone(&storage);
        let address = StringBytes::as_bytes(&storage).as_ptr();
        // SAFETY: the storage contains the valid UTF-8 literal above.
        let value = unsafe { StringBytes::into_string_unchecked(storage) };
        assert_eq!(value, "text");
        assert_ne!(value.as_ptr(), address);
        assert_eq!(StringBytes::as_bytes(&other), b"text");
    }

    #[test]
    fn into_string_unchecked_rc_c_string_reuses_buffer_without_nul() {
        let storage = Rc::new(std::ffi::CString::new(b"text".as_slice()).unwrap());
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
    fn rc_into_bytes_vec_reuses_buffer() {
        let mut source = Vec::with_capacity(32);
        source.extend_from_slice(b"text");
        let source_capacity = source.capacity();
        let storage = Rc::new(source);
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let capacity = source_capacity;
        let value = StringBytes::rc_into_bytes(storage);
        assert_eq!(value, b"text");
        if capacity != 0 {
            assert_eq!(value.as_ptr(), address);
        }
        assert_eq!(value.capacity(), capacity);
    }

    #[test]
    fn rc_into_string_vec_reuses_buffer() {
        let mut source = Vec::with_capacity(32);
        source.extend_from_slice(b"text");
        let source_capacity = source.capacity();
        let storage = Rc::new(source);
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let capacity = source_capacity;
        let value = StringBytes::rc_into_string(storage).unwrap();
        assert_eq!(value, "text");
        if capacity != 0 {
            assert_eq!(value.as_ptr(), address);
        }
        assert_eq!(value.capacity(), capacity);
    }

    #[test]
    fn rc_into_string_vec_error_keeps_buffer() {
        let mut source = Vec::with_capacity(32);
        source.extend_from_slice(b"ok\xff");
        let source_capacity = source.capacity();
        let storage = Rc::new(source);
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let capacity = source_capacity;
        let storage_address = std::ptr::from_ref(&*storage);
        let error = StringBytes::rc_into_string(storage).unwrap_err();
        assert_eq!(error.utf8_error().valid_up_to(), 2);
        assert_eq!(error.utf8_error().error_len(), Some(1));
        let value = error.into_inner();
        assert_eq!(std::ptr::from_ref(&*value), storage_address);
        assert_eq!(StringBytes::as_bytes(&value), b"ok\xff");
        if capacity != 0 {
            assert_eq!(StringBytes::as_bytes(&value).as_ptr(), address);
        }
        assert_eq!(value.capacity(), capacity);
    }
}
