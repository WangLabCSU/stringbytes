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
    use std::rc::Rc;
    use std::string::FromUtf8Error;
    use std::sync::Arc;

    use crate::test_utils::{
        assert_deref_matches_as_bytes, assert_into_bytes_preserves_bytes, assert_stable_bytes,
        assert_to_bytes_preserves_bytes, assert_try_borrow_str_preserves_bytes,
        assert_try_into_string_preserves_bytes, assert_try_to_string_preserves_bytes,
    };
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

    fn check_copied_conversion<T: StringBytes>(storage: T, expected: &[u8], into_bytes: bool) {
        let address = storage.as_bytes().as_ptr();
        let mut bytes = if into_bytes {
            assert_into_bytes_preserves_bytes(storage)
        } else {
            assert_try_into_string_preserves_bytes(storage)
                .map_or_else(FromUtf8Error::into_bytes, String::into_bytes)
        };
        assert_eq!(bytes, expected);
        if !bytes.is_empty() {
            assert_ne!(bytes.as_ptr(), address);
            bytes[0] ^= 0xff;
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
    fn io_slice_guarantees() {
        for input in INPUTS {
            // Include exhausted views and offsets inside UTF-8 sequences.
            for offset in 0..=input.len() {
                let expected = &input[offset..];
                let address = expected.as_ptr();
                let mut storage = IoSlice::new(input);
                storage.advance(offset);
                assert_eq!(storage.as_bytes().as_ptr(), address);
                assert_eq!(StringBytes::as_bytes(&storage), expected);
                assert_stable_bytes(&storage);
                assert_deref_matches_as_bytes(&storage);
                assert_to_bytes_preserves_bytes(&storage);
                let _ = assert_try_borrow_str_preserves_bytes(&storage);
                let _ = assert_try_to_string_preserves_bytes(&storage);
                let mut bytes = assert_into_bytes_preserves_bytes(storage);
                if !bytes.is_empty() {
                    assert_ne!(bytes.as_ptr(), address);
                    bytes[0] ^= 0xff;
                }

                let mut storage = IoSlice::new(input);
                storage.advance(offset);
                let mut bytes = assert_try_into_string_preserves_bytes(storage)
                    .map_or_else(FromUtf8Error::into_bytes, String::into_bytes);
                if !bytes.is_empty() {
                    assert_ne!(bytes.as_ptr(), address);
                    bytes[0] ^= 0xff;
                }
            }
        }
    }

    #[test]
    fn io_slice_mut_guarantees() {
        for input in INPUTS {
            // Include exhausted views and offsets inside UTF-8 sequences.
            for offset in 0..=input.len() {
                let expected = &input[offset..];
                let mut buffer = input.to_vec();
                let address = buffer[offset..].as_ptr();
                let mut storage = IoSliceMut::new(&mut buffer);
                storage.advance(offset);
                assert_eq!(storage.as_bytes().as_ptr(), address);
                assert_eq!(StringBytes::as_bytes(&storage), expected);
                assert_stable_bytes(&storage);
                assert_deref_matches_as_bytes(&storage);
                assert_to_bytes_preserves_bytes(&storage);
                let _ = assert_try_borrow_str_preserves_bytes(&storage);
                let _ = assert_try_to_string_preserves_bytes(&storage);
                let mut bytes = assert_into_bytes_preserves_bytes(storage);
                if !bytes.is_empty() {
                    assert_ne!(bytes.as_ptr(), address);
                    bytes[0] ^= 0xff;
                }
                assert_eq!(buffer, input);

                let mut storage = IoSliceMut::new(&mut buffer);
                storage.advance(offset);
                let mut bytes = assert_try_into_string_preserves_bytes(storage)
                    .map_or_else(FromUtf8Error::into_bytes, String::into_bytes);
                if !bytes.is_empty() {
                    assert_ne!(bytes.as_ptr(), address);
                    bytes[0] ^= 0xff;
                }
                assert_eq!(buffer, input);
            }
        }
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
    fn into_bytes_copies_through_owning_pointers() {
        check_pointer_conversions(true);
    }

    #[test]
    fn try_into_string_copies_through_owning_pointers() {
        check_pointer_conversions(false);
    }
}
