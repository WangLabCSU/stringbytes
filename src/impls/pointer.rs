use std::rc::Rc;
use std::str::Utf8Error;
use std::string::FromUtf8Error;
use std::sync::Arc;

use super::impl_pointer_conversions_for_sized;
use crate::{sealed, StringBytes};

// A blanket Deref implementation could expose different bytes on each call.
// Limit delegation to the pointer types below.
macro_rules! impl_string_bytes_for_pointer {
    ($($pointer:ty => $into_bytes:ident, $into_string:ident),+ $(,)?) => {
        $(
            impl<T: StringBytes + ?Sized> sealed::Sealed for $pointer {}

            impl<T: StringBytes + ?Sized> StringBytes for $pointer {
                #[inline]
                fn as_bytes(&self) -> &[u8] {
                    // Call T's implementation to avoid recursing through this wrapper.
                    T::as_bytes(&**self)
                }

                #[inline]
                fn to_bytes(&self) -> Vec<u8> {
                    T::to_bytes(&**self)
                }

                #[inline]
                fn into_bytes(self) -> Vec<u8> {
                    T::$into_bytes(self)
                }

                #[inline]
                fn try_borrow_str(&self) -> Result<&str, Utf8Error> {
                    // Preserve T's fast path for text that is already valid UTF-8.
                    T::try_borrow_str(&**self)
                }

                #[inline]
                fn try_to_string(&self) -> Result<String, FromUtf8Error> {
                    T::try_to_string(&**self)
                }

                #[inline]
                fn try_into_string(self) -> Result<String, FromUtf8Error> {
                    T::$into_string(self)
                }

                // Support nested owners such as Rc<Box<str>>.
                impl_pointer_conversions_for_sized!();
            }
        )+
    };
}

impl_string_bytes_for_pointer!(&T => to_bytes, try_to_string);
impl_string_bytes_for_pointer!(&mut T => to_bytes, try_to_string);
impl_string_bytes_for_pointer!(Box<T> => box_into_bytes, box_into_string);
impl_string_bytes_for_pointer!(Rc<T> => rc_into_bytes, rc_into_string);
impl_string_bytes_for_pointer!(Arc<T> => arc_into_bytes, arc_into_string);

#[cfg(test)]
mod tests {
    use std::borrow::Cow;
    use std::rc::Rc;
    use std::sync::Arc;

    #[cfg(feature = "bytes")]
    use bytes::{Bytes, BytesMut};

    use crate::test_utils::{
        assert_as_mut_matches_as_bytes, assert_as_ref_matches_as_bytes,
        assert_deref_matches_as_bytes, assert_into_bytes_preserves_bytes,
        assert_into_bytes_reuses_allocation, assert_stable_bytes, assert_to_bytes_preserves_bytes,
        assert_try_borrow_str_preserves_bytes, assert_try_into_string_preserves_bytes,
        assert_try_into_string_reuses_allocation, assert_try_to_string_preserves_bytes,
    };
    use crate::StringBytes;

    // Include NUL, decomposed Unicode, and invalid, incomplete, and overlong UTF-8.
    const INPUTS: [&[u8]; 8] = [
        b"",
        b"text",
        "é说明".as_bytes(),
        b"\0e\xcc\x81",
        b"ok\xff",
        b"ok\xe2\x82",
        b"\xc0\x80",
        b"\xed\xa0\x80",
    ];

    #[test]
    fn shared_reference_guarantees() {
        for input in INPUTS {
            let storage = input;
            assert_eq!(StringBytes::as_bytes(&storage), input);
            assert_stable_bytes(&storage);
            assert_as_ref_matches_as_bytes(&storage);
            assert_deref_matches_as_bytes(&storage);
            assert_to_bytes_preserves_bytes(&storage);
            let _ = assert_try_borrow_str_preserves_bytes(&storage);
            let _ = assert_try_to_string_preserves_bytes(&storage);
            assert_into_bytes_preserves_bytes(storage);
            let _ = assert_try_into_string_preserves_bytes(storage);
        }

        for input in INPUTS {
            let source = input.to_vec();
            let storage = &source;
            assert_eq!(StringBytes::as_bytes(&storage), input);
            assert_stable_bytes(&storage);
            assert_as_ref_matches_as_bytes(&storage);
            assert_deref_matches_as_bytes(&storage);
            assert_to_bytes_preserves_bytes(&storage);
            let _ = assert_try_borrow_str_preserves_bytes(&storage);
            let _ = assert_try_to_string_preserves_bytes(&storage);
            assert_into_bytes_preserves_bytes(storage);
            let _ = assert_try_into_string_preserves_bytes(storage);
        }

        for input in [*b"text", *b"ok\xff\0"] {
            let source = input;
            let storage = &source;
            assert_eq!(StringBytes::as_bytes(&storage), input);
            assert_stable_bytes(&storage);
            assert_as_ref_matches_as_bytes(&storage);
            assert_deref_matches_as_bytes(&storage);
            assert_to_bytes_preserves_bytes(&storage);
            let _ = assert_try_borrow_str_preserves_bytes(&storage);
            let _ = assert_try_to_string_preserves_bytes(&storage);
            assert_into_bytes_preserves_bytes(storage);
            let _ = assert_try_into_string_preserves_bytes(storage);
        }

        for input in ["", "text", "é说明", "\0e\u{301}"] {
            let storage = input;
            assert_eq!(StringBytes::as_bytes(&storage), input.as_bytes());
            assert_stable_bytes(&storage);
            assert_as_ref_matches_as_bytes(&storage);
            assert_deref_matches_as_bytes(&storage);
            assert_to_bytes_preserves_bytes(&storage);
            let _ = assert_try_borrow_str_preserves_bytes(&storage);
            let _ = assert_try_to_string_preserves_bytes(&storage);
            assert_into_bytes_preserves_bytes(storage);
            let _ = assert_try_into_string_preserves_bytes(storage);
        }

        for input in ["", "text", "é说明", "\0e\u{301}"] {
            let source = String::from(input);
            let storage = &source;
            assert_eq!(StringBytes::as_bytes(&storage), input.as_bytes());
            assert_stable_bytes(&storage);
            assert_as_ref_matches_as_bytes(&storage);
            assert_deref_matches_as_bytes(&storage);
            assert_to_bytes_preserves_bytes(&storage);
            let _ = assert_try_borrow_str_preserves_bytes(&storage);
            let _ = assert_try_to_string_preserves_bytes(&storage);
            assert_into_bytes_preserves_bytes(storage);
            let _ = assert_try_into_string_preserves_bytes(storage);
        }

        for input in INPUTS {
            let source = input.to_vec();
            let storage: &dyn StringBytes = &source;
            assert_eq!(StringBytes::as_bytes(&storage), input);
            assert_stable_bytes(&storage);
            assert_deref_matches_as_bytes(&storage);
            assert_to_bytes_preserves_bytes(&storage);
            let _ = assert_try_borrow_str_preserves_bytes(&storage);
            let _ = assert_try_to_string_preserves_bytes(&storage);
            assert_into_bytes_preserves_bytes(storage);
            let _ = assert_try_into_string_preserves_bytes(storage);
        }
    }

