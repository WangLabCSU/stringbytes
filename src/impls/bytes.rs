use std::borrow::Cow;
use std::rc::Rc;
use std::string::FromUtf8Error;
use std::sync::Arc;

#[cfg(feature = "bytes")]
use bytes::{Bytes, BytesMut};

use super::{impl_pointer_conversions_for_sized, impl_string_bytes};
use crate::{sealed, StringBytes};

// Arrays store bytes inline, but boxed arrays can give their allocation to
// the resulting vector or string.
impl_string_bytes!([const N: usize]; [u8; N] {
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        self
    }

    impl_pointer_conversions_for_sized! {
        #[inline]
        fn box_into_bytes(self: Box<Self>) -> Vec<u8> {
            // Passing self as an argument lets Box<[u8; N]> coerce to Box<[u8]>.
            <[u8]>::into_vec(self)
        }
    }
});

impl_string_bytes!([u8] {
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        self
    }

    #[inline]
    fn box_into_bytes(self: Box<Self>) -> Vec<u8> {
        // Use the slice's inherent method to transfer the allocation.
        <[u8]>::into_vec(self)
    }
});

impl_string_bytes!(Vec<u8> {
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        self.as_slice()
    }

    #[inline]
    fn into_bytes(self) -> Vec<u8> {
        self
    }

    impl_pointer_conversions_for_sized!();
});

// Neither variant owns a transferable allocation; copy directly from the
// array without first cloning it through Cow::into_owned.
impl_string_bytes!([const N: usize]; Cow<'_, [u8; N]> {
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        self.as_ref()
    }

    impl_pointer_conversions_for_sized! {
        #[inline]
        fn box_into_bytes(self: Box<Self>) -> Vec<u8> {
            // An owned array is part of the Cow enum, whose allocation cannot
            // be transferred to Vec. Copy without first moving the Cow out.
            (*self).to_bytes()
        }
    }
});

impl_string_bytes!(Cow<'_, [u8]> {
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        self.as_ref()
    }

    #[inline]
    fn into_bytes(self) -> Vec<u8> {
        // Reuse an owned Vec; copy a borrowed slice.
        self.into_owned()
    }

    impl_pointer_conversions_for_sized!();
});

impl_string_bytes!(Cow<'_, Vec<u8>> {
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        self.as_ref().as_slice()
    }

    #[inline]
    fn into_bytes(self) -> Vec<u8> {
        // Reuse an owned Vec, including its spare capacity.
        self.into_owned()
    }

    impl_pointer_conversions_for_sized!();
});

#[cfg(feature = "bytes")]
impl_string_bytes!(Bytes {
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        self.as_ref()
    }

    #[inline]
    fn into_bytes(self) -> Vec<u8> {
        // Let bytes decide whether its backing storage can be reused.
        Vec::from(self)
    }

    impl_pointer_conversions_for_sized!();
});

