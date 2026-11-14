use std::borrow::Cow;
use std::rc::Rc;
use std::sync::Arc;

use crate::impls::{impl_pointer_conversions_for_sized, impl_string_bytes};
use crate::{sealed, FromUtf8Error, StringBytes, Utf8Error};

impl_string_bytes!(
    /// Borrows UTF-8 text from either variant without validating it again.
    ///
    /// Consuming conversions reuse an owned string's buffer and capacity,
    /// or copy a borrowed string. Copying conversions copy either variant.
    /// Checked string conversions always succeed, and
    /// [`to_string_lossy`](StringBytes::to_string_lossy) returns a borrowed string.
    Cow<'_, str> {
        /// Borrows the UTF-8 bytes of either variant.
        #[inline]
        fn as_bytes(&self) -> &[u8] {
            self.as_ref().as_bytes()
        }

        /// Reuses an owned string's buffer and capacity, or copies a borrowed string.
        #[inline]
        fn into_bytes(self) -> Vec<u8> {
            String::into_bytes(self.into_owned())
        }

        /// Borrows the string without checking UTF-8.
        #[inline]
        fn try_borrow_str(&self) -> Result<&str, Utf8Error> {
            Ok(self.as_ref())
        }

        /// Copies the string without checking UTF-8.
        #[inline]
        fn try_to_string(&self) -> Result<String, Utf8Error> {
            // Borrowing first ensures that even an owned String is copied.
            Ok(String::from(self.as_ref()))
        }

        /// Borrows the string without allocating or checking UTF-8.
        #[inline]
        fn to_string_lossy(&self) -> Cow<'_, str> {
            Cow::Borrowed(self.as_ref())
        }

        /// Reuses an owned string or copies a borrowed string, without checking UTF-8.
        #[inline]
        fn try_into_string(self) -> Result<String, FromUtf8Error<Self>> {
            Ok(self.into_owned())
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
    fn as_bytes_cow_str_borrowed_exposes_stable_contents() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let storage: Cow<'_, str> = Cow::Borrowed(expected);
            assert_eq!(StringBytes::as_bytes(&storage), input);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            assert_eq!(StringBytes::as_bytes(&storage), input);
            assert_eq!(StringBytes::as_bytes(&storage).as_ptr(), address);
        }
    }

    #[test]
    fn as_bytes_cow_str_borrowed_matches_deref() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let storage: Cow<'_, str> = Cow::Borrowed(expected);
            let bytes = StringBytes::as_bytes(&storage);
            let other = StringBytes::as_bytes(std::ops::Deref::deref(&storage));
            assert_eq!(bytes, input);
            assert_eq!(other, input);
            assert!(std::ptr::eq(bytes, other));
        }
    }

    #[test]
    fn as_bytes_cow_str_owned_exposes_stable_contents() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut source = String::with_capacity(expected.len() + 32);
            source.push_str(expected);
            let storage: Cow<'_, str> = Cow::Owned(source);
            assert_eq!(StringBytes::as_bytes(&storage), input);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            assert_eq!(StringBytes::as_bytes(&storage), input);
            assert_eq!(StringBytes::as_bytes(&storage).as_ptr(), address);
        }
    }

    #[test]
    fn as_bytes_cow_str_owned_matches_deref() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut source = String::with_capacity(expected.len() + 32);
            source.push_str(expected);
            let storage: Cow<'_, str> = Cow::Owned(source);
            let bytes = StringBytes::as_bytes(&storage);
            let other = StringBytes::as_bytes(std::ops::Deref::deref(&storage));
            assert_eq!(bytes, input);
            assert_eq!(other, input);
            assert!(std::ptr::eq(bytes, other));
        }
    }

    #[test]
    fn to_bytes_cow_str_borrowed_returns_an_independent_copy() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let storage: Cow<'_, str> = Cow::Borrowed(expected);
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
    fn to_bytes_cow_str_owned_returns_an_independent_copy() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut source = String::with_capacity(expected.len() + 32);
            source.push_str(expected);
            let storage: Cow<'_, str> = Cow::Owned(source);
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
    fn into_bytes_cow_str_borrowed_copies_storage() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let storage: Cow<'_, str> = Cow::Borrowed(expected);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let bytes = StringBytes::into_bytes(storage);
            assert_eq!(bytes, input);
            if !input.is_empty() {
                assert_ne!(bytes.as_ptr(), address);
            }
        }
    }

    #[test]
    fn into_bytes_cow_str_owned_reuses_buffer() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut source = String::with_capacity(expected.len() + 32);
            source.push_str(expected);
            let source_capacity = source.capacity();
            let storage: Cow<'_, str> = Cow::Owned(source);
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
    fn try_borrow_str_cow_str_borrowed_borrows_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let storage: Cow<'_, str> = Cow::Borrowed(expected);
            let value = StringBytes::try_borrow_str(&storage).unwrap();
            assert_eq!(value, expected);
            assert!(std::ptr::eq(
                value.as_bytes(),
                StringBytes::as_bytes(&storage)
            ));
        }
    }

    #[test]
    fn try_borrow_str_cow_str_owned_borrows_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let mut source = String::with_capacity(expected.len() + 32);
            source.push_str(expected);
            let storage: Cow<'_, str> = Cow::Owned(source);
            let value = StringBytes::try_borrow_str(&storage).unwrap();
            assert_eq!(value, expected);
            assert!(std::ptr::eq(
                value.as_bytes(),
                StringBytes::as_bytes(&storage)
            ));
        }
    }

    #[test]
    fn borrow_str_unchecked_cow_str_borrowed_borrows_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let storage: Cow<'_, str> = Cow::Borrowed(expected);
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
    fn borrow_str_unchecked_cow_str_owned_borrows_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let mut source = String::with_capacity(expected.len() + 32);
            source.push_str(expected);
            let storage: Cow<'_, str> = Cow::Owned(source);
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
    fn try_to_string_cow_str_borrowed_copies_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let storage: Cow<'_, str> = Cow::Borrowed(expected);
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
    fn try_to_string_cow_str_owned_copies_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut source = String::with_capacity(expected.len() + 32);
            source.push_str(expected);
            let storage: Cow<'_, str> = Cow::Owned(source);
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
    fn to_string_lossy_cow_str_borrowed_borrows_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let storage: Cow<'_, str> = Cow::Borrowed(expected);
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
    fn to_string_lossy_cow_str_owned_borrows_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let mut source = String::with_capacity(expected.len() + 32);
            source.push_str(expected);
            let storage: Cow<'_, str> = Cow::Owned(source);
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
    fn to_string_unchecked_cow_str_borrowed_copies_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let storage: Cow<'_, str> = Cow::Borrowed(expected);
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
    fn to_string_unchecked_cow_str_owned_copies_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut source = String::with_capacity(expected.len() + 32);
            source.push_str(expected);
            let storage: Cow<'_, str> = Cow::Owned(source);
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
    fn try_into_string_cow_str_borrowed_copies_storage() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let storage: Cow<'_, str> = Cow::Borrowed(expected);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let value = StringBytes::try_into_string(storage).unwrap();
            assert_eq!(value, expected);
            if !input.is_empty() {
                assert_ne!(value.as_ptr(), address);
            }
        }
    }

    #[test]
    fn try_into_string_cow_str_owned_reuses_buffer() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let mut source = String::with_capacity(expected.len() + 32);
            source.push_str(expected);
            let source_capacity = source.capacity();
            let storage: Cow<'_, str> = Cow::Owned(source);
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
    fn into_string_lossy_cow_str_borrowed_valid_utf8_copies_storage() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let storage: Cow<'_, str> = Cow::Borrowed(expected);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let value = StringBytes::into_string_lossy(storage);
            assert_eq!(value, expected);
            if !input.is_empty() {
                assert_ne!(value.as_ptr(), address);
            }
        }
    }

    #[test]
    fn into_string_lossy_cow_str_owned_valid_utf8_reuses_buffer() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let mut source = String::with_capacity(expected.len() + 32);
            source.push_str(expected);
            let source_capacity = source.capacity();
            let storage: Cow<'_, str> = Cow::Owned(source);
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
    fn into_string_unchecked_cow_str_borrowed_copies_storage() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let storage: Cow<'_, str> = Cow::Borrowed(expected);
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
    fn into_string_unchecked_cow_str_owned_reuses_buffer() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let mut source = String::with_capacity(expected.len() + 32);
            source.push_str(expected);
            let source_capacity = source.capacity();
            let storage: Cow<'_, str> = Cow::Owned(source);
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
    fn box_into_bytes_owned_cow_str_reuses_inner_buffer() {
        let storage: Box<Cow<'static, str>> = Box::new(Cow::Owned(String::from("text")));
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
    fn rc_into_bytes_owned_cow_str_reuses_inner_buffer() {
        let storage: Rc<Cow<'static, str>> = Rc::new(Cow::Owned(String::from("text")));
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
    fn arc_into_bytes_owned_cow_str_reuses_inner_buffer() {
        let storage: Arc<Cow<'static, str>> = Arc::new(Cow::Owned(String::from("text")));
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
    fn box_into_string_owned_cow_str_reuses_inner_buffer() {
        let storage: Box<Cow<'static, str>> = Box::new(Cow::Owned(String::from("text")));
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
    fn rc_into_string_owned_cow_str_reuses_inner_buffer() {
        let storage: Rc<Cow<'static, str>> = Rc::new(Cow::Owned(String::from("text")));
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
    fn arc_into_string_owned_cow_str_reuses_inner_buffer() {
        let storage: Arc<Cow<'static, str>> = Arc::new(Cow::Owned(String::from("text")));
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