    #[test]
    fn mutable_reference_guarantees() {
        for input in INPUTS {
            let mut source = input.to_vec();
            let mut storage = source.as_mut_slice();
            assert_eq!(StringBytes::as_bytes(&storage), input);
            assert_stable_bytes(&storage);
            assert_as_ref_matches_as_bytes(&storage);
            assert_deref_matches_as_bytes(&storage);
            assert_to_bytes_preserves_bytes(&storage);
            let _ = assert_try_borrow_str_preserves_bytes(&storage);
            let _ = assert_try_to_string_preserves_bytes(&storage);
            assert_into_bytes_preserves_bytes(&mut *storage);
            let _ = assert_try_into_string_preserves_bytes(&mut *storage);
            assert_as_mut_matches_as_bytes(&mut storage);
        }

        for input in INPUTS {
            let mut source = input.to_vec();
            let mut storage = &mut source;
            assert_eq!(StringBytes::as_bytes(&storage), input);
            assert_stable_bytes(&storage);
            assert_as_ref_matches_as_bytes(&storage);
            assert_deref_matches_as_bytes(&storage);
            assert_to_bytes_preserves_bytes(&storage);
            let _ = assert_try_borrow_str_preserves_bytes(&storage);
            let _ = assert_try_to_string_preserves_bytes(&storage);
            assert_into_bytes_preserves_bytes(&mut *storage);
            let _ = assert_try_into_string_preserves_bytes(&mut *storage);
            assert_as_mut_matches_as_bytes(&mut storage);
        }

        for input in [*b"text", *b"ok\xff\0"] {
            let mut source = input;
            let mut storage = &mut source;
            assert_eq!(StringBytes::as_bytes(&storage), input);
            assert_stable_bytes(&storage);
            assert_as_ref_matches_as_bytes(&storage);
            assert_deref_matches_as_bytes(&storage);
            assert_to_bytes_preserves_bytes(&storage);
            let _ = assert_try_borrow_str_preserves_bytes(&storage);
            let _ = assert_try_to_string_preserves_bytes(&storage);
            assert_into_bytes_preserves_bytes::<&mut [u8; 4]>(&mut *storage);
            let _ = assert_try_into_string_preserves_bytes::<&mut [u8; 4]>(&mut *storage);
            assert_as_mut_matches_as_bytes(&mut storage);
        }

        for input in ["", "text", "é说明", "\0e\u{301}"] {
            let mut source = String::from(input);
            let storage = source.as_mut_str();
            assert_eq!(StringBytes::as_bytes(&storage), input.as_bytes());
            assert_stable_bytes(&storage);
            assert_as_ref_matches_as_bytes(&storage);
            assert_deref_matches_as_bytes(&storage);
            assert_to_bytes_preserves_bytes(&storage);
            let _ = assert_try_borrow_str_preserves_bytes(&storage);
            let _ = assert_try_to_string_preserves_bytes(&storage);
            assert_into_bytes_preserves_bytes(&mut *storage);
            let _ = assert_try_into_string_preserves_bytes(&mut *storage);
        }

        for input in ["", "text", "é说明", "\0e\u{301}"] {
            let mut source = String::from(input);
            let storage = &mut source;
            assert_eq!(StringBytes::as_bytes(&storage), input.as_bytes());
            assert_stable_bytes(&storage);
            assert_as_ref_matches_as_bytes(&storage);
            assert_deref_matches_as_bytes(&storage);
            assert_to_bytes_preserves_bytes(&storage);
            let _ = assert_try_borrow_str_preserves_bytes(&storage);
            let _ = assert_try_to_string_preserves_bytes(&storage);
            assert_into_bytes_preserves_bytes(&mut *storage);
            let _ = assert_try_into_string_preserves_bytes(&mut *storage);
        }

        for input in INPUTS {
            let mut source = input.to_vec();
            let storage: &mut dyn StringBytes = &mut source;
            assert_eq!(StringBytes::as_bytes(&storage), input);
            assert_stable_bytes(&storage);
            assert_deref_matches_as_bytes(&storage);
            assert_to_bytes_preserves_bytes(&storage);
            let _ = assert_try_borrow_str_preserves_bytes(&storage);
            let _ = assert_try_to_string_preserves_bytes(&storage);
            assert_into_bytes_preserves_bytes(&mut *storage);
            let _ = assert_try_into_string_preserves_bytes(&mut *storage);
        }
    }