#[cfg(feature = "bytes")]
impl_string_bytes!(BytesMut {
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        self.as_ref()
    }

    #[inline]
    fn into_bytes(self) -> Vec<u8> {
        // Split buffers may still share an allocation. Vec::from handles
        // that sharing and any offset into the original buffer.
        Vec::from(self)
    }

    impl_pointer_conversions_for_sized!();
});

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

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

    fn check_cow_array_conversions(into_bytes: bool) {
        fn check<const N: usize>(input: [u8; N], into_bytes: bool) {
            check_copied_conversion(Cow::Borrowed(&input), &input, into_bytes);
            check_copied_conversion(Cow::<'_, [u8; N]>::Owned(input), &input, into_bytes);
        }

        check([], into_bytes);
        for input in [*b"text", *b"\0e\xcc\x81", *b"ok\xff\0", *b"ok\xe2\x82"] {
            check(input, into_bytes);
        }
    }

    fn check_boxed_cow_array_conversions(into_bytes: bool) {
        fn check<const N: usize>(input: [u8; N], into_bytes: bool) {
            for cow in [Cow::Borrowed(&input), Cow::<'_, [u8; N]>::Owned(input)] {
                check_copied_conversion(Box::new(cow), &input, into_bytes);
            }
        }

        check([], into_bytes);
        check(*b"text", into_bytes);
        check(*b"ok\xff", into_bytes);
    }

    fn check_copied_conversion<T: StringBytes>(storage: T, expected: &[u8], into_bytes: bool) {
        let address = storage.as_bytes().as_ptr();
        let bytes = if into_bytes {
            assert_into_bytes_preserves_bytes(storage)
        } else {
            match assert_try_into_string_preserves_bytes(storage) {
                Ok(text) => text.into_bytes(),
                Err(error) => error.into_bytes(),
            }
        };
        assert_eq!(bytes, expected);
        if !bytes.is_empty() {
            assert_ne!(bytes.as_ptr(), address);
        }
    }

    #[cfg(feature = "bytes")]
    fn check_nontransferable_buffers(into_bytes: bool) {
        for input in [b"".as_slice(), b"text", b"ok\xff"] {
            check_copied_conversion(Bytes::from_static(input), input, into_bytes);
            check_copied_conversion(Bytes::from_owner(Vec::from(input)), input, into_bytes);

            let bytes = Bytes::from(Vec::from(input));
            check_copied_conversion(bytes.clone(), input, into_bytes);
            assert_eq!(&bytes[..], input);

            let mut bytes = BytesMut::from(input);
            bytes.extend_from_slice(b"tail");
            check_copied_conversion(bytes.split_to(input.len()), input, into_bytes);
            assert_eq!(&bytes[..], b"tail");
        }
    }

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
    fn array_guarantees() {
        for input in [
            *b"text",
            *b"\0e\xcc\x81",
            *b"ok\xff\0",
            *b"ok\xe2\x82",
            *b"\xc0\x80\0\0",
            *b"\xed\xa0\x80\0",
        ] {
            let mut storage = input;
            assert_eq!(StringBytes::as_bytes(&storage), input);
            assert_stable_bytes(&storage);
            assert_as_ref_matches_as_bytes(&storage);
            assert_to_bytes_preserves_bytes(&storage);
            let _ = assert_try_borrow_str_preserves_bytes(&storage);
            let _ = assert_try_to_string_preserves_bytes(&storage);
            assert_into_bytes_preserves_bytes(storage);
            let _ = assert_try_into_string_preserves_bytes(storage);
            assert_as_mut_matches_as_bytes(&mut storage);
        }

        let mut storage = [0_u8; 0];
        assert_eq!(StringBytes::as_bytes(&storage), [0_u8; 0]);
        assert_stable_bytes(&storage);
        assert_as_ref_matches_as_bytes(&storage);
        assert_to_bytes_preserves_bytes(&storage);
        let _ = assert_try_borrow_str_preserves_bytes(&storage);
        let _ = assert_try_to_string_preserves_bytes(&storage);
        assert_into_bytes_preserves_bytes(storage);
        let _ = assert_try_into_string_preserves_bytes(storage);
        assert_as_mut_matches_as_bytes(&mut storage);

        let mut storage = [0xff_u8; 64];
        assert_eq!(StringBytes::as_bytes(&storage), [0xff_u8; 64]);
        assert_stable_bytes(&storage);
        assert_as_ref_matches_as_bytes(&storage);
        assert_to_bytes_preserves_bytes(&storage);
        let _ = assert_try_borrow_str_preserves_bytes(&storage);
        let _ = assert_try_to_string_preserves_bytes(&storage);
        assert_into_bytes_preserves_bytes(storage);
        let _ = assert_try_into_string_preserves_bytes(storage);
        assert_as_mut_matches_as_bytes(&mut storage);
    }

    #[test]
    fn slice_guarantees() {
        for input in INPUTS {
            let mut buffer = input.to_vec();
            let storage = buffer.as_mut_slice();
            assert_eq!(StringBytes::as_bytes(storage), input);
            assert_stable_bytes(storage);
            assert_as_ref_matches_as_bytes(storage);
            assert_to_bytes_preserves_bytes(storage);
            let _ = assert_try_borrow_str_preserves_bytes(storage);
            let _ = assert_try_to_string_preserves_bytes(storage);
            assert_as_mut_matches_as_bytes(storage);
        }
    }

    #[test]
    fn vec_guarantees() {
        for input in INPUTS {
            let mut storage = input.to_vec();
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
    }

    #[test]
    fn cow_slice_guarantees() {
        for input in INPUTS {
            for storage in [Cow::Borrowed(input), Cow::<'_, [u8]>::Owned(input.to_vec())] {
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
        }
    }

    #[test]
    fn cow_vec_guarantees() {
        for input in INPUTS {
            let vector = input.to_vec();
            for storage in [
                Cow::Borrowed(&vector),
                Cow::<'_, Vec<u8>>::Owned(input.to_vec()),
            ] {
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
    }

    #[test]
    fn cow_array_guarantees() {
        for input in [
            *b"text",
            *b"\0e\xcc\x81",
            *b"ok\xff\0",
            *b"ok\xe2\x82",
            *b"\xc0\x80\0\0",
            *b"\xed\xa0\x80\0",
        ] {
            for storage in [Cow::Borrowed(&input), Cow::<'_, [u8; 4]>::Owned(input)] {
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

        for storage in [Cow::Borrowed(&[]), Cow::<'_, [u8; 0]>::Owned([])] {
            assert_eq!(StringBytes::as_bytes(&storage), b"");
            assert_stable_bytes(&storage);
            assert_deref_matches_as_bytes(&storage);
            assert_to_bytes_preserves_bytes(&storage);
            let _ = assert_try_borrow_str_preserves_bytes(&storage);
            let _ = assert_try_to_string_preserves_bytes(&storage);
            assert_into_bytes_preserves_bytes(storage.clone());
            let _ = assert_try_into_string_preserves_bytes(storage);
        }
    }

    #[cfg(feature = "bytes")]
    #[test]
    fn bytes_guarantees() {
        for input in INPUTS {
            let original = Bytes::from([b"prefix".as_slice(), input].concat());
            for storage in [
                Bytes::from_static(input),
                Bytes::from(input.to_vec()),
                Bytes::from_owner(input.to_vec()),
                original.slice(6..),
            ] {
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
        }
    }

    #[cfg(feature = "bytes")]
    #[test]
    fn bytes_mut_guarantees() {
        for input in INPUTS {
            let mut original = BytesMut::from(input);
            original.extend_from_slice(b"tail");
            let prefix = original.split_to(input.len());
            for mut storage in [BytesMut::from(input), prefix] {
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
            assert_eq!(&original[..], b"tail");
            assert_deref_matches_as_bytes(&original);
        }
    }

    #[test]
    fn into_bytes_reuses_owned_storage() {
        for input in [
            b"".as_slice(),
            b"text",
            "é说明".as_bytes(),
            b"\0e\xcc\x81",
            b"ok\xff",
        ] {
            let make_vec = || {
                let mut bytes = Vec::with_capacity(input.len() + 32);
                bytes.extend_from_slice(input);
                bytes
            };

            let bytes = make_vec();
            let address = bytes.as_ptr();
            let capacity = bytes.capacity();
            assert_into_bytes_reuses_allocation(bytes, input, address, capacity);

            let bytes = make_vec();
            let address = bytes.as_ptr();
            let capacity = bytes.capacity();
            assert_into_bytes_reuses_allocation(
                Cow::<'_, [u8]>::Owned(bytes),
                input,
                address,
                capacity,
            );

            let bytes = make_vec();
            let address = bytes.as_ptr();
            let capacity = bytes.capacity();
            assert_into_bytes_reuses_allocation(
                Cow::<'_, Vec<u8>>::Owned(bytes),
                input,
                address,
                capacity,
            );

            #[cfg(feature = "bytes")]
            {
                let bytes = make_vec();
                let address = bytes.as_ptr();
                let capacity = bytes.capacity();
                assert_into_bytes_reuses_allocation(Bytes::from(bytes), input, address, capacity);

                let bytes = Box::<[u8]>::from(input);
                let address = bytes.as_ptr();
                assert_into_bytes_reuses_allocation(
                    Bytes::from(bytes),
                    input,
                    address,
                    input.len(),
                );

                let mut bytes = BytesMut::with_capacity(input.len() + 32);
                bytes.extend_from_slice(input);
                let address = bytes.as_ptr();
                let capacity = bytes.capacity();
                assert_into_bytes_reuses_allocation(bytes, input, address, capacity);

                let mut bytes = BytesMut::with_capacity(input.len() + 32);
                bytes.extend_from_slice(input);
                let address = bytes.as_ptr();
                let capacity = bytes.capacity();
                assert_into_bytes_reuses_allocation(bytes.freeze(), input, address, capacity);
            }
        }
    }

    #[test]
    fn into_bytes_copies_arrays_and_borrowed_cow() {
        assert_eq!(StringBytes::into_bytes([0_u8; 0]), b"");
        assert_eq!(StringBytes::into_bytes(*b"text"), b"text");
        assert_eq!(StringBytes::into_bytes([b'a', 0xff]), b"a\xff");
        check_cow_array_conversions(true);

        for input in [b"".as_slice(), b"text", b"ok\xff"] {
            let bytes = StringBytes::into_bytes(Cow::Borrowed(input));
            assert_eq!(bytes, input);
            if !input.is_empty() {
                assert_ne!(bytes.as_ptr(), input.as_ptr());
            }
            let vector = Vec::from(input);
            check_copied_conversion(Cow::Borrowed(&vector), input, true);
            assert_eq!(vector, input);
        }
    }

    #[cfg(feature = "bytes")]
    #[test]
    fn into_bytes_reuses_sliced_allocations() {
        for input in [b"text".as_slice(), b"ok\xff"] {
            // A slice's address differs from the allocation's original address.
            let mut original = Vec::with_capacity(64);
            original.extend_from_slice(b"prefix");
            original.extend_from_slice(input);
            let address = original.as_ptr();
            let capacity = original.capacity();
            let original = Bytes::from(original);
            let slice = original.slice(6..);
            assert_ne!(slice.as_ptr(), address);
            drop(original);
            assert_into_bytes_reuses_allocation(slice, input, address, capacity);

            let mut original = BytesMut::with_capacity(64);
            original.extend_from_slice(b"prefix");
            original.extend_from_slice(input);
            let address = original.as_ptr();
            let capacity = original.capacity();
            let prefix = original.split_to(6);
            assert_ne!(original.as_ptr(), address);
            drop(prefix);
            assert_into_bytes_reuses_allocation(original, input, address, capacity);
        }
    }

    #[cfg(feature = "bytes")]
    #[test]
    fn into_bytes_copies_nontransferable_buffers() {
        check_nontransferable_buffers(true);
    }

    #[test]
    fn try_into_string_reuses_owned_storage() {
        for input in [
            b"".as_slice(),
            b"text",
            "é说明".as_bytes(),
            b"\0e\xcc\x81",
            b"ok\xff",
        ] {
            let make_vec = || {
                let mut bytes = Vec::with_capacity(input.len() + 32);
                bytes.extend_from_slice(input);
                bytes
            };

            let bytes = make_vec();
            let address = bytes.as_ptr();
            let capacity = bytes.capacity();
            assert_try_into_string_reuses_allocation(bytes, input, address, capacity);

            let bytes = make_vec();
            let address = bytes.as_ptr();
            let capacity = bytes.capacity();
            assert_try_into_string_reuses_allocation(
                Cow::<'_, [u8]>::Owned(bytes),
                input,
                address,
                capacity,
            );

            let bytes = make_vec();
            let address = bytes.as_ptr();
            let capacity = bytes.capacity();
            assert_try_into_string_reuses_allocation(
                Cow::<'_, Vec<u8>>::Owned(bytes),
                input,
                address,
                capacity,
            );

            #[cfg(feature = "bytes")]
            {
                let bytes = make_vec();
                let address = bytes.as_ptr();
                let capacity = bytes.capacity();
                assert_try_into_string_reuses_allocation(
                    Bytes::from(bytes),
                    input,
                    address,
                    capacity,
                );

                let bytes = Box::<[u8]>::from(input);
                let address = bytes.as_ptr();
                assert_try_into_string_reuses_allocation(
                    Bytes::from(bytes),
                    input,
                    address,
                    input.len(),
                );

                let mut bytes = BytesMut::with_capacity(input.len() + 32);
                bytes.extend_from_slice(input);
                let address = bytes.as_ptr();
                let capacity = bytes.capacity();
                assert_try_into_string_reuses_allocation(bytes, input, address, capacity);

                let mut bytes = BytesMut::with_capacity(input.len() + 32);
                bytes.extend_from_slice(input);
                let address = bytes.as_ptr();
                let capacity = bytes.capacity();
                assert_try_into_string_reuses_allocation(bytes.freeze(), input, address, capacity);
            }
        }
    }

    #[test]
    fn try_into_string_error_preserves_owned_bytes() {
        for input in [
            b"\xff".as_slice(),
            b"ok\xe2\x82",
            b"\xc0\x80",
            b"\xed\xa0\x80",
        ] {
            let mut vector = Vec::with_capacity(input.len() + 16);
            vector.extend_from_slice(input);
            let pointer = vector.as_ptr();
            let capacity = vector.capacity();
            let error: std::string::FromUtf8Error = vector.try_into_string().unwrap_err();
            assert_eq!(error.as_bytes(), input);
            let vector = error.into_bytes();
            assert_eq!(vector, input);
            assert_eq!(vector.as_ptr(), pointer);
            assert_eq!(vector.capacity(), capacity);

            let boxed = Box::<[u8]>::from(input);
            let pointer = boxed.as_ptr();
            let bytes = boxed.try_into_string().unwrap_err().into_bytes();
            // Recovering a Vec from the error should keep the Box's allocation.
            assert_eq!(bytes, input);
            assert_eq!(bytes.as_ptr(), pointer);

            let cow = Cow::<'_, [u8]>::Owned(vector);
            let pointer = cow.as_ptr();
            let vector = cow.try_into_string().unwrap_err().into_bytes();
            assert_eq!(vector, input);
            assert_eq!(vector.as_ptr(), pointer);
            assert_eq!(vector.capacity(), capacity);
        }

        // An erased boxed array should still reuse its allocation on failure.
        let boxed: Box<dyn StringBytes> = Box::new([b'a', 0xff]);
        let pointer = boxed.as_bytes().as_ptr();
        let error = boxed.try_into_string().unwrap_err();
        assert_eq!(error.utf8_error().valid_up_to(), 1);
        let bytes = error.into_bytes();
        assert_eq!(bytes, b"a\xff");
        assert_eq!(bytes.as_ptr(), pointer);
        assert_eq!(bytes.capacity(), 2);

        // Borrowed input can also be recovered from the error.
        let error = b"ok\xff".as_slice().try_into_string().unwrap_err();
        assert_eq!(error.utf8_error().valid_up_to(), 2);
        assert_eq!(error.into_bytes(), b"ok\xff");
    }

    #[cfg(feature = "bytes")]
    #[test]
    fn try_into_string_reuses_sliced_allocations() {
        for input in [b"text".as_slice(), b"ok\xff"] {
            // A slice's address differs from the allocation's original address.
            let mut original = Vec::with_capacity(64);
            original.extend_from_slice(b"prefix");
            original.extend_from_slice(input);
            let address = original.as_ptr();
            let capacity = original.capacity();
            let original = Bytes::from(original);
            let slice = original.slice(6..);
            assert_ne!(slice.as_ptr(), address);
            drop(original);
            assert_try_into_string_reuses_allocation(slice, input, address, capacity);

            let mut original = BytesMut::with_capacity(64);
            original.extend_from_slice(b"prefix");
            original.extend_from_slice(input);
            let address = original.as_ptr();
            let capacity = original.capacity();
            let prefix = original.split_to(6);
            assert_ne!(original.as_ptr(), address);
            drop(prefix);
            assert_try_into_string_reuses_allocation(original, input, address, capacity);
        }
    }

    #[cfg(feature = "bytes")]
    #[test]
    fn try_into_string_copies_nontransferable_buffers() {
        check_nontransferable_buffers(false);
    }

    #[test]
    fn try_into_string_copies_borrowed_cow() {
        check_cow_array_conversions(false);
        for input in [b"".as_slice(), b"text", b"ok\xff"] {
            check_copied_conversion(Cow::Borrowed(input), input, false);
            let vector = Vec::from(input);
            check_copied_conversion(Cow::Borrowed(&vector), input, false);
            assert_eq!(vector, input);
        }
    }

    #[test]
    fn box_into_bytes_reuses_slice_and_array_allocations() {
        fn check<T: StringBytes + ?Sized>(storage: Box<T>, expected: &[u8]) {
            let address = storage.as_bytes().as_ptr();
            let bytes = T::box_into_bytes(storage);
            assert_eq!(bytes, expected);
            assert_eq!(bytes.as_ptr(), address);
            assert_eq!(bytes.capacity(), expected.len());
        }
        for input in [b"".as_slice(), b"text", b"ok\xff"] {
            check(Box::<[u8]>::from(input), input);
        }
        check(Box::new([0_u8; 0]), b"");
        check(Box::new(*b"text"), b"text");
        check(Box::new([b'a', 0xff]), b"a\xff");
    }

    #[test]
    fn box_into_bytes_copies_cow_arrays() {
        check_boxed_cow_array_conversions(true);
    }

    #[test]
    fn box_into_string_reuses_slice_and_array_allocations() {
        fn check<T: StringBytes + ?Sized>(storage: Box<T>, expected: &[u8]) {
            let address = storage.as_bytes().as_ptr();
            let bytes = match T::box_into_string(storage) {
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
            assert_eq!(bytes.as_ptr(), address);
            assert_eq!(bytes.capacity(), expected.len());
        }
        for input in [b"".as_slice(), b"text", b"ok\xff"] {
            check(Box::<[u8]>::from(input), input);
        }
        check(Box::new([0_u8; 0]), b"");
        check(Box::new(*b"text"), b"text");
        check(Box::new([b'a', 0xff]), b"a\xff");
    }

    #[test]
    fn box_into_string_copies_cow_arrays() {
        check_boxed_cow_array_conversions(false);
    }
}
