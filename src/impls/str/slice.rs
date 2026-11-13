use std::borrow::Cow;
use std::rc::Rc;
use std::str::Utf8Error;
use std::string::FromUtf8Error;
use std::sync::Arc;

use crate::impls::impl_string_bytes;
use crate::{sealed, StringBytes};

// str is unsized, so consuming conversions use owning pointer receivers.
impl_string_bytes!(str {
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        str::as_bytes(self)
    }

    #[inline]
    fn try_borrow_str(&self) -> Result<&str, Utf8Error> {
        Ok(self)
    }

    #[inline]
    fn try_to_string(&self) -> Result<String, FromUtf8Error> {
        Ok(self.to_owned())
    }

    #[inline]
    fn to_string_lossy(&self) -> Cow<'_, str> {
        Cow::Borrowed(self)
    }

    #[inline]
    fn box_into_bytes(self: Box<Self>) -> Vec<u8> {
        <[u8]>::into_vec(self.into_boxed_bytes())
    }

    #[inline]
    fn box_into_string(self: Box<Self>) -> Result<String, FromUtf8Error> {
        // String::from takes over the Box's allocation.
        Ok(String::from(self))
    }

    // Rc<str> and Arc<str> store reference counts alongside the bytes.
    // Their allocations cannot be transferred to String, so these
    // conversions copy even when only one strong reference remains.
    #[inline]
    fn rc_into_string(self: Rc<Self>) -> Result<String, FromUtf8Error> {
        Ok(String::from(&*self))
    }

    #[inline]
    fn arc_into_string(self: Arc<Self>) -> Result<String, FromUtf8Error> {
        Ok(String::from(&*self))
    }
});

#[cfg(test)]
mod tests {
    use std::borrow::Cow;
    use std::rc::Rc;
    use std::sync::Arc;

    use crate::StringBytes;

    #[test]
    fn as_bytes_str_exposes_stable_contents() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let storage = expected;
            assert_eq!(StringBytes::as_bytes(storage), input);
            let address = StringBytes::as_bytes(storage).as_ptr();
            assert_eq!(StringBytes::as_bytes(storage), input);
            assert_eq!(StringBytes::as_bytes(storage).as_ptr(), address);
        }
    }

    #[test]
    fn as_bytes_str_matches_as_ref() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let storage = expected;
            let bytes = StringBytes::as_bytes(storage);
            let other = AsRef::<[u8]>::as_ref(storage);
            assert_eq!(bytes, input);
            assert_eq!(other, input);
            assert!(std::ptr::eq(bytes, other));
        }
    }

    #[test]
    fn to_bytes_str_returns_an_independent_copy() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let storage = expected;
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
    fn try_borrow_str_str_borrows_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let storage = expected;
            let value = StringBytes::try_borrow_str(storage).unwrap();
            assert_eq!(value, expected);
            assert!(std::ptr::eq(
                value.as_bytes(),
                StringBytes::as_bytes(storage)
            ));
        }
    }

    #[test]
    fn borrow_str_unchecked_str_borrows_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let storage = expected;
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
    fn try_to_string_str_copies_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let storage = expected;
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
    fn to_string_lossy_str_borrows_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let storage = expected;
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
    fn to_string_unchecked_str_copies_valid_utf8() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let input = expected.as_bytes();
            let storage = expected;
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
    fn box_into_bytes_str_reuses_allocation() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let storage: Box<str> = Box::<str>::from(expected);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let capacity = expected.len();
            let value = <str as StringBytes>::box_into_bytes(storage);
            assert_eq!(value, expected.as_bytes());
            assert_eq!(value.capacity(), capacity);
            if capacity != 0 {
                assert_eq!(value.as_ptr(), address);
            }
        }
    }

    #[test]
    fn box_into_string_str_reuses_allocation() {
        for expected in ["", "text", "é说明", "\0e\u{301}"] {
            let storage: Box<str> = Box::<str>::from(expected);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let capacity = expected.len();
            let value = <str as StringBytes>::box_into_string(storage).unwrap();
            assert_eq!(value, expected);
            assert_eq!(value.capacity(), capacity);
            if capacity != 0 {
                assert_eq!(value.as_ptr(), address);
            }
        }
    }

    #[test]
    fn rc_into_string_unique_str_copies() {
        let storage = Rc::<str>::from("étext");
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let value = StringBytes::rc_into_string(storage).unwrap();
        assert_eq!(value, "étext");
        assert_ne!(value.as_ptr(), address);
    }

    #[test]
    fn rc_into_string_shared_str_copies() {
        let storage = Rc::<str>::from("étext");
        let other = Rc::clone(&storage);
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let value = StringBytes::rc_into_string(storage).unwrap();
        assert_eq!(value, "étext");
        assert_ne!(value.as_ptr(), address);
        assert_eq!(&*other, "étext");
    }

    #[test]
    fn arc_into_string_unique_str_copies() {
        let storage = Arc::<str>::from("étext");
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let value = StringBytes::arc_into_string(storage).unwrap();
        assert_eq!(value, "étext");
        assert_ne!(value.as_ptr(), address);
    }

    #[test]
    fn arc_into_string_shared_str_copies() {
        let storage = Arc::<str>::from("étext");
        let other = Arc::clone(&storage);
        let address = StringBytes::as_bytes(&storage).as_ptr();
        let value = StringBytes::arc_into_string(storage).unwrap();
        assert_eq!(value, "étext");
        assert_ne!(value.as_ptr(), address);
        assert_eq!(&*other, "étext");
    }
}