    #[test]
    fn box_guarantees() {
        for input in INPUTS {
            let mut storage = Box::<[u8]>::from(input);
            assert_eq!(StringBytes::as_bytes(&storage), input);
            assert_stable_bytes(&storage);
            assert_as_ref_matches_as_bytes(&storage);
            assert_deref_matches_as_bytes(&storage);
            assert_to_bytes_preserves_bytes(&storage);
            let _ = assert_try_borrow_str_preserves_bytes(&storage);
            let _ = assert_try_to_string_preserves_bytes(&storage);
            assert_into_bytes_preserves_bytes(storage.clone());
            let _ = assert_try_into_string_preserves_bytes(storage.clone());
            assert_as_mut_matches_as_bytes(&mut storage);
        }

        for input in INPUTS {
            let storage = Box::new(input.to_vec());
            assert_eq!(StringBytes::as_bytes(&storage), input);
            assert_stable_bytes(&storage);
            assert_deref_matches_as_bytes(&storage);
            assert_to_bytes_preserves_bytes(&storage);
            let _ = assert_try_borrow_str_preserves_bytes(&storage);
            let _ = assert_try_to_string_preserves_bytes(&storage);
            assert_into_bytes_preserves_bytes(storage.clone());
            let _ = assert_try_into_string_preserves_bytes(storage);
        }

        for input in [*b"text", *b"ok\xff\0"] {
            let storage = Box::new(input);
            assert_eq!(StringBytes::as_bytes(&storage), input);
            assert_stable_bytes(&storage);
            assert_deref_matches_as_bytes(&storage);
            assert_to_bytes_preserves_bytes(&storage);
            let _ = assert_try_borrow_str_preserves_bytes(&storage);
            let _ = assert_try_to_string_preserves_bytes(&storage);
            assert_into_bytes_preserves_bytes(storage.clone());
            let _ = assert_try_into_string_preserves_bytes(storage);
        }

        for input in ["", "text", "é说明", "\0e\u{301}"] {
            let storage = Box::<str>::from(input);
            assert_eq!(StringBytes::as_bytes(&storage), input.as_bytes());
            assert_stable_bytes(&storage);
            assert_deref_matches_as_bytes(&storage);
            assert_to_bytes_preserves_bytes(&storage);
            let _ = assert_try_borrow_str_preserves_bytes(&storage);
            let _ = assert_try_to_string_preserves_bytes(&storage);
            assert_into_bytes_preserves_bytes(storage.clone());
            let _ = assert_try_into_string_preserves_bytes(storage);
        }

        for input in ["", "text", "é说明", "\0e\u{301}"] {
            let storage = Box::new(String::from(input));
            assert_eq!(StringBytes::as_bytes(&storage), input.as_bytes());
            assert_stable_bytes(&storage);
            assert_deref_matches_as_bytes(&storage);
            assert_to_bytes_preserves_bytes(&storage);
            let _ = assert_try_borrow_str_preserves_bytes(&storage);
            let _ = assert_try_to_string_preserves_bytes(&storage);
            assert_into_bytes_preserves_bytes(storage.clone());
            let _ = assert_try_into_string_preserves_bytes(storage);
        }
    }

    #[test]
    fn box_dyn_guarantees() {
        for input in INPUTS {
            let storage: Box<dyn StringBytes> = Box::new(input.to_vec());
            assert_eq!(StringBytes::as_bytes(&storage), input);
            assert_stable_bytes(&storage);
            assert_deref_matches_as_bytes(&storage);
            assert_to_bytes_preserves_bytes(&storage);
            let _ = assert_try_borrow_str_preserves_bytes(&storage);
            let _ = assert_try_to_string_preserves_bytes(&storage);
            assert_into_bytes_preserves_bytes(storage);
            let storage: Box<dyn StringBytes> = Box::new(input.to_vec());
            let _ = assert_try_into_string_preserves_bytes(storage);
        }
    }

