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

// Restrict Cow to byte slices and str so both variants satisfy the trait's
// guarantees.
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

    use super::super::test_support::assert_reuses_allocation;
    use crate::StringBytes;

    fn check_owned_conversions(into_bytes: bool) {
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
            assert_reuses_allocation(bytes, input, address, capacity, into_bytes);

            let bytes = make_vec();
            let address = bytes.as_ptr();
            let capacity = bytes.capacity();
            assert_reuses_allocation(
                Cow::<'_, [u8]>::Owned(bytes),
                input,
                address,
                capacity,
                into_bytes,
            );

            #[cfg(feature = "bytes")]
            {
                let bytes = make_vec();
                let address = bytes.as_ptr();
                let capacity = bytes.capacity();
                assert_reuses_allocation(Bytes::from(bytes), input, address, capacity, into_bytes);

                let bytes = Box::<[u8]>::from(input);
                let address = bytes.as_ptr();
                assert_reuses_allocation(
                    Bytes::from(bytes),
                    input,
                    address,
                    input.len(),
                    into_bytes,
                );

                let mut bytes = BytesMut::with_capacity(input.len() + 32);
                bytes.extend_from_slice(input);
                let address = bytes.as_ptr();
                let capacity = bytes.capacity();
                assert_reuses_allocation(bytes, input, address, capacity, into_bytes);

                let mut bytes = BytesMut::with_capacity(input.len() + 32);
                bytes.extend_from_slice(input);
                let address = bytes.as_ptr();
                let capacity = bytes.capacity();
                assert_reuses_allocation(bytes.freeze(), input, address, capacity, into_bytes);
            }
        }
    }

    #[cfg(feature = "bytes")]
    fn check_sliced_allocation_reuse(into_bytes: bool) {
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
            assert_reuses_allocation(slice, input, address, capacity, into_bytes);

            let mut original = BytesMut::with_capacity(64);
            original.extend_from_slice(b"prefix");
            original.extend_from_slice(input);
            let address = original.as_ptr();
            let capacity = original.capacity();
            let prefix = original.split_to(6);
            assert_ne!(original.as_ptr(), address);
            drop(prefix);
            assert_reuses_allocation(original, input, address, capacity, into_bytes);
        }
    }

    fn check_copied_conversion<T: StringBytes>(storage: T, expected: &[u8], into_bytes: bool) {
        let address = storage.as_bytes().as_ptr();
        let bytes = if into_bytes {
            storage.into_bytes()
        } else {
            match storage.try_into_string() {
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

    #[test]
    fn as_bytes_borrows_the_as_ref_slice() {
        fn check<T: StringBytes + AsRef<[u8]> + ?Sized>(storage: &T) {
            for _ in 0..3 {
                assert!(std::ptr::eq(
                    StringBytes::as_bytes(storage),
                    storage.as_ref()
                ));
            }
        }

        for input in [b"".as_slice(), b"text", b"\0e\xcc\x81", b"ok\xff"] {
            check(input);
            check(&Vec::from(input));
            check(&Cow::Borrowed(input));
            check(&Cow::<'_, [u8]>::Owned(Vec::from(input)));
            #[cfg(feature = "bytes")]
            {
                check(&Bytes::from_static(input));
                check(&BytesMut::from(input));
            }
        }
        check(&[0_u8; 0]);
        check(&[0_u8; 4]);
        check(&[0xff_u8; 64]);
    }

    #[test]
    fn to_bytes_copies_and_preserves_the_source() {
        fn check<T: StringBytes + ?Sized>(storage: &T) {
            let expected = storage.as_bytes().to_vec();
            let address = storage.as_bytes().as_ptr();
            let mut bytes = StringBytes::to_bytes(storage);
            assert_eq!(bytes, expected);
            if !bytes.is_empty() {
                assert_ne!(bytes.as_ptr(), address);
                bytes[0] ^= 0xff;
            }
            assert_eq!(storage.as_bytes(), expected);
            assert_eq!(storage.as_bytes().as_ptr(), address);
        }

        for input in [b"".as_slice(), b"text", b"ok\xff"] {
            check(input);
            check(&Vec::from(input));
            check(&Cow::Borrowed(input));
            check(&Cow::<'_, [u8]>::Owned(Vec::from(input)));
            #[cfg(feature = "bytes")]
            {
                check(&Bytes::from(Vec::from(input)));
                check(&BytesMut::from(input));
            }
        }
        check(&[0_u8; 0]);
        check(&[b'a', 0xff]);
    }

    #[test]
    fn into_bytes_reuses_owned_storage() {
        check_owned_conversions(true);
    }

    #[test]
    fn into_bytes_copies_arrays_and_borrowed_cow() {
        assert_eq!(StringBytes::into_bytes([0_u8; 0]), b"");
        assert_eq!(StringBytes::into_bytes(*b"text"), b"text");
        assert_eq!(StringBytes::into_bytes([b'a', 0xff]), b"a\xff");

        for input in [b"".as_slice(), b"text", b"ok\xff"] {
            let bytes = StringBytes::into_bytes(Cow::Borrowed(input));
            assert_eq!(bytes, input);
            if !input.is_empty() {
                assert_ne!(bytes.as_ptr(), input.as_ptr());
            }
        }
    }

    #[cfg(feature = "bytes")]
    #[test]
    fn into_bytes_reuses_sliced_allocations() {
        check_sliced_allocation_reuse(true);
    }

    #[cfg(feature = "bytes")]
    #[test]
    fn into_bytes_copies_nontransferable_buffers() {
        check_nontransferable_buffers(true);
    }

    #[test]
    fn try_borrow_str_borrows_the_same_bytes() {
        fn check<T: StringBytes + ?Sized>(storage: &T, expected: &str) {
            let text = storage.try_borrow_str().unwrap();
            assert_eq!(text, expected);
            assert!(std::ptr::eq(text.as_bytes(), storage.as_bytes()));
        }

        for input in ["", "text", "é说明", "\0e\u{301}"] {
            check(input.as_bytes(), input);
            check(&Vec::from(input.as_bytes()), input);
            check(&Box::<[u8]>::from(input.as_bytes()), input);
            check(&Cow::Borrowed(input.as_bytes()), input);
            check(&Cow::<'_, [u8]>::Owned(Vec::from(input.as_bytes())), input);

            #[cfg(feature = "bytes")]
            {
                check(&Bytes::from_static(input.as_bytes()), input);
                check(&BytesMut::from(input.as_bytes()), input);
            }
        }
        check(&[0_u8; 0], "");
        check(b"text", "text");
    }

    #[test]
    fn try_borrow_str_reports_invalid_utf8() {
        fn check<T: StringBytes + ?Sized>(
            storage: &T,
            valid_up_to: usize,
            error_len: Option<usize>,
        ) {
            let error = StringBytes::try_borrow_str(storage).unwrap_err();
            assert_eq!(error.valid_up_to(), valid_up_to);
            assert_eq!(error.error_len(), error_len);
        }

        // None denotes an incomplete sequence; all other errors have a length.
        for (input, valid_up_to, error_len) in [
            (b"\xff".as_slice(), 0, Some(1)),
            (b"ok\xe2\x82", 2, None),
            (b"\xc0\x80", 0, Some(1)),
            (b"\xed\xa0\x80", 0, Some(1)),
        ] {
            check(input, valid_up_to, error_len);
            check(&Vec::from(input), valid_up_to, error_len);
            check(&Cow::Borrowed(input), valid_up_to, error_len);
            check(
                &Cow::<'_, [u8]>::Owned(Vec::from(input)),
                valid_up_to,
                error_len,
            );
            #[cfg(feature = "bytes")]
            {
                check(&Bytes::from_static(input), valid_up_to, error_len);
                check(&BytesMut::from(input), valid_up_to, error_len);
            }
        }
        check(&[0xff], 0, Some(1));
        check(&[b'o', b'k', 0xe2, 0x82], 2, None);
    }

    #[test]
    fn try_to_string_preserves_bytes_and_source() {
        fn check<T: StringBytes>(storage: T) {
            let expected = storage.as_bytes().to_vec();
            let copied = storage.try_to_string().unwrap();
            assert_eq!(copied.as_bytes(), expected);
            assert_eq!(storage.as_bytes(), expected);

            if !expected.is_empty() {
                assert_ne!(copied.as_ptr(), storage.as_bytes().as_ptr());
            }
        }

        // Include embedded NUL and decomposed Unicode to catch any normalization.
        for input in ["", "text", "é说明", "\0e\u{301}"] {
            check(input.as_bytes());
            check(Vec::from(input.as_bytes()));
            check(Box::<[u8]>::from(input.as_bytes()));
            check(Cow::Borrowed(input.as_bytes()));
            check(Cow::<'_, [u8]>::Owned(Vec::from(input.as_bytes())));

            #[cfg(feature = "bytes")]
            {
                check(Bytes::from_static(input.as_bytes()));
                check(BytesMut::from(input.as_bytes()));
            }
        }

        check([0_u8; 0]);
        check(*b"text");
        check(Box::new(*b"text"));
    }

    #[test]
    fn try_to_string_rejects_invalid_utf8() {
        fn check<T: StringBytes>(storage: T) {
            let expected = storage.as_bytes().to_vec();
            let error = storage.try_to_string().unwrap_err();
            assert_eq!(error.as_bytes(), expected);
            assert_eq!(storage.as_bytes(), expected);
            assert_ne!(error.as_bytes().as_ptr(), storage.as_bytes().as_ptr());
        }

        // Reject invalid, truncated, and overlong UTF-8 without lossy replacement.
        for input in [b"\xff".as_slice(), b"\xe2\x82", b"\xc0\x80"] {
            check(input);
            check(Vec::from(input));
            check(Box::<[u8]>::from(input));
            check(Cow::Borrowed(input));
            check(Cow::<'_, [u8]>::Owned(Vec::from(input)));

            #[cfg(feature = "bytes")]
            {
                check(Bytes::from_static(input));
                check(BytesMut::from(input));
            }
        }

        check([0xff]);
        check(Box::new([0xff]));
    }

    #[test]
    fn try_into_string_reuses_owned_storage() {
        check_owned_conversions(false);
    }

    #[test]
    fn try_into_string_preserves_the_storage_bytes() {
        fn check<T: StringBytes>(storage: T) {
            // Check contents here; allocation reuse is tested separately.
            let expected = storage.as_bytes().to_vec();
            let converted = storage.try_into_string().unwrap();
            assert_eq!(converted.as_bytes(), expected);
        }

        // Include embedded NUL and decomposed Unicode to catch any normalization.
        for input in ["", "text", "é说明", "\0e\u{301}"] {
            check(input.as_bytes());
            check(Vec::from(input.as_bytes()));
            check(Box::<[u8]>::from(input.as_bytes()));
            check(Cow::Borrowed(input.as_bytes()));
            check(Cow::<'_, [u8]>::Owned(Vec::from(input.as_bytes())));

            #[cfg(feature = "bytes")]
            {
                check(Bytes::from_static(input.as_bytes()));
                check(BytesMut::from(input.as_bytes()));
            }
        }

        check([0_u8; 0]);
        check(*b"text");
        check(Box::new(*b"text"));
    }

    #[test]
    fn try_into_string_rejects_invalid_utf8() {
        fn check<T: StringBytes>(storage: T) {
            let expected = storage.as_bytes().to_vec();
            let error = storage.try_into_string().unwrap_err();
            assert_eq!(error.as_bytes(), expected);
        }

        // Reject invalid, truncated, and overlong UTF-8 without lossy replacement.
        for input in [b"\xff".as_slice(), b"\xe2\x82", b"\xc0\x80"] {
            check(input);
            check(Vec::from(input));
            check(Box::<[u8]>::from(input));
            check(Cow::Borrowed(input));
            check(Cow::<'_, [u8]>::Owned(Vec::from(input)));

            #[cfg(feature = "bytes")]
            {
                check(Bytes::from_static(input));
                check(BytesMut::from(input));
            }
        }

        check([0xff]);
        check(Box::new([0xff]));
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
        check_sliced_allocation_reuse(false);
    }

    #[cfg(feature = "bytes")]
    #[test]
    fn try_into_string_copies_nontransferable_buffers() {
        check_nontransferable_buffers(false);
    }

    #[test]
    fn try_into_string_copies_borrowed_cow() {
        for input in [b"".as_slice(), b"text", b"ok\xff"] {
            check_copied_conversion(Cow::Borrowed(input), input, false);
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
    fn as_mut_borrows_the_same_slice_as_as_bytes() {
        fn check_mut<T: StringBytes + AsMut<[u8]> + ?Sized>(value: &mut T) {
            let original = value.as_bytes();
            let pointer = original.as_ptr();
            // Wrapping addition also handles 0xff.
            let expected: Vec<u8> = original.iter().map(|byte| byte.wrapping_add(1)).collect();
            let mutable = value.as_mut();
            assert_eq!(mutable.as_ptr(), pointer);
            assert_eq!(mutable.len(), expected.len());
            for byte in mutable {
                *byte = byte.wrapping_add(1);
            }
            // as_bytes must see the changes made through AsMut.
            assert_eq!(value.as_bytes(), expected);
        }

        for input in [b"".as_slice(), b"AC", b"A\xff"] {
            let mut vector = Vec::from(input);
            check_mut(vector.as_mut_slice()); // T = [u8].
            check_mut(&mut vector); // T = Vec<u8>.

            #[cfg(feature = "bytes")]
            check_mut(&mut BytesMut::from(input));
        }

        check_mut(&mut [0_u8; 0]);
        check_mut(&mut [0xff_u8; 64]);
    }

    #[test]
    fn from_preserves_the_source_bytes() {
        fn check<S: StringBytes, T: StringBytes + From<S>>(input: S) {
            // From may allocate, so compare contents rather than addresses.
            let expected = input.as_bytes().to_vec();
            let storage = T::from(input);
            assert_eq!(storage.as_bytes(), expected);
        }

        for input in [b"".as_slice(), b"AC", b"A\xff"] {
            check::<_, Vec<u8>>(input);
            check::<_, Cow<'_, [u8]>>(input);

            // Cover owned sources as well as borrowed slices.
            check::<_, Cow<'_, [u8]>>(Vec::from(input));
            check::<_, Vec<u8>>(Box::<[u8]>::from(input));

            #[cfg(feature = "bytes")]
            {
                // These literals provide the 'static lifetime Bytes requires.
                check::<_, Bytes>(input);
                check::<_, BytesMut>(input);
                check::<_, Bytes>(Vec::from(input));
                check::<_, Bytes>(Box::<[u8]>::from(input));
                check::<_, Vec<u8>>(Bytes::from_static(input));
                check::<_, Vec<u8>>(BytesMut::from(input));
                check::<_, BytesMut>(Bytes::from_static(input));
                check::<_, Bytes>(BytesMut::from(input));
            }
        }

        check::<_, Vec<u8>>([0_u8; 0]);
        check::<_, Vec<u8>>(*b"AC");
    }
}
