use std::rc::Rc;
use std::string::FromUtf8Error;
use std::sync::Arc;

use ::bytes::Bytes;

use crate::impls::{impl_pointer_conversions_for_sized, impl_string_bytes};
use crate::{sealed, StringBytes};

impl_string_bytes!(
    /// Borrows the bytes visible through this buffer.
    ///
    /// [`into_bytes`](StringBytes::into_bytes) uses the buffer's conversion to
    /// [`Vec<u8>`]. A uniquely owned, transferable allocation is reused;
    /// static data, shared buffers, and [`Bytes::from_owner`] storage are copied.
    /// Converting a sliced buffer may move its bytes within the allocation.
    ///
    /// Checked and lossy string conversions validate UTF-8.
    Bytes {
        /// Borrows the bytes visible through this buffer.
        #[inline]
        fn as_bytes(&self) -> &[u8] {
            self.as_ref()
        }

        /// Converts the visible bytes using [`Vec::from`].
        ///
        /// Allocation reuse depends on the buffer's ownership and backing storage.
        #[inline]
        fn into_bytes(self) -> Vec<u8> {
            // Let bytes decide whether its backing storage can be reused.
            Vec::from(self)
        }

        impl_pointer_conversions_for_sized!();
    }
);

#[cfg(test)]
mod tests {
    use std::borrow::Cow;
    use std::rc::Rc;
    use std::sync::Arc;

    use ::bytes::{Bytes, BytesMut};

    use crate::StringBytes;