    #[test]
    fn rc_guarantees() {
        for input in INPUTS {
            let storage = Rc::<[u8]>::from(input);
            assert_eq!(StringBytes::as_bytes(&storage), input);
            assert_stable_bytes(&storage);
            assert_as_ref_matches_as_bytes(&storage);
            assert_deref_matches_as_bytes(&storage);
            assert_to_bytes_preserves_bytes(&storage);
            let _ = assert_try_borrow_str_preserves_bytes(&storage);
            let _ = assert_try_to_string_preserves_bytes(&storage);
            assert_into_bytes_preserves_bytes(storage.clone());
            let _ = assert_try_into_string_preserves_bytes(storage);
        }

        for input in INPUTS {
            let storage = Rc::new(input.to_vec());
            assert_eq!(StringBytes::as_bytes(&storage), input);
            assert_stable_bytes(&storage);
            assert_deref_matches_as_bytes(&storage);
            assert_to_bytes_preserves_bytes(&storage);
            let _ = assert_try_borrow_str_preserves_bytes(&storage);
            let _ = assert_try_to_string_preserves_bytes(&storage);
            assert_into_bytes_preserves_bytes(storage.clone());
            let _ = assert_try_into_string_preserves_bytes(storage);
        }

        for input in [*b"text", *b"ok\xff\0"] {
            let storage = Rc::new(input);
            assert_eq!(StringBytes::as_bytes(&storage), input);
            assert_stable_bytes(&storage);
            assert_deref_matches_as_bytes(&storage);
            assert_to_bytes_preserves_bytes(&storage);
            let _ = assert_try_borrow_str_preserves_bytes(&storage);
            let _ = assert_try_to_string_preserves_bytes(&storage);
            assert_into_bytes_preserves_bytes(storage.clone());
            let _ = assert_try_into_string_preserves_bytes(storage);
        }

        for input in ["", "text", "é说明", "\0e\u{301}"] {
            let storage = Rc::<str>::from(input);
            assert_eq!(StringBytes::as_bytes(&storage), input.as_bytes());
            assert_stable_bytes(&storage);
            assert_deref_matches_as_bytes(&storage);
            assert_to_bytes_preserves_bytes(&storage);
            let _ = assert_try_borrow_str_preserves_bytes(&storage);
            let _ = assert_try_to_string_preserves_bytes(&storage);
            assert_into_bytes_preserves_bytes(storage.clone());
            let _ = assert_try_into_string_preserves_bytes(storage);
        }

        for input in ["", "text", "é说明", "\0e\u{301}"] {
            let storage = Rc::new(String::from(input));
            assert_eq!(StringBytes::as_bytes(&storage), input.as_bytes());
            assert_stable_bytes(&storage);
            assert_deref_matches_as_bytes(&storage);
            assert_to_bytes_preserves_bytes(&storage);
            let _ = assert_try_borrow_str_preserves_bytes(&storage);
            let _ = assert_try_to_string_preserves_bytes(&storage);
            assert_into_bytes_preserves_bytes(storage.clone());
            let _ = assert_try_into_string_preserves_bytes(storage);
        }
    }

    #[test]
    fn rc_dyn_guarantees() {
        for input in INPUTS {
            let storage: Rc<dyn StringBytes> = Rc::new(input.to_vec());
            assert_eq!(StringBytes::as_bytes(&storage), input);
            assert_stable_bytes(&storage);
            assert_deref_matches_as_bytes(&storage);
            assert_to_bytes_preserves_bytes(&storage);
            let _ = assert_try_borrow_str_preserves_bytes(&storage);
            let _ = assert_try_to_string_preserves_bytes(&storage);
            assert_into_bytes_preserves_bytes(storage);
            let storage: Rc<dyn StringBytes> = Rc::new(input.to_vec());
            let _ = assert_try_into_string_preserves_bytes(storage);
        }
    }

    #[test]
    fn arc_guarantees() {
        for input in INPUTS {
            let storage = Arc::<[u8]>::from(input);
            assert_eq!(StringBytes::as_bytes(&storage), input);
            assert_stable_bytes(&storage);
            assert_as_ref_matches_as_bytes(&storage);
            assert_deref_matches_as_bytes(&storage);
            assert_to_bytes_preserves_bytes(&storage);
            let _ = assert_try_borrow_str_preserves_bytes(&storage);
            let _ = assert_try_to_string_preserves_bytes(&storage);
            assert_into_bytes_preserves_bytes(storage.clone());
            let _ = assert_try_into_string_preserves_bytes(storage);
        }

        for input in INPUTS {
            let storage = Arc::new(input.to_vec());
            assert_eq!(StringBytes::as_bytes(&storage), input);
            assert_stable_bytes(&storage);
            assert_deref_matches_as_bytes(&storage);
            assert_to_bytes_preserves_bytes(&storage);
            let _ = assert_try_borrow_str_preserves_bytes(&storage);
            let _ = assert_try_to_string_preserves_bytes(&storage);
            assert_into_bytes_preserves_bytes(storage.clone());
            let _ = assert_try_into_string_preserves_bytes(storage);
        }

        for input in [*b"text", *b"ok\xff\0"] {
            let storage = Arc::new(input);
            assert_eq!(StringBytes::as_bytes(&storage), input);
            assert_stable_bytes(&storage);
            assert_deref_matches_as_bytes(&storage);
            assert_to_bytes_preserves_bytes(&storage);
            let _ = assert_try_borrow_str_preserves_bytes(&storage);
            let _ = assert_try_to_string_preserves_bytes(&storage);
            assert_into_bytes_preserves_bytes(storage.clone());
            let _ = assert_try_into_string_preserves_bytes(storage);
        }

        for input in ["", "text", "é说明", "\0e\u{301}"] {
            let storage = Arc::<str>::from(input);
            assert_eq!(StringBytes::as_bytes(&storage), input.as_bytes());
            assert_stable_bytes(&storage);
            assert_deref_matches_as_bytes(&storage);
            assert_to_bytes_preserves_bytes(&storage);
            let _ = assert_try_borrow_str_preserves_bytes(&storage);
            let _ = assert_try_to_string_preserves_bytes(&storage);
            assert_into_bytes_preserves_bytes(storage.clone());
            let _ = assert_try_into_string_preserves_bytes(storage);
        }

        for input in ["", "text", "é说明", "\0e\u{301}"] {
            let storage = Arc::new(String::from(input));
            assert_eq!(StringBytes::as_bytes(&storage), input.as_bytes());
            assert_stable_bytes(&storage);
            assert_deref_matches_as_bytes(&storage);
            assert_to_bytes_preserves_bytes(&storage);
            let _ = assert_try_borrow_str_preserves_bytes(&storage);
            let _ = assert_try_to_string_preserves_bytes(&storage);
            assert_into_bytes_preserves_bytes(storage.clone());
            let _ = assert_try_into_string_preserves_bytes(storage);
        }
    }

