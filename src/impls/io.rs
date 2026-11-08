use std::io::{IoSlice, IoSliceMut};

use super::impl_string_bytes;
use crate::{sealed, StringBytes};

// I/O slices borrow their buffers. The default conversions copy the current
// view, even when the wrapper is consumed through Box, Rc, or Arc.
impl_string_bytes!(IoSlice<'_> {
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        self
    }
});

impl_string_bytes!(IoSliceMut<'_> {
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        self
    }
});

#[cfg(test)]
mod tests {
    use std::io::{IoSlice, IoSliceMut};
    use std::ops::Deref;
    use std::rc::Rc;
    use std::string::FromUtf8Error;
    use std::sync::Arc;

    use crate::StringBytes;

    const INPUTS: [&[u8]; 7] = [
        b"",
        b"text",
        "é说明".as_bytes(),
        b"\0e\xcc\x81",
        b"ok\xff",
        b"\xe2\x82",
        b"\xed\xa0\x80",
    ];

    fn for_each_view(mut check: impl FnMut(&dyn StringBytes, &[u8], *const u8)) {
        for input in INPUTS {
            // Include exhausted views and offsets inside UTF-8 sequences.
            for offset in 0..=input.len() {
                let expected = &input[offset..];
                let mut view = IoSlice::new(input);
                view.advance(offset);
                check(&view, expected, expected.as_ptr());

                let mut buffer = input.to_vec();
                let address = buffer[offset..].as_ptr();
                let mut view = IoSliceMut::new(&mut buffer);
                view.advance(offset);
                check(&view, expected, address);
                assert_eq!(buffer, input);
            }
        }
    }

    fn string_bytes(result: Result<String, FromUtf8Error>, expected: &[u8]) -> Vec<u8> {
        match result {
            Ok(text) => {
                assert_eq!(text, std::str::from_utf8(expected).unwrap());
                text.into_bytes()
            }
            Err(error) => {
                assert_eq!(
                    error.utf8_error(),
                    std::str::from_utf8(expected).unwrap_err()
                );
                error.into_bytes()
            }
        }
    }

    fn check_copied_conversion<T: StringBytes>(storage: T, expected: &[u8], into_bytes: bool) {
        let address = storage.as_bytes().as_ptr();
        let mut bytes = if into_bytes {
            StringBytes::into_bytes(storage)
        } else {
            string_bytes(storage.try_into_string(), expected)
        };
        assert_eq!(bytes, expected);
        if !bytes.is_empty() {
            assert_ne!(bytes.as_ptr(), address);
            bytes[0] ^= 0xff;
        }
    }

    fn check_consuming_conversions(into_bytes: bool) {
        for input in INPUTS {
            for offset in 0..=input.len() {
                let expected = &input[offset..];
                let mut view = IoSlice::new(input);
                view.advance(offset);
                check_copied_conversion(view, expected, into_bytes);

                let mut buffer = input.to_vec();
                let mut view = IoSliceMut::new(&mut buffer);
                view.advance(offset);
                check_copied_conversion(view, expected, into_bytes);
                assert_eq!(buffer, input);
            }
        }
    }

    fn check_pointer_conversions(into_bytes: bool) {
        macro_rules! check {
            ($pointer:ident) => {
                for input in INPUTS {
                    for offset in 0..=input.len() {
                        let expected = &input[offset..];
                        let mut view = IoSlice::new(input);
                        view.advance(offset);
                        let storage: $pointer<dyn StringBytes + Send + Sync + '_> =
                            $pointer::new(view);
                        check_copied_conversion(storage, expected, into_bytes);

                        let mut buffer = input.to_vec();
                        let mut view = IoSliceMut::new(&mut buffer);
                        view.advance(offset);
                        let storage: $pointer<dyn StringBytes + Send + Sync + '_> =
                            $pointer::new(view);
                        check_copied_conversion(storage, expected, into_bytes);
                        assert_eq!(buffer, input);
                    }
                }
            };
        }

        check!(Box);
        check!(Rc);
        check!(Arc);
    }

    #[test]
    fn as_bytes_borrows_the_current_view() {
        for_each_view(|storage, expected, address| {
            for _ in 0..3 {
                let bytes = storage.as_bytes();
                assert_eq!(bytes, expected);
                assert_eq!(bytes.as_ptr(), address);
            }
        });
    }

    #[test]
    fn as_bytes_reflects_io_slice_mut_edits() {
        let mut buffer = *b"prefixtext";
        let address = buffer[6..].as_ptr();
        {
            let mut view = IoSliceMut::new(&mut buffer);
            view.advance(6);
            view.copy_from_slice(b"a\xffb\0");
            assert_eq!(view.as_bytes(), b"a\xffb\0");
            assert_eq!(view.as_bytes().as_ptr(), address);
        }
        assert_eq!(&buffer, b"prefixa\xffb\0");
    }

    #[test]
    fn to_bytes_copies_and_preserves_the_source() {
        for_each_view(|storage, expected, address| {
            let mut bytes = storage.to_bytes();
            assert_eq!(bytes, expected);
            if !bytes.is_empty() {
                assert_ne!(bytes.as_ptr(), address);
                bytes[0] ^= 0xff;
            }
            assert_eq!(storage.as_bytes(), expected);
            assert_eq!(storage.as_bytes().as_ptr(), address);
        });
    }

    #[test]
    fn into_bytes_copies_the_current_view() {
        check_consuming_conversions(true);
    }

    #[test]
    fn into_bytes_copies_through_owning_pointers() {
        check_pointer_conversions(true);
    }

    #[test]
    fn try_borrow_str_validates_the_current_view() {
        for_each_view(|storage, expected, address| {
            let result = storage.try_borrow_str();
            assert_eq!(result, std::str::from_utf8(expected));
            if let Ok(text) = result {
                assert_eq!(text.as_ptr(), address);
            }
        });
    }

    #[test]
    fn try_to_string_copies_and_preserves_the_source() {
        for_each_view(|storage, expected, address| {
            let mut bytes = string_bytes(storage.try_to_string(), expected);
            assert_eq!(bytes, expected);
            if !bytes.is_empty() {
                assert_ne!(bytes.as_ptr(), address);
                bytes[0] ^= 0xff;
            }
            assert_eq!(storage.as_bytes(), expected);
            assert_eq!(storage.as_bytes().as_ptr(), address);
        });
    }

    #[test]
    fn try_into_string_copies_and_validates_the_current_view() {
        check_consuming_conversions(false);
    }

    #[test]
    fn try_into_string_copies_through_owning_pointers() {
        check_pointer_conversions(false);
    }

    #[test]
    fn deref_target_implements_string_bytes_and_borrows_the_same_slice() {
        fn check<T>(storage: &T)
        where
            T: StringBytes + Deref,
            T::Target: StringBytes,
        {
            assert!(std::ptr::eq(
                StringBytes::as_bytes(storage),
                StringBytes::as_bytes(T::deref(storage))
            ));
        }

        for input in INPUTS {
            for offset in 0..=input.len() {
                let mut view = IoSlice::new(input);
                view.advance(offset);
                check(&view);

                let mut buffer = input.to_vec();
                let mut view = IoSliceMut::new(&mut buffer);
                view.advance(offset);
                check(&view);
            }
        }
    }
}