    #[test]
    fn as_bytes_bytes_exposes_stable_contents() {
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
            let storage = Bytes::from(source);
            assert_eq!(StringBytes::as_bytes(&storage), input);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            assert_eq!(StringBytes::as_bytes(&storage), input);
            assert_eq!(StringBytes::as_bytes(&storage).as_ptr(), address);
        }
    }

    #[test]
    fn as_bytes_bytes_matches_as_ref() {
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
            let storage = Bytes::from(source);
            let bytes = StringBytes::as_bytes(&storage);
            let other = AsRef::<[u8]>::as_ref(&storage);
            assert_eq!(bytes, input);
            assert_eq!(other, input);
            assert!(std::ptr::eq(bytes, other));
        }
    }

    #[test]
    fn as_bytes_bytes_matches_deref() {
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
            let storage = Bytes::from(source);
            let bytes = StringBytes::as_bytes(&storage);
            let other = StringBytes::as_bytes(std::ops::Deref::deref(&storage));
            assert_eq!(bytes, input);
            assert_eq!(other, input);
            assert!(std::ptr::eq(bytes, other));
        }
    }

    #[test]
    fn to_bytes_bytes_returns_an_independent_copy() {
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
            let storage = Bytes::from(source);
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
    fn into_bytes_bytes_reuses_buffer() {
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
            let storage = Bytes::from(source);
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
    fn into_bytes_empty_bytes_from_box() {
        let storage = Bytes::from(Box::<[u8]>::from([]));
        let capacity = 0;
        let value = StringBytes::into_bytes(storage);
        assert_eq!(value, b"");
        assert_eq!(value.capacity(), capacity);
    }

    #[test]
    fn into_bytes_bytes_from_box_reuses_allocation() {
        let storage = Bytes::from(Box::<[u8]>::from(b"text".as_slice()));
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let capacity = b"text".len();
        let value = StringBytes::into_bytes(storage);
        assert_eq!(value, b"text");
        if capacity != 0 {
            assert_eq!(value.as_ptr(), address);
        }
        assert_eq!(value.capacity(), capacity);
    }

    #[test]
    fn into_bytes_bytes_from_frozen_bytes_mut_reuses_allocation() {
        let mut source = BytesMut::with_capacity(32);
        source.extend_from_slice(b"text");
        let source_capacity = source.capacity();
        let storage = source.freeze();
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let capacity = source_capacity;
        let value = StringBytes::into_bytes(storage);
        assert_eq!(value, b"text");
        if capacity != 0 {
            assert_eq!(value.as_ptr(), address);
        }
        assert_eq!(value.capacity(), capacity);
    }

    #[test]
    fn into_bytes_static_bytes_copies() {
        let storage = Bytes::from_static(b"text");
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let value = StringBytes::into_bytes(storage);
        assert_eq!(value, b"text");
        assert_ne!(value.as_ptr(), address);
    }

    #[test]
    fn into_bytes_owner_backed_bytes_copies() {
        let storage = Bytes::from_owner(Vec::from(b"text".as_slice()));
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let value = StringBytes::into_bytes(storage);
        assert_eq!(value, b"text");
        assert_ne!(value.as_ptr(), address);
    }

    #[test]
    fn into_bytes_shared_bytes_copies() {
        let owner = Bytes::from(Vec::from(b"text".as_slice()));
        let storage = owner.clone();
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let value = StringBytes::into_bytes(storage);
        assert_eq!(value, b"text");
        assert_ne!(value.as_ptr(), address);
        assert_eq!(&owner[..], b"text");
    }

    #[test]
    fn into_bytes_bytes_compacts_unique_slice_into_original_allocation() {
        let mut original = Vec::with_capacity(64);
        original.extend_from_slice(b"prefix");
        original.extend_from_slice(b"text");
        let address = original.as_ptr();
        let capacity = original.capacity();
        let original = Bytes::from(original);
        let storage = original.slice(6..);
        assert_ne!(storage.as_ptr(), address);
        drop(original);
        let value = StringBytes::into_bytes(storage);
        assert_eq!(value, b"text");
        assert_eq!(value.as_ptr(), address);
        assert_eq!(value.capacity(), capacity);
    }

    #[test]
    fn try_borrow_str_bytes_borrows_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let storage = Bytes::from(source);
            let value = StringBytes::try_borrow_str(&storage).unwrap();
            assert_eq!(value, expected);
            assert!(std::ptr::eq(
                value.as_bytes(),
                StringBytes::as_bytes(&storage)
            ));
        }
    }

    #[test]
    fn try_borrow_str_bytes_reports_invalid_utf8() {
        for (input, valid_up_to, error_len) in [
            (b"ok\xff".as_slice(), 2, Some(1)),
            (b"ok\xe2\x82", 2, None),
            (b"\xc0\x80", 0, Some(1)),
            (b"\xed\xa0\x80", 0, Some(1)),
        ] {
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let storage = Bytes::from(source);
            let error = StringBytes::try_borrow_str(&storage).unwrap_err();
            assert_eq!(error.valid_up_to(), valid_up_to);
            assert_eq!(error.error_len(), error_len);
            assert_eq!(StringBytes::as_bytes(&storage), input);
        }
    }

    #[test]
    fn borrow_str_unchecked_bytes_borrows_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let storage = Bytes::from(source);
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
    fn try_to_string_bytes_copies_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let storage = Bytes::from(source);
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
    fn try_to_string_bytes_returns_a_copy_of_invalid_bytes() {
        for (input, valid_up_to, error_len) in [
            (b"ok\xff".as_slice(), 2, Some(1)),
            (b"ok\xe2\x82", 2, None),
            (b"\xc0\x80", 0, Some(1)),
            (b"\xed\xa0\x80", 0, Some(1)),
        ] {
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let storage = Bytes::from(source);
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
    fn to_string_lossy_bytes_borrows_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let storage = Bytes::from(source);
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
    fn to_string_lossy_bytes_owns_replacement_text() {
        for (input, expected) in [
            (b"a\xffb".as_slice(), "a�b"),
            (b"ok\xe2\x82", "ok�"),
            (b"\xc0\x80", "��"),
            (b"\xed\xa0\x80", "���"),
        ] {
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let storage = Bytes::from(source);
            let value = StringBytes::to_string_lossy(&storage);
            assert!(matches!(value, Cow::Owned(_)));
            assert_eq!(value, expected);
            assert_eq!(StringBytes::as_bytes(&storage), input);
        }
    }

    #[test]
    fn to_string_unchecked_bytes_copies_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let storage = Bytes::from(source);
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
    fn try_into_string_bytes_reuses_buffer() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let source_capacity = source.capacity();
            let storage = Bytes::from(source);
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
    fn try_into_string_bytes_error_reuses_buffer() {
        for (input, valid_up_to, error_len) in [
            (b"ok\xff".as_slice(), 2, Some(1)),
            (b"ok\xe2\x82", 2, None),
            (b"\xc0\x80", 0, Some(1)),
            (b"\xed\xa0\x80", 0, Some(1)),
        ] {
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let source_capacity = source.capacity();
            let storage = Bytes::from(source);
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
    fn try_into_string_empty_bytes_from_box() {
        let storage = Bytes::from(Box::<[u8]>::from([]));
        let capacity = 0;
        let value = StringBytes::try_into_string(storage).unwrap();
        assert_eq!(value, "");
        assert_eq!(value.capacity(), capacity);
    }

    #[test]
    fn try_into_string_bytes_from_box_reuses_allocation() {
        let storage = Bytes::from(Box::<[u8]>::from(b"text".as_slice()));
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let capacity = b"text".len();
        let value = StringBytes::try_into_string(storage).unwrap();
        assert_eq!(value, "text");
        if capacity != 0 {
            assert_eq!(value.as_ptr(), address);
        }
        assert_eq!(value.capacity(), capacity);
    }

    #[test]
    fn try_into_string_bytes_from_frozen_bytes_mut_reuses_allocation() {
        let mut source = BytesMut::with_capacity(32);
        source.extend_from_slice(b"text");
        let source_capacity = source.capacity();
        let storage = source.freeze();
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let capacity = source_capacity;
        let value = StringBytes::try_into_string(storage).unwrap();
        assert_eq!(value, "text");
        if capacity != 0 {
            assert_eq!(value.as_ptr(), address);
        }
        assert_eq!(value.capacity(), capacity);
    }

    #[test]
    fn try_into_string_static_bytes_copies() {
        let storage = Bytes::from_static(b"text");
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let value = StringBytes::try_into_string(storage).unwrap();
        assert_eq!(value, "text");
        assert_ne!(value.as_ptr(), address);
    }

    #[test]
    fn try_into_string_owner_backed_bytes_copies() {
        let storage = Bytes::from_owner(Vec::from(b"text".as_slice()));
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let value = StringBytes::try_into_string(storage).unwrap();
        assert_eq!(value, "text");
        assert_ne!(value.as_ptr(), address);
    }

    #[test]
    fn try_into_string_shared_bytes_copies() {
        let owner = Bytes::from(Vec::from(b"text".as_slice()));
        let storage = owner.clone();
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let value = StringBytes::try_into_string(storage).unwrap();
        assert_eq!(value, "text");
        assert_ne!(value.as_ptr(), address);
        assert_eq!(&owner[..], b"text");
    }

    #[test]
    fn try_into_string_bytes_compacts_unique_slice_into_original_allocation() {
        let mut original = Vec::with_capacity(64);
        original.extend_from_slice(b"prefix");
        original.extend_from_slice(b"text");
        let address = original.as_ptr();
        let capacity = original.capacity();
        let original = Bytes::from(original);
        let storage = original.slice(6..);
        assert_ne!(storage.as_ptr(), address);
        drop(original);
        let value = StringBytes::try_into_string(storage).unwrap();
        assert_eq!(value, "text");
        assert_eq!(value.as_ptr(), address);
        assert_eq!(value.capacity(), capacity);
    }

    #[test]
    fn try_into_string_bytes_from_box_error_reuses_allocation() {
        let storage = Bytes::from(Box::<[u8]>::from(b"ok\xff".as_slice()));
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let capacity = b"ok\xff".len();
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
    fn try_into_string_bytes_from_frozen_bytes_mut_error_reuses_allocation() {
        let mut source = BytesMut::with_capacity(32);
        source.extend_from_slice(b"ok\xff");
        let source_capacity = source.capacity();
        let storage = source.freeze();
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let capacity = source_capacity;
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
    fn try_into_string_static_bytes_error_copies() {
        let storage = Bytes::from_static(b"ok\xff");
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let error = StringBytes::try_into_string(storage).unwrap_err();
        assert_eq!(error.utf8_error().valid_up_to(), 2);
        assert_eq!(error.utf8_error().error_len(), Some(1));
        let value = error.into_bytes();
        assert_eq!(value, b"ok\xff");
        assert_ne!(value.as_ptr(), address);
    }

    #[test]
    fn try_into_string_owner_backed_bytes_error_copies() {
        let storage = Bytes::from_owner(Vec::from(b"ok\xff".as_slice()));
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let error = StringBytes::try_into_string(storage).unwrap_err();
        assert_eq!(error.utf8_error().valid_up_to(), 2);
        assert_eq!(error.utf8_error().error_len(), Some(1));
        let value = error.into_bytes();
        assert_eq!(value, b"ok\xff");
        assert_ne!(value.as_ptr(), address);
    }

    #[test]
    fn try_into_string_shared_bytes_error_copies() {
        let owner = Bytes::from(Vec::from(b"ok\xff".as_slice()));
        let storage = owner.clone();
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let error = StringBytes::try_into_string(storage).unwrap_err();
        assert_eq!(error.utf8_error().valid_up_to(), 2);
        assert_eq!(error.utf8_error().error_len(), Some(1));
        let value = error.into_bytes();
        assert_eq!(value, b"ok\xff");
        assert_ne!(value.as_ptr(), address);
        assert_eq!(&owner[..], b"ok\xff");
    }

    #[test]
    fn try_into_string_bytes_error_compacts_unique_slice_into_original_allocation() {
        let mut original = Vec::with_capacity(64);
        original.extend_from_slice(b"prefix");
        original.extend_from_slice(b"ok\xff");
        let address = original.as_ptr();
        let capacity = original.capacity();
        let original = Bytes::from(original);
        let storage = original.slice(6..);
        assert_ne!(storage.as_ptr(), address);
        drop(original);
        let error = StringBytes::try_into_string(storage).unwrap_err();
        assert_eq!(error.utf8_error().valid_up_to(), 2);
        assert_eq!(error.utf8_error().error_len(), Some(1));
        let value = error.into_bytes();
        assert_eq!(value, b"ok\xff");
        assert_eq!(value.as_ptr(), address);
        assert_eq!(value.capacity(), capacity);
    }

    #[test]
    fn into_string_lossy_bytes_valid_utf8_reuses_buffer() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let source_capacity = source.capacity();
            let storage = Bytes::from(source);
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
    fn into_string_lossy_bytes_replaces_invalid_utf8() {
        for (input, expected) in [
            (b"a\xffb".as_slice(), "a�b"),
            (b"ok\xe2\x82", "ok�"),
            (b"\xc0\x80", "��"),
            (b"\xed\xa0\x80", "���"),
        ] {
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let storage = Bytes::from(source);
            let value = StringBytes::into_string_lossy(storage);
            assert_eq!(value, expected);
        }
    }

    #[test]
    fn into_string_lossy_empty_bytes_from_box() {
        let storage = Bytes::from(Box::<[u8]>::from([]));
        let capacity = 0;
        let value = StringBytes::into_string_lossy(storage);
        assert_eq!(value, "");
        assert_eq!(value.capacity(), capacity);
    }

    #[test]
    fn into_string_lossy_bytes_from_box_reuses_allocation() {
        let storage = Bytes::from(Box::<[u8]>::from(b"text".as_slice()));
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let capacity = b"text".len();
        let value = StringBytes::into_string_lossy(storage);
        assert_eq!(value, "text");
        if capacity != 0 {
            assert_eq!(value.as_ptr(), address);
        }
        assert_eq!(value.capacity(), capacity);
    }

    #[test]
    fn into_string_lossy_bytes_from_frozen_bytes_mut_reuses_allocation() {
        let mut source = BytesMut::with_capacity(32);
        source.extend_from_slice(b"text");
        let source_capacity = source.capacity();
        let storage = source.freeze();
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let capacity = source_capacity;
        let value = StringBytes::into_string_lossy(storage);
        assert_eq!(value, "text");
        if capacity != 0 {
            assert_eq!(value.as_ptr(), address);
        }
        assert_eq!(value.capacity(), capacity);
    }

    #[test]
    fn into_string_lossy_static_bytes_copies() {
        let storage = Bytes::from_static(b"text");
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let value = StringBytes::into_string_lossy(storage);
        assert_eq!(value, "text");
        assert_ne!(value.as_ptr(), address);
    }

    #[test]
    fn into_string_lossy_owner_backed_bytes_copies() {
        let storage = Bytes::from_owner(Vec::from(b"text".as_slice()));
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let value = StringBytes::into_string_lossy(storage);
        assert_eq!(value, "text");
        assert_ne!(value.as_ptr(), address);
    }

    #[test]
    fn into_string_lossy_shared_bytes_copies() {
        let owner = Bytes::from(Vec::from(b"text".as_slice()));
        let storage = owner.clone();
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let value = StringBytes::into_string_lossy(storage);
        assert_eq!(value, "text");
        assert_ne!(value.as_ptr(), address);
        assert_eq!(&owner[..], b"text");
    }

    #[test]
    fn into_string_lossy_bytes_compacts_unique_slice_into_original_allocation() {
        let mut original = Vec::with_capacity(64);
        original.extend_from_slice(b"prefix");
        original.extend_from_slice(b"text");
        let address = original.as_ptr();
        let capacity = original.capacity();
        let original = Bytes::from(original);
        let storage = original.slice(6..);
        assert_ne!(storage.as_ptr(), address);
        drop(original);
        let value = StringBytes::into_string_lossy(storage);
        assert_eq!(value, "text");
        assert_eq!(value.as_ptr(), address);
        assert_eq!(value.capacity(), capacity);
    }

    #[test]
    fn into_string_unchecked_bytes_reuses_buffer() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut source = Vec::with_capacity(input.len() + 32);
            source.extend_from_slice(input);
            let source_capacity = source.capacity();
            let storage = Bytes::from(source);
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
    fn into_string_unchecked_empty_bytes_from_box() {
        let storage = Bytes::from(Box::<[u8]>::from([]));
        let capacity = 0;
        // SAFETY: the storage contains the valid UTF-8 literal above.
        let value = unsafe { StringBytes::into_string_unchecked(storage) };
        assert_eq!(value, "");
        assert_eq!(value.capacity(), capacity);
    }

    #[test]
    fn into_string_unchecked_bytes_from_box_reuses_allocation() {
        let storage = Bytes::from(Box::<[u8]>::from(b"text".as_slice()));
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let capacity = b"text".len();
        // SAFETY: the storage contains the valid UTF-8 literal above.
        let value = unsafe { StringBytes::into_string_unchecked(storage) };
        assert_eq!(value, "text");
        if capacity != 0 {
            assert_eq!(value.as_ptr(), address);
        }
        assert_eq!(value.capacity(), capacity);
    }

    #[test]
    fn into_string_unchecked_bytes_from_frozen_bytes_mut_reuses_allocation() {
        let mut source = BytesMut::with_capacity(32);
        source.extend_from_slice(b"text");
        let source_capacity = source.capacity();
        let storage = source.freeze();
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let capacity = source_capacity;
        // SAFETY: the storage contains the valid UTF-8 literal above.
        let value = unsafe { StringBytes::into_string_unchecked(storage) };
        assert_eq!(value, "text");
        if capacity != 0 {
            assert_eq!(value.as_ptr(), address);
        }
        assert_eq!(value.capacity(), capacity);
    }

    #[test]
    fn into_string_unchecked_static_bytes_copies() {
        let storage = Bytes::from_static(b"text");
        let address = StringBytes::as_bytes(&storage).as_ptr();
        // SAFETY: the storage contains the valid UTF-8 literal above.
        let value = unsafe { StringBytes::into_string_unchecked(storage) };
        assert_eq!(value, "text");
        assert_ne!(value.as_ptr(), address);
    }

    #[test]
    fn into_string_unchecked_owner_backed_bytes_copies() {
        let storage = Bytes::from_owner(Vec::from(b"text".as_slice()));
        let address = StringBytes::as_bytes(&storage).as_ptr();
        // SAFETY: the storage contains the valid UTF-8 literal above.
        let value = unsafe { StringBytes::into_string_unchecked(storage) };
        assert_eq!(value, "text");
        assert_ne!(value.as_ptr(), address);
    }

    #[test]
    fn into_string_unchecked_shared_bytes_copies() {
        let owner = Bytes::from(Vec::from(b"text".as_slice()));
        let storage = owner.clone();
        let address = StringBytes::as_bytes(&storage).as_ptr();
        // SAFETY: the storage contains the valid UTF-8 literal above.
        let value = unsafe { StringBytes::into_string_unchecked(storage) };
        assert_eq!(value, "text");
        assert_ne!(value.as_ptr(), address);
        assert_eq!(&owner[..], b"text");
    }

    #[test]
    fn into_string_unchecked_bytes_compacts_unique_slice_into_original_allocation() {
        let mut original = Vec::with_capacity(64);
        original.extend_from_slice(b"prefix");
        original.extend_from_slice(b"text");
        let address = original.as_ptr();
        let capacity = original.capacity();
        let original = Bytes::from(original);
        let storage = original.slice(6..);
        assert_ne!(storage.as_ptr(), address);
        drop(original);
        // SAFETY: the storage contains the valid UTF-8 literal above.
        let value = unsafe { StringBytes::into_string_unchecked(storage) };
        assert_eq!(value, "text");
        assert_eq!(value.as_ptr(), address);
        assert_eq!(value.capacity(), capacity);
    }

    #[test]
    fn box_into_bytes_bytes_reuses_inner_buffer() {
        let storage: Box<Bytes> = Box::new(Bytes::from(Vec::from(b"text".as_slice())));
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
    fn rc_into_bytes_bytes_reuses_inner_buffer() {
        let storage: Rc<Bytes> = Rc::new(Bytes::from(Vec::from(b"text".as_slice())));
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
    fn arc_into_bytes_bytes_reuses_inner_buffer() {
        let storage: Arc<Bytes> = Arc::new(Bytes::from(Vec::from(b"text".as_slice())));
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
    fn box_into_string_bytes_reuses_inner_buffer() {
        let storage: Box<Bytes> = Box::new(Bytes::from(Vec::from(b"text".as_slice())));
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
    fn rc_into_string_bytes_reuses_inner_buffer() {
        let storage: Rc<Bytes> = Rc::new(Bytes::from(Vec::from(b"text".as_slice())));
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
    fn arc_into_string_bytes_reuses_inner_buffer() {
        let storage: Arc<Bytes> = Arc::new(Bytes::from(Vec::from(b"text".as_slice())));
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