    #[test]
    fn arc_dyn_guarantees() {
        for input in INPUTS {
            let storage: Arc<dyn StringBytes + Send + Sync> = Arc::new(input.to_vec());
            assert_eq!(StringBytes::as_bytes(&storage), input);
            assert_stable_bytes(&storage);
            assert_deref_matches_as_bytes(&storage);
            assert_to_bytes_preserves_bytes(&storage);
            let _ = assert_try_borrow_str_preserves_bytes(&storage);
            let _ = assert_try_to_string_preserves_bytes(&storage);
            assert_into_bytes_preserves_bytes(storage);
            let storage: Arc<dyn StringBytes + Send + Sync> = Arc::new(input.to_vec());
            let _ = assert_try_into_string_preserves_bytes(storage);
        }
    }

    #[test]
    fn nested_pointer_guarantees() {
        for input in INPUTS {
            let storage = Rc::new(Arc::new(Box::new(input.to_vec())));
            assert_eq!(StringBytes::as_bytes(&storage), input);
            assert_stable_bytes(&storage);
            assert_deref_matches_as_bytes(&storage);
            assert_to_bytes_preserves_bytes(&storage);
            let _ = assert_try_borrow_str_preserves_bytes(&storage);
            let _ = assert_try_to_string_preserves_bytes(&storage);
            assert_into_bytes_preserves_bytes(storage.clone());
            let _ = assert_try_into_string_preserves_bytes(storage);
        }
    }

    #[test]
    fn into_bytes_copies_borrowed_and_shared_storage() {
        fn check<T: StringBytes>(storage: T, expected: &[u8]) {
            let address = storage.as_bytes().as_ptr();
            let bytes = assert_into_bytes_preserves_bytes(storage);
            assert_eq!(bytes, expected);
            if !bytes.is_empty() {
                assert_ne!(bytes.as_ptr(), address);
            }
        }

        for input in [b"".as_slice(), b"text", b"ok\xff"] {
            check(input, input);

            let mut storage = Vec::from(input);
            check(&storage, input);
            check(&mut storage, input);
            assert_eq!(storage, input);

            let rc = Rc::<[u8]>::from(input);
            check(Rc::clone(&rc), input);
            assert_eq!(&*rc, input);
            check(rc, input);
            let arc = Arc::<[u8]>::from(input);
            check(Arc::clone(&arc), input);
            assert_eq!(&*arc, input);
            check(arc, input);

            let rc = Rc::new(Vec::from(input));
            check(Rc::clone(&rc), input);
            assert_eq!(&**rc, input);
            let arc = Arc::new(Vec::from(input));
            check(Arc::clone(&arc), input);
            assert_eq!(&**arc, input);

            let rc: Rc<dyn StringBytes> = Rc::new(Vec::from(input));
            check(Rc::clone(&rc), input);
            assert_eq!(rc.as_bytes(), input);
            let arc: Arc<dyn StringBytes + Send + Sync> = Arc::new(Vec::from(input));
            check(Arc::clone(&arc), input);
            assert_eq!(arc.as_bytes(), input);

            let nested = Rc::new(Arc::new(Box::new(Vec::from(input))));
            check(Rc::clone(&nested), input);
            assert_eq!(nested.as_bytes(), input);
        }

        for text in ["", "é说明", "\0e\u{301}"] {
            let input = text.as_bytes();
            check(text, input);
            let mut storage = String::from(text);
            check(&storage, input);
            check(&mut storage, input);
            assert_eq!(storage, text);

            let rc = Rc::<str>::from(text);
            check(Rc::clone(&rc), input);
            assert_eq!(&*rc, text);
            check(rc, input);
            let arc = Arc::<str>::from(text);
            check(Arc::clone(&arc), input);
            assert_eq!(&*arc, text);
            check(arc, input);
        }
    }

    #[test]
    fn into_bytes_reuses_allocations_through_wrappers() {
        fn check_owners<T: StringBytes + Send + Sync + 'static>(
            make: impl Fn() -> (T, usize),
            expected: &[u8],
        ) {
            // Each wrapper receives fresh, uniquely owned storage.
            let (storage, capacity) = make();
            let address = storage.as_bytes().as_ptr();
            assert_into_bytes_reuses_allocation(Box::new(storage), expected, address, capacity);
            let (storage, capacity) = make();
            let address = storage.as_bytes().as_ptr();
            let boxed: Box<dyn StringBytes> = Box::new(storage);
            assert_into_bytes_reuses_allocation(boxed, expected, address, capacity);

            // Weak references must not force a copy, even after type erasure.
            let (storage, capacity) = make();
            let address = storage.as_bytes().as_ptr();
            let rc = Rc::new(storage);
            let weak = Rc::downgrade(&rc);
            assert_into_bytes_reuses_allocation(rc, expected, address, capacity);
            assert!(weak.upgrade().is_none());
            let (storage, capacity) = make();
            let address = storage.as_bytes().as_ptr();
            let rc: Rc<dyn StringBytes> = Rc::new(storage);
            let weak = Rc::downgrade(&rc);
            assert_into_bytes_reuses_allocation(rc, expected, address, capacity);
            assert!(weak.upgrade().is_none());

            let (storage, capacity) = make();
            let address = storage.as_bytes().as_ptr();
            let arc = Arc::new(storage);
            let weak = Arc::downgrade(&arc);
            assert_into_bytes_reuses_allocation(arc, expected, address, capacity);
            assert!(weak.upgrade().is_none());
            let (storage, capacity) = make();
            let address = storage.as_bytes().as_ptr();
            let arc: Arc<dyn StringBytes + Send + Sync> = Arc::new(storage);
            let weak = Arc::downgrade(&arc);
            assert_into_bytes_reuses_allocation(arc, expected, address, capacity);
            assert!(weak.upgrade().is_none());

            let (storage, capacity) = make();
            let address = storage.as_bytes().as_ptr();
            let nested: Box<dyn StringBytes> = Box::new(Rc::new(Arc::new(Box::new(storage))));
            assert_into_bytes_reuses_allocation(nested, expected, address, capacity);
        }

