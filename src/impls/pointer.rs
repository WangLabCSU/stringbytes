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

    use super::super::test_support::assert_reuses_allocation;
    use crate::StringBytes;

    fn for_each_borrowed_pointer(mut check: impl FnMut(&dyn StringBytes, &[u8], *const u8)) {
        for input in [
            b"".as_slice(),
            b"text",
            "é说明".as_bytes(),
            b"\0e\xcc\x81",
            b"ok\xff",
        ] {
            let mut bytes = Vec::from(input);
            let address = bytes.as_ptr();
            check(&&bytes, input, address);
            check(&&mut bytes, input, address);
            check(&input, input, input.as_ptr());

            let boxed = Box::<[u8]>::from(input);
            let address = boxed.as_ptr();
            check(&boxed, input, address);
            let rc = Rc::<[u8]>::from(input);
            let address = rc.as_ptr();
            check(&rc, input, address);
            let arc = Arc::<[u8]>::from(input);
            let address = arc.as_ptr();
            check(&arc, input, address);

            let bytes = Vec::from(input);
            let address = bytes.as_ptr();
            let nested = Rc::new(Arc::new(Box::new(bytes)));
            check(&nested, input, address);

            let bytes = Vec::from(input);
            let address = bytes.as_ptr();
            let boxed: Box<dyn StringBytes> = Box::new(bytes);
            check(&boxed, input, address);
            let bytes = Vec::from(input);
            let address = bytes.as_ptr();
            let rc: Rc<dyn StringBytes> = Rc::new(bytes);
            check(&rc, input, address);
            let bytes = Vec::from(input);
            let address = bytes.as_ptr();
            let arc: Arc<dyn StringBytes + Send + Sync> = Arc::new(bytes);
            check(&arc, input, address);
        }

        for input in ["", "text", "é说明", "\0e\u{301}"] {
            let mut text = String::from(input);
            let address = text.as_ptr();
            check(&&text, input.as_bytes(), address);
            check(&&mut text, input.as_bytes(), address);
            check(&input, input.as_bytes(), input.as_ptr());
            check(&text.as_mut_str(), input.as_bytes(), address);

            let boxed = Box::<str>::from(input);
            let address = boxed.as_ptr();
            check(&boxed, input.as_bytes(), address);
            let rc = Rc::<str>::from(input);
            let address = rc.as_ptr();
            check(&rc, input.as_bytes(), address);
            let arc = Arc::<str>::from(input);
            let address = arc.as_ptr();
            check(&arc, input.as_bytes(), address);
        }
    }

    fn check_allocation_reuse(into_bytes: bool) {
        fn check<T: StringBytes>(storage: T, expected: &[u8], capacity: usize, into_bytes: bool) {
            let address = storage.as_bytes().as_ptr();
            assert_reuses_allocation(storage, expected, address, capacity, into_bytes);
        }

        fn check_owners<T: StringBytes + Send + Sync + 'static>(
            make: impl Fn() -> (T, usize),
            expected: &[u8],
            into_bytes: bool,
        ) {
            // Each wrapper receives fresh, uniquely owned storage.
            let (storage, capacity) = make();
            check(Box::new(storage), expected, capacity, into_bytes);
            let (storage, capacity) = make();
            let boxed: Box<dyn StringBytes> = Box::new(storage);
            check(boxed, expected, capacity, into_bytes);

            // Weak references must not force a copy, even after type erasure.
            let (storage, capacity) = make();
            let rc = Rc::new(storage);
            let weak = Rc::downgrade(&rc);
            check(rc, expected, capacity, into_bytes);
            assert!(weak.upgrade().is_none());
            let (storage, capacity) = make();
            let rc: Rc<dyn StringBytes> = Rc::new(storage);
            let weak = Rc::downgrade(&rc);
            check(rc, expected, capacity, into_bytes);
            assert!(weak.upgrade().is_none());

            let (storage, capacity) = make();
            let arc = Arc::new(storage);
            let weak = Arc::downgrade(&arc);
            check(arc, expected, capacity, into_bytes);
            assert!(weak.upgrade().is_none());
            let (storage, capacity) = make();
            let arc: Arc<dyn StringBytes + Send + Sync> = Arc::new(storage);
            let weak = Arc::downgrade(&arc);
            check(arc, expected, capacity, into_bytes);
            assert!(weak.upgrade().is_none());

            let (storage, capacity) = make();
            let nested: Box<dyn StringBytes> = Box::new(Rc::new(Arc::new(Box::new(storage))));
            check(nested, expected, capacity, into_bytes);
        }

        for input in [b"".as_slice(), b"text", "é说明".as_bytes(), b"ok\xff"] {
            let make_vec = || {
                let mut bytes = Vec::with_capacity(input.len() + 32);
                bytes.extend_from_slice(input);
                let capacity = bytes.capacity();
                (bytes, capacity)
            };
            check_owners(make_vec, input, into_bytes);
            check_owners(
                || (Box::<[u8]>::from(input), input.len()),
                input,
                into_bytes,
            );
            check_owners(
                || {
                    let (bytes, capacity) = make_vec();
                    (Cow::<'static, [u8]>::Owned(bytes), capacity)
                },
                input,
                into_bytes,
            );

            if let Ok(text) = std::str::from_utf8(input) {
                let make_string = || {
                    let mut value = String::with_capacity(text.len() + 32);
                    value.push_str(text);
                    let capacity = value.capacity();
                    (value, capacity)
                };
                check_owners(make_string, input, into_bytes);
                check_owners(|| (Box::<str>::from(text), text.len()), input, into_bytes);
                check_owners(
                    || {
                        let (text, capacity) = make_string();
                        (Cow::<'static, str>::Owned(text), capacity)
                    },
                    input,
                    into_bytes,
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
                    into_bytes,
                );
                check_owners(
                    || (Bytes::from(Box::<[u8]>::from(input)), input.len()),
                    input,
                    into_bytes,
                );
                let make_mut = || {
                    let mut bytes = BytesMut::with_capacity(input.len() + 32);
                    bytes.extend_from_slice(input);
                    let capacity = bytes.capacity();
                    (bytes, capacity)
                };
                check_owners(make_mut, input, into_bytes);
                check_owners(
                    || {
                        let (bytes, capacity) = make_mut();
                        (bytes.freeze(), capacity)
                    },
                    input,
                    into_bytes,
                );
            }
        }

        check_owners(|| (Box::new(*b"text"), 4), b"text", into_bytes);
        check_owners(|| (Box::new([b'a', 0xff]), 2), b"a\xff", into_bytes);
        check_owners(|| (Box::new([0_u8; 0]), 0), b"", into_bytes);
    }

    #[test]
    fn as_bytes_forwards_the_original_slice() {
        for_each_borrowed_pointer(|storage, expected, address| {
            for _ in 0..3 {
                let bytes = StringBytes::as_bytes(storage);
                assert_eq!(bytes, expected);
                assert_eq!(bytes.as_ptr(), address);
            }
        });
    }

    #[test]
    fn to_bytes_copies_and_preserves_the_source() {
        for_each_borrowed_pointer(|storage, expected, address| {
            let mut bytes = StringBytes::to_bytes(storage);
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
    fn into_bytes_copies_borrowed_and_shared_storage() {
        fn check<T: StringBytes>(storage: T, expected: &[u8]) {
            let address = storage.as_bytes().as_ptr();
            let bytes = storage.into_bytes();
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
        check_allocation_reuse(true);
    }

    #[test]
    fn try_borrow_str_forwards_contents_and_errors() {
        for_each_borrowed_pointer(|storage, expected, address| {
            match (
                StringBytes::try_borrow_str(storage),
                std::str::from_utf8(expected),
            ) {
                (Ok(actual), Ok(expected)) => {
                    assert_eq!(actual, expected);
                    assert_eq!(actual.as_ptr(), address);
                }
                (Err(actual), Err(expected)) => assert_eq!(actual, expected),
                result => panic!("unexpected UTF-8 conversion: {result:?}"),
            }
        });
    }

    #[test]
    fn try_to_string_copies_and_preserves_the_source() {
        for_each_borrowed_pointer(|storage, expected, address| {
            let mut bytes = match StringBytes::try_to_string(storage) {
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
            };
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
            let text = storage.try_into_string().unwrap();
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
        check_allocation_reuse(false);
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

    #[test]
    fn as_ref_borrows_the_same_slice_as_as_bytes() {
        fn check<T: StringBytes + AsRef<[u8]> + ?Sized>(storage: &T) {
            assert!(std::ptr::eq(
                StringBytes::as_bytes(storage),
                storage.as_ref()
            ));
        }

        for input in [b"".as_slice(), b"AC", b"A\xff"] {
            check(&input);
            check(&Box::<[u8]>::from(input));
            check(&Rc::<[u8]>::from(input));
            check(&Arc::<[u8]>::from(input));
        }
    }

    #[test]
    fn as_mut_borrows_the_same_slice_as_as_bytes() {
        fn check<T: StringBytes + AsMut<[u8]> + ?Sized>(storage: &mut T) {
            let address = storage.as_bytes().as_ptr();
            let expected: Vec<u8> = storage
                .as_bytes()
                .iter()
                .map(|byte| byte.wrapping_add(1))
                .collect();
            let bytes = storage.as_mut();
            assert_eq!(bytes.as_ptr(), address);
            assert_eq!(bytes.len(), expected.len());
            for byte in bytes {
                *byte = byte.wrapping_add(1);
            }
            assert_eq!(storage.as_bytes(), expected);
        }

        for input in [b"".as_slice(), b"AC", b"A\xff"] {
            let mut bytes = Vec::from(input);
            check(&mut bytes.as_mut_slice());
            check(&mut &mut bytes);
            check(&mut Box::<[u8]>::from(input));
        }
    }

    #[test]
    fn from_preserves_the_source_bytes() {
        fn check<S: StringBytes, T: StringBytes + From<S>>(source: S) {
            let expected = source.as_bytes().to_vec();
            assert_eq!(T::from(source).as_bytes(), expected);
        }

        for input in [b"".as_slice(), b"AC", b"A\xff"] {
            check::<_, &[u8]>(input);
            check::<_, Box<[u8]>>(input);
            check::<_, Rc<[u8]>>(input);
            check::<_, Arc<[u8]>>(input);
            check::<_, Box<&[u8]>>(input);
            check::<_, Rc<&[u8]>>(input);
            check::<_, Arc<&[u8]>>(input);
            check::<_, Box<[u8]>>(Vec::from(input));
            check::<_, Rc<[u8]>>(Vec::from(input));
            check::<_, Arc<[u8]>>(Vec::from(input));
            check::<_, Rc<[u8]>>(Box::<[u8]>::from(input));
            check::<_, Arc<[u8]>>(Box::<[u8]>::from(input));
        }
        check::<_, Box<[u8]>>([0xff; 2]);
    }
}
