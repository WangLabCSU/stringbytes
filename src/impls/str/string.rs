use std::borrow::Cow;
use std::rc::Rc;
use std::sync::Arc;

use crate::impls::{impl_pointer_conversions_for_sized, impl_string_bytes};
use crate::{sealed, FromUtf8Error, StringBytes, Utf8Error};

impl_string_bytes!(
    /// Uses the string's UTF-8 bytes without validating them again.
    ///
    /// Consuming conversions retain the buffer and capacity.
    /// Borrowing conversions return a view of the string; copying conversions
    /// leave the original unchanged. Checked string conversions always succeed,
    /// and [`to_string_lossy`](StringBytes::to_string_lossy) returns a borrowed string.
    String {
        /// Borrows the UTF-8 bytes of the string.
        #[inline]
        fn as_bytes(&self) -> &[u8] {
            String::as_bytes(self)
        }

        /// Converts the string into bytes, preserving its buffer and capacity.
        #[inline]
        fn into_bytes(self) -> Vec<u8> {
            String::into_bytes(self)
        }

        /// Borrows the string without checking UTF-8.
        #[inline]
        fn try_borrow_str(&self) -> Result<&str, Utf8Error> {
            Ok(self.as_str())
        }

        /// Clones the string without checking UTF-8.
        #[inline]
        fn try_to_string(&self) -> Result<String, Utf8Error> {
            Ok(self.clone())
        }

        /// Borrows the string without allocating or checking UTF-8.
        #[inline]
        fn to_string_lossy(&self) -> Cow<'_, str> {
            Cow::Borrowed(self.as_str())
        }

        /// Returns this string unchanged, preserving its buffer and capacity.
        #[inline]
        fn try_into_string(self) -> Result<String, FromUtf8Error<Self>> {
            Ok(self)
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
    fn as_bytes_string_exposes_stable_contents() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut storage = String::with_capacity(expected.len() + 32);
            storage.push_str(expected);
            assert_eq!(StringBytes::as_bytes(&storage), input);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            assert_eq!(StringBytes::as_bytes(&storage), input);
            assert_eq!(StringBytes::as_bytes(&storage).as_ptr(), address);
        }
    }

    #[test]
    fn as_bytes_string_matches_as_ref() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut storage = String::with_capacity(expected.len() + 32);
            storage.push_str(expected);
            let bytes = StringBytes::as_bytes(&storage);
            let other = AsRef::<[u8]>::as_ref(&storage);
            assert_eq!(bytes, input);
            assert_eq!(other, input);
            assert!(std::ptr::eq(bytes, other));
        }
    }

    #[test]
    fn as_bytes_string_matches_deref() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut storage = String::with_capacity(expected.len() + 32);
            storage.push_str(expected);
            let bytes = StringBytes::as_bytes(&storage);
            let other = StringBytes::as_bytes(std::ops::Deref::deref(&storage));
            assert_eq!(bytes, input);
            assert_eq!(other, input);
            assert!(std::ptr::eq(bytes, other));
        }
    }

    #[test]
    fn to_bytes_string_returns_an_independent_copy() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut storage = String::with_capacity(expected.len() + 32);
            storage.push_str(expected);
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
    fn into_bytes_string_reuses_buffer() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut storage = String::with_capacity(expected.len() + 32);
            storage.push_str(expected);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let capacity = storage.capacity();
            let bytes = StringBytes::into_bytes(storage);
            assert_eq!(bytes, input);
            assert_eq!(bytes.capacity(), capacity);
            if capacity != 0 {
                assert_eq!(bytes.as_ptr(), address);
            }
        }
    }

    #[test]
    fn try_borrow_str_string_borrows_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let mut storage = String::with_capacity(expected.len() + 32);
            storage.push_str(expected);
            let value = StringBytes::try_borrow_str(&storage).unwrap();
            assert_eq!(value, expected);
            assert!(std::ptr::eq(
                value.as_bytes(),
                StringBytes::as_bytes(&storage)
            ));
        }
    }

    #[test]
    fn borrow_str_unchecked_string_borrows_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let mut storage = String::with_capacity(expected.len() + 32);
            storage.push_str(expected);
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
    fn try_to_string_string_copies_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut storage = String::with_capacity(expected.len() + 32);
            storage.push_str(expected);
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
    fn to_string_lossy_string_borrows_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let mut storage = String::with_capacity(expected.len() + 32);
            storage.push_str(expected);
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
    fn to_string_unchecked_string_copies_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let mut storage = String::with_capacity(expected.len() + 32);
            storage.push_str(expected);
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
    fn try_into_string_string_reuses_buffer() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let mut storage = String::with_capacity(expected.len() + 32);
            storage.push_str(expected);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let capacity = storage.capacity();
            let value = StringBytes::try_into_string(storage).unwrap();
            assert_eq!(value, expected);
            assert_eq!(value.capacity(), capacity);
            if capacity != 0 {
                assert_eq!(value.as_ptr(), address);
            }
        }
    }

    #[test]
    fn into_string_lossy_string_valid_utf8_reuses_buffer() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let mut storage = String::with_capacity(expected.len() + 32);
            storage.push_str(expected);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let capacity = storage.capacity();
            let value = StringBytes::into_string_lossy(storage);
            assert_eq!(value, expected);
            assert_eq!(value.capacity(), capacity);
            if capacity != 0 {
                assert_eq!(value.as_ptr(), address);
            }
        }
    }

    #[test]
    fn into_string_unchecked_string_reuses_buffer() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let mut storage = String::with_capacity(expected.len() + 32);
            storage.push_str(expected);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let capacity = storage.capacity();
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
    fn box_into_bytes_string_reuses_inner_buffer() {
        let storage: Box<String> = Box::new(String::from("text"));
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
    fn rc_into_bytes_string_reuses_inner_buffer() {
        let storage: Rc<String> = Rc::new(String::from("text"));
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
    fn arc_into_bytes_string_reuses_inner_buffer() {
        let storage: Arc<String> = Arc::new(String::from("text"));
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
    fn box_into_string_string_reuses_inner_buffer() {
        let storage: Box<String> = Box::new(String::from("text"));
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
    fn rc_into_string_string_reuses_inner_buffer() {
        let storage: Rc<String> = Rc::new(String::from("text"));
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
    fn arc_into_string_string_reuses_inner_buffer() {
        let storage: Arc<String> = Arc::new(String::from("text"));
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