        for input in [b"".as_slice(), b"text", "é说明".as_bytes(), b"ok\xff"] {
            let make_vec = || {
                let mut bytes = Vec::with_capacity(input.len() + 32);
                bytes.extend_from_slice(input);
                let capacity = bytes.capacity();
                (bytes, capacity)
            };
            check_owners(make_vec, input);
            check_owners(|| (Box::<[u8]>::from(input), input.len()), input);
            check_owners(
                || {
                    let (bytes, capacity) = make_vec();
                    (Cow::<'static, [u8]>::Owned(bytes), capacity)
                },
                input,
            );
            check_owners(
                || {
                    let (bytes, capacity) = make_vec();
                    (Cow::<'static, Vec<u8>>::Owned(bytes), capacity)
                },
                input,
            );

            if let Ok(text) = std::str::from_utf8(input) {
                let make_string = || {
                    let mut value = String::with_capacity(text.len() + 32);
                    value.push_str(text);
                    let capacity = value.capacity();
                    (value, capacity)
                };
                check_owners(make_string, input);
                check_owners(|| (Box::<str>::from(text), text.len()), input);
                check_owners(
                    || {
                        let (text, capacity) = make_string();
                        (Cow::<'static, str>::Owned(text), capacity)
                    },
                    input,
                );
                check_owners(
                    || {
                        let (text, capacity) = make_string();
                        (Cow::<'static, String>::Owned(text), capacity)
                    },
                    input,
                );
            }

            #[cfg(feature = "bytes")]
            {
                check_owners(
                    || {
                        let (bytes, capacity) = make_vec();
                        (Bytes::from(bytes), capacity)
                    },
                    input,
                );
                check_owners(
                    || (Bytes::from(Box::<[u8]>::from(input)), input.len()),
                    input,
                );
                let make_mut = || {
                    let mut bytes = BytesMut::with_capacity(input.len() + 32);
                    bytes.extend_from_slice(input);
                    let capacity = bytes.capacity();
                    (bytes, capacity)
                };
                check_owners(make_mut, input);
                check_owners(
                    || {
                        let (bytes, capacity) = make_mut();
                        (bytes.freeze(), capacity)
                    },
                    input,
                );
            }
        }

        check_owners(|| (Box::new(*b"text"), 4), b"text");
        check_owners(|| (Box::new([b'a', 0xff]), 2), b"a\xff");
        check_owners(|| (Box::new([0_u8; 0]), 0), b"");
    }

    #[test]
    fn try_to_and_try_into_string_preserve_text_overrides() {
        struct TextView;

        impl crate::sealed::Sealed for TextView {}

        impl StringBytes for TextView {
            fn as_bytes(&self) -> &[u8] {
                // Reference conversions must call try_to_string directly.
                panic!("reference conversion must call try_to_string")
            }

            fn try_to_string(&self) -> Result<String, std::string::FromUtf8Error> {
                Ok(String::from("étext"))
            }
        }

        fn check<T: StringBytes>(storage: T) {
            assert_eq!(storage.try_to_string().unwrap(), "étext");
            assert_eq!(storage.try_into_string().unwrap(), "étext");
        }

        let mut text = TextView;
        check(&text);
        check(&mut text);

        let nested = Rc::new(Arc::new(Box::new(TextView)));
        check(Rc::clone(&nested));
        let nested = Arc::new(Arc::new(Box::new(TextView)));
        check(Arc::clone(&nested));

        let boxed: Box<dyn StringBytes> = Box::new(TextView);
        check(&*boxed);
        let rc: Rc<dyn StringBytes> = Rc::new(TextView);
        check(&rc);
        let arc: Arc<dyn StringBytes + Send + Sync> = Arc::new(TextView);
        check(&arc);
    }

    #[test]
    fn try_to_and_try_into_string_do_not_prevalidate() {
        struct ByteView;

        impl crate::sealed::Sealed for ByteView {}

        impl StringBytes for ByteView {
            fn as_bytes(&self) -> &[u8] {
                "étext".as_bytes()
            }

            fn try_borrow_str(&self) -> Result<&str, std::str::Utf8Error> {
                // String conversions must validate their owned buffer directly.
                panic!("try_into_string must not call try_borrow_str")
            }
        }

        fn check<T: StringBytes>(storage: T) {
            assert_eq!(storage.try_to_string().unwrap(), "étext");
            assert_eq!(storage.try_into_string().unwrap(), "étext");
        }

        let mut text = ByteView;
        check(&text);
        check(&mut text);
        check(Box::new(ByteView));

        let rc = Rc::new(ByteView);
        check(Rc::clone(&rc));
        let arc = Arc::new(ByteView);
        check(Arc::clone(&arc));

        // A second owner forces nested conversions to copy.
        let nested = Rc::new(Arc::new(Box::new(ByteView)));
        check(Rc::clone(&nested));
        let nested = Arc::new(Arc::new(Box::new(ByteView)));
        check(Arc::clone(&nested));

        let boxed: Box<dyn StringBytes> = Box::new(ByteView);
        check(&*boxed);
        check(boxed);
        let rc: Rc<dyn StringBytes> = Rc::new(ByteView);
        check(Rc::clone(&rc));
        let arc: Arc<dyn StringBytes + Send + Sync> = Arc::new(ByteView);
        check(Arc::clone(&arc));
    }

    #[test]
    fn try_into_string_shared_errors_hold_a_copy() {
        fn check<T: StringBytes + Clone>(storage: T, expected: &[u8]) {
            let address = storage.as_bytes().as_ptr();
            let error = storage.clone().try_into_string().unwrap_err();
            assert_eq!(error.as_bytes(), expected);
            assert_ne!(error.as_bytes().as_ptr(), address);
            assert_eq!(storage.as_bytes(), expected);
            assert_eq!(storage.as_bytes().as_ptr(), address);
        }

        for input in [
            b"\xff".as_slice(),
            b"ok\xe2\x82",
            b"\xc0\x80",
            b"\xed\xa0\x80",
        ] {
            check(Rc::new(Vec::from(input)), input);
            check(Arc::new(Vec::from(input)), input);
            check(Rc::<[u8]>::from(input), input);
            check(Arc::<[u8]>::from(input), input);
        }
    }

    #[test]
    fn try_into_string_from_references_preserves_the_source() {
        let mut source = String::from("étext");
        let copy = source.try_to_string().unwrap();
        assert_eq!(copy, source);
        assert_ne!(copy.as_ptr(), source.as_ptr());

        let copy = (&source).try_into_string().unwrap();
        assert_eq!(copy, source);
        assert_ne!(copy.as_ptr(), source.as_ptr());

        let copy = (&mut source).try_into_string().unwrap();
        assert_eq!(copy, source);
        assert_ne!(copy.as_ptr(), source.as_ptr());

        // On failure, the error owns a copy and the source remains usable.
        let mut source = vec![b'a', 0xff];
        let error = source.try_to_string().unwrap_err();
        assert_eq!(error.utf8_error().valid_up_to(), 1);
        assert_eq!(error.as_bytes(), source);
        assert_ne!(error.as_bytes().as_ptr(), source.as_ptr());

        let error = (&source).try_into_string().unwrap_err();
        assert_eq!(error.utf8_error().valid_up_to(), 1);
        assert_eq!(error.as_bytes(), source);
        assert_ne!(error.as_bytes().as_ptr(), source.as_ptr());

        let error = (&mut source).try_into_string().unwrap_err();
        assert_eq!(error.utf8_error().valid_up_to(), 1);
        assert_eq!(error.as_bytes(), source);
        assert_ne!(error.as_bytes().as_ptr(), source.as_ptr());
    }

    #[test]
    fn try_into_string_copies_borrowed_and_shared_storage() {
        fn check<T: StringBytes>(storage: T) {
            let pointer = storage.as_bytes().as_ptr();
            let text = assert_try_into_string_preserves_bytes(storage).unwrap();
            assert_eq!(text, "text");
            // Nonempty, borrowed or shared data needs a separate allocation.
            assert_ne!(text.as_ptr(), pointer);
        }

        check("text");
        check(b"text".as_slice());

        let rc = Rc::<[u8]>::from(b"text".as_slice());
        check(Rc::clone(&rc));
        assert_eq!(&*rc, b"text");
        check(rc);
        let arc = Arc::<[u8]>::from(b"text".as_slice());
        check(Arc::clone(&arc));
        assert_eq!(&*arc, b"text");
        check(arc);

        let rc = Rc::<str>::from("text");
        check(Rc::clone(&rc));
        assert_eq!(&*rc, "text");
        check(rc);
        let arc = Arc::<str>::from("text");
        check(Arc::clone(&arc));
        assert_eq!(&*arc, "text");
        check(arc);

        // Shared ownership forces a copy even when the contents are sized.
        let rc = Rc::new(Vec::from(b"text".as_slice()));
        check(Rc::clone(&rc));
        assert_eq!(&**rc, b"text");
        let arc = Arc::new(String::from("text"));
        check(Arc::clone(&arc));
        assert_eq!(&**arc, "text");
    }

    #[test]
    fn try_into_string_reuses_allocations_through_wrappers() {
        fn check_owners<T: StringBytes + Send + Sync + 'static>(
            make: impl Fn() -> (T, usize),
            expected: &[u8],
        ) {
            // Each wrapper receives fresh, uniquely owned storage.
            let (storage, capacity) = make();
            let address = storage.as_bytes().as_ptr();
            assert_try_into_string_reuses_allocation(
                Box::new(storage),
                expected,
                address,
                capacity,
            );
            let (storage, capacity) = make();
            let address = storage.as_bytes().as_ptr();
            let boxed: Box<dyn StringBytes> = Box::new(storage);
            assert_try_into_string_reuses_allocation(boxed, expected, address, capacity);

            // Weak references must not force a copy, even after type erasure.
            let (storage, capacity) = make();
            let address = storage.as_bytes().as_ptr();
            let rc = Rc::new(storage);
            let weak = Rc::downgrade(&rc);
            assert_try_into_string_reuses_allocation(rc, expected, address, capacity);
            assert!(weak.upgrade().is_none());
            let (storage, capacity) = make();
            let address = storage.as_bytes().as_ptr();
            let rc: Rc<dyn StringBytes> = Rc::new(storage);
            let weak = Rc::downgrade(&rc);
            assert_try_into_string_reuses_allocation(rc, expected, address, capacity);
            assert!(weak.upgrade().is_none());

            let (storage, capacity) = make();
            let address = storage.as_bytes().as_ptr();
            let arc = Arc::new(storage);
            let weak = Arc::downgrade(&arc);
            assert_try_into_string_reuses_allocation(arc, expected, address, capacity);
            assert!(weak.upgrade().is_none());
            let (storage, capacity) = make();
            let address = storage.as_bytes().as_ptr();
            let arc: Arc<dyn StringBytes + Send + Sync> = Arc::new(storage);
            let weak = Arc::downgrade(&arc);
            assert_try_into_string_reuses_allocation(arc, expected, address, capacity);
            assert!(weak.upgrade().is_none());

            let (storage, capacity) = make();
            let address = storage.as_bytes().as_ptr();
            let nested: Box<dyn StringBytes> = Box::new(Rc::new(Arc::new(Box::new(storage))));
            assert_try_into_string_reuses_allocation(nested, expected, address, capacity);
        }

        for input in [b"".as_slice(), b"text", "é说明".as_bytes(), b"ok\xff"] {
            let make_vec = || {
                let mut bytes = Vec::with_capacity(input.len() + 32);
                bytes.extend_from_slice(input);
                let capacity = bytes.capacity();
                (bytes, capacity)
            };
            check_owners(make_vec, input);
            check_owners(|| (Box::<[u8]>::from(input), input.len()), input);
            check_owners(
                || {
                    let (bytes, capacity) = make_vec();
                    (Cow::<'static, [u8]>::Owned(bytes), capacity)
                },
                input,
            );
            check_owners(
                || {
                    let (bytes, capacity) = make_vec();
                    (Cow::<'static, Vec<u8>>::Owned(bytes), capacity)
                },
                input,
            );

            if let Ok(text) = std::str::from_utf8(input) {
                let make_string = || {
                    let mut value = String::with_capacity(text.len() + 32);
                    value.push_str(text);
                    let capacity = value.capacity();
                    (value, capacity)
                };
                check_owners(make_string, input);
                check_owners(|| (Box::<str>::from(text), text.len()), input);
                check_owners(
                    || {
                        let (text, capacity) = make_string();
                        (Cow::<'static, str>::Owned(text), capacity)
                    },
                    input,
                );
                check_owners(
                    || {
                        let (text, capacity) = make_string();
                        (Cow::<'static, String>::Owned(text), capacity)
                    },
                    input,
                );
            }

            #[cfg(feature = "bytes")]
            {
                check_owners(
                    || {
                        let (bytes, capacity) = make_vec();
                        (Bytes::from(bytes), capacity)
                    },
                    input,
                );
                check_owners(
                    || (Bytes::from(Box::<[u8]>::from(input)), input.len()),
                    input,
                );
                let make_mut = || {
                    let mut bytes = BytesMut::with_capacity(input.len() + 32);
                    bytes.extend_from_slice(input);
                    let capacity = bytes.capacity();
                    (bytes, capacity)
                };
                check_owners(make_mut, input);
                check_owners(
                    || {
                        let (bytes, capacity) = make_mut();
                        (bytes.freeze(), capacity)
                    },
                    input,
                );
            }
        }

        check_owners(|| (Box::new(*b"text"), 4), b"text");
        check_owners(|| (Box::new([b'a', 0xff]), 2), b"a\xff");
        check_owners(|| (Box::new([0_u8; 0]), 0), b"");
    }

    #[test]
    fn pointer_helpers_into_bytes_use_the_concrete_type() {
        macro_rules! check {
            ($pointer:ident, $convert:ident) => {
                for input in [b"".as_slice(), b"text", b"ok\xff"] {
                    let mut bytes = Vec::with_capacity(input.len() + 32);
                    bytes.extend_from_slice(input);
                    let address = bytes.as_ptr();
                    let capacity = bytes.capacity();
                    let storage: $pointer<dyn StringBytes> = $pointer::new(bytes);
                    let bytes = storage.$convert();
                    assert_eq!(bytes, input);
                    assert_eq!(bytes.as_ptr(), address);
                    assert_eq!(bytes.capacity(), capacity);
                }
            };
        }

        check!(Box, box_into_bytes);
        check!(Rc, rc_into_bytes);
        check!(Arc, arc_into_bytes);
    }

    #[test]
    fn pointer_helpers_into_string_use_the_concrete_type() {
        macro_rules! check {
            ($pointer:ident, $convert:ident) => {
                let mut bytes = Vec::with_capacity(32);
                bytes.extend_from_slice("étext".as_bytes());
                let address = bytes.as_ptr();
                let capacity = bytes.capacity();
                let storage: $pointer<dyn StringBytes> = $pointer::new(bytes);

                // Exercise the pointer methods directly through the trait object.
                let text = storage.$convert().unwrap();
                assert_eq!(text, "étext");
                assert_eq!(text.as_ptr(), address);
                assert_eq!(text.capacity(), capacity);

                let bytes = vec![b'a', 0xff];
                let address = bytes.as_ptr();
                let storage: $pointer<dyn StringBytes> = $pointer::new(bytes);
                let error = storage.$convert().unwrap_err();
                assert_eq!(error.utf8_error().valid_up_to(), 1);
                let recovered = error.into_bytes();
                assert_eq!(recovered, b"a\xff");
                assert_eq!(recovered.as_ptr(), address);
            };
        }

        check!(Box, box_into_string);
        check!(Rc, rc_into_string);
        check!(Arc, arc_into_string);

        let rc: Rc<dyn StringBytes> = Rc::new(String::from("text"));
        let text = Rc::clone(&rc).rc_into_string().unwrap();
        assert_eq!(text, "text");
        assert_eq!(rc.as_bytes(), b"text");
        assert_ne!(text.as_ptr(), rc.as_bytes().as_ptr());

        let arc: Arc<dyn StringBytes + Send + Sync> = Arc::new(String::from("text"));
        let text = Arc::clone(&arc).arc_into_string().unwrap();
        assert_eq!(text, "text");
        assert_eq!(arc.as_bytes(), b"text");
        assert_ne!(text.as_ptr(), arc.as_bytes().as_ptr());
    }
}
