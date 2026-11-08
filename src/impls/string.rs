// These storage types guarantee UTF-8 and override string conversions to
// skip validation. Their pointer helpers delegate to these overrides,
// preserving the same behavior through smart pointers.

use std::borrow::Cow;
use std::rc::Rc;
use std::str::Utf8Error;
use std::string::FromUtf8Error;
use std::sync::Arc;

use super::{impl_pointer_conversions_for_sized, impl_string_bytes};
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

// String also guarantees UTF-8, so its string conversions skip the default
// checks. Consuming it returns the same value, retaining its buffer and capacity.
impl_string_bytes!(String {
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        String::as_bytes(self)
    }

    #[inline]
    fn into_bytes(self) -> Vec<u8> {
        String::into_bytes(self)
    }

    #[inline]
    fn try_borrow_str(&self) -> Result<&str, Utf8Error> {
        Ok(self.as_str())
    }

    #[inline]
    fn try_to_string(&self) -> Result<String, FromUtf8Error> {
        Ok(self.clone())
    }

    #[inline]
    fn try_into_string(self) -> Result<String, FromUtf8Error> {
        Ok(self)
    }

    impl_pointer_conversions_for_sized!();
});

// Owned Cow<str> values can pass on their String; borrowed values are copied
// when an owned result is needed. Both variants already contain valid UTF-8.
impl_string_bytes!(Cow<'_, str> {
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        self.as_ref().as_bytes()
    }

    #[inline]
    fn into_bytes(self) -> Vec<u8> {
        String::into_bytes(self.into_owned())
    }

    #[inline]
    fn try_borrow_str(&self) -> Result<&str, Utf8Error> {
        Ok(self.as_ref())
    }

    #[inline]
    fn try_to_string(&self) -> Result<String, FromUtf8Error> {
        // Borrowing first ensures that even an owned String is copied.
        Ok(String::from(self.as_ref()))
    }

    #[inline]
    fn try_into_string(self) -> Result<String, FromUtf8Error> {
        Ok(self.into_owned())
    }

    impl_pointer_conversions_for_sized!();
});

impl_string_bytes!(Cow<'_, String> {
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        self.as_ref().as_bytes()
    }

    #[inline]
    fn into_bytes(self) -> Vec<u8> {
        String::into_bytes(self.into_owned())
    }

    #[inline]
    fn try_borrow_str(&self) -> Result<&str, Utf8Error> {
        Ok(self.as_ref().as_str())
    }

    #[inline]
    fn try_to_string(&self) -> Result<String, FromUtf8Error> {
        Ok(self.as_ref().clone())
    }

    #[inline]
    fn try_into_string(self) -> Result<String, FromUtf8Error> {
        Ok(self.into_owned())
    }

    impl_pointer_conversions_for_sized!();
});

#[cfg(test)]
mod tests {
    use std::borrow::Cow;
    use std::ops::Deref;
    use std::rc::Rc;
    use std::sync::Arc;

    use crate::StringBytes;

    const INPUTS: [&str; 4] = ["", "text", "é说明", "\0e\u{301}"];

    fn owned(input: &str) -> String {
        let mut text = String::with_capacity(input.len() + 32);
        text.push_str(input);
        text
    }

    #[test]
    fn as_bytes_borrows_the_original_buffer() {
        fn check<T: StringBytes + ?Sized>(storage: &T, expected: &[u8]) {
            for _ in 0..3 {
                assert!(std::ptr::eq(StringBytes::as_bytes(storage), expected));
            }
        }

        for input in INPUTS {
            check(input, input.as_bytes());
            let text = owned(input);
            check(&text, text.as_bytes());
            let cow = Cow::Borrowed(input);
            check(&cow, input.as_bytes());
            let cow = Cow::<'_, str>::Owned(text);
            check(&cow, cow.as_ref().as_bytes());
            let text = owned(input);
            check(&Cow::Borrowed(&text), text.as_bytes());
            let cow = Cow::<'_, String>::Owned(text);
            check(&cow, cow.as_ref().as_bytes());
        }
    }

    #[test]
    fn to_bytes_copies_and_preserves_the_source() {
        fn check<T: StringBytes + ?Sized>(storage: &T, expected: &str) {
            let address = storage.as_bytes().as_ptr();
            let mut bytes = StringBytes::to_bytes(storage);
            assert_eq!(bytes, expected.as_bytes());
            if !bytes.is_empty() {
                assert_ne!(bytes.as_ptr(), address);
                bytes[0] ^= 0xff;
            }
            assert_eq!(storage.as_bytes(), expected.as_bytes());
            assert_eq!(storage.as_bytes().as_ptr(), address);
        }

        for input in INPUTS {
            check(input, input);
            check(&owned(input), input);
            check(&Cow::Borrowed(input), input);
            check(&Cow::<'_, str>::Owned(owned(input)), input);
            check(&Cow::Borrowed(&owned(input)), input);
            check(&Cow::<'_, String>::Owned(owned(input)), input);
        }
    }

    #[test]
    fn into_bytes_reuses_owned_storage() {
        fn check<T: StringBytes>(storage: T, expected: &str, capacity: usize) {
            let address = storage.as_bytes().as_ptr();
            // Use the trait method even for String, which has an inherent method.
            let bytes = StringBytes::into_bytes(storage);
            assert_eq!(bytes, expected.as_bytes());
            assert_eq!(bytes.as_ptr(), address);
            assert_eq!(bytes.capacity(), capacity);
        }

        for input in INPUTS {
            let text = owned(input);
            let capacity = text.capacity();
            check(text, input, capacity);
            let text = owned(input);
            let capacity = text.capacity();
            check(Cow::<'_, str>::Owned(text), input, capacity);
            let text = owned(input);
            let capacity = text.capacity();
            check(Cow::<'_, String>::Owned(text), input, capacity);
        }
    }

    #[test]
    fn into_bytes_copies_borrowed_cow() {
        fn check<T: StringBytes>(storage: T, input: &str) {
            let bytes = StringBytes::into_bytes(storage);
            assert_eq!(bytes, input.as_bytes());
            if !input.is_empty() {
                assert_ne!(bytes.as_ptr(), input.as_ptr());
            }
        }

        for input in INPUTS {
            check(Cow::Borrowed(input), input);
            let text = owned(input);
            check(Cow::Borrowed(&text), &text);
            assert_eq!(text, input);
        }
    }

    #[test]
    fn try_borrow_str_borrows_the_original_buffer() {
        fn check<T: StringBytes + ?Sized>(storage: &T, expected: &str) {
            let text = StringBytes::try_borrow_str(storage).unwrap();
            assert_eq!(text, expected);
            assert!(std::ptr::eq(text.as_bytes(), storage.as_bytes()));
        }

        for input in INPUTS {
            check(input, input);
            check(&owned(input), input);
            check(&Cow::Borrowed(input), input);
            check(&Cow::<'_, str>::Owned(owned(input)), input);
            check(&Cow::Borrowed(&owned(input)), input);
            check(&Cow::<'_, String>::Owned(owned(input)), input);
        }
    }

    #[test]
    fn try_to_string_copies_and_preserves_the_source() {
        fn check<T: StringBytes + ?Sized>(storage: &T, expected: &str) {
            let address = storage.as_bytes().as_ptr();
            let mut text = StringBytes::try_to_string(storage).unwrap();
            assert_eq!(text, expected);
            if !text.is_empty() {
                assert_ne!(text.as_ptr(), address);
            }
            text.clear();
            text.push_str("changed");
            assert_eq!(storage.as_bytes(), expected.as_bytes());
            assert_eq!(storage.as_bytes().as_ptr(), address);
        }

        for input in INPUTS {
            check(input, input);
            check(&owned(input), input);
            check(&Cow::Borrowed(input), input);
            check(&Cow::<'_, str>::Owned(owned(input)), input);
            check(&Cow::Borrowed(&owned(input)), input);
            check(&Cow::<'_, String>::Owned(owned(input)), input);
        }
    }

    #[test]
    fn try_into_string_reuses_owned_storage() {
        fn check<T: StringBytes>(storage: T, expected: &str, capacity: usize) {
            let address = storage.as_bytes().as_ptr();
            let text = StringBytes::try_into_string(storage).unwrap();
            assert_eq!(text, expected);
            assert_eq!(text.as_ptr(), address);
            assert_eq!(text.capacity(), capacity);
        }

        for input in INPUTS {
            let text = owned(input);
            let capacity = text.capacity();
            check(text, input, capacity);
            let text = owned(input);
            let capacity = text.capacity();
            check(Cow::<'_, str>::Owned(text), input, capacity);
            let text = owned(input);
            let capacity = text.capacity();
            check(Cow::<'_, String>::Owned(text), input, capacity);
        }
    }

    #[test]
    fn try_into_string_copies_borrowed_cow() {
        fn check<T: StringBytes>(storage: T, input: &str) {
            let text = StringBytes::try_into_string(storage).unwrap();
            assert_eq!(text, input);
            if !input.is_empty() {
                assert_ne!(text.as_ptr(), input.as_ptr());
            }
        }

        for input in INPUTS {
            check(Cow::Borrowed(input), input);
            let text = owned(input);
            check(Cow::Borrowed(&text), &text);
            assert_eq!(text, input);
        }
    }

    #[test]
    fn box_into_bytes_reuses_the_str_allocation() {
        for input in INPUTS {
            let boxed = Box::<str>::from(input);
            let address = boxed.as_ptr();
            let bytes = <str as StringBytes>::box_into_bytes(boxed);
            assert_eq!(bytes, input.as_bytes());
            assert_eq!(bytes.as_ptr(), address);
            assert_eq!(bytes.capacity(), input.len());
        }
    }

    #[test]
    fn box_into_string_reuses_the_str_allocation() {
        for input in INPUTS {
            let boxed = Box::<str>::from(input);
            let address = boxed.as_ptr();
            let text = <str as StringBytes>::box_into_string(boxed).unwrap();
            assert_eq!(text, input);
            assert_eq!(text.as_ptr(), address);
            assert_eq!(text.capacity(), input.len());
        }
    }

    #[test]
    fn rc_into_string_copies_str_with_one_or_more_owners() {
        for input in INPUTS {
            for shared in [false, true] {
                let source = Rc::<str>::from(input);
                let address = source.as_ptr();
                let other = shared.then(|| Rc::clone(&source));
                let text = <str as StringBytes>::rc_into_string(source).unwrap();
                assert_eq!(text, input);
                if !input.is_empty() {
                    assert_ne!(text.as_ptr(), address);
                }
                if let Some(other) = other {
                    assert_eq!(&*other, input);
                }
            }
        }
    }

    #[test]
    fn arc_into_string_copies_str_with_one_or_more_owners() {
        for input in INPUTS {
            for shared in [false, true] {
                let source = Arc::<str>::from(input);
                let address = source.as_ptr();
                let other = shared.then(|| Arc::clone(&source));
                let text = <str as StringBytes>::arc_into_string(source).unwrap();
                assert_eq!(text, input);
                if !input.is_empty() {
                    assert_ne!(text.as_ptr(), address);
                }
                if let Some(other) = other {
                    assert_eq!(&*other, input);
                }
            }
        }
    }

    #[test]
    fn as_ref_borrows_the_same_slice_as_as_bytes() {
        fn check<T: StringBytes + AsRef<[u8]> + ?Sized>(storage: &T) {
            assert!(std::ptr::eq(
                StringBytes::as_bytes(storage),
                storage.as_ref()
            ));
        }

        for input in INPUTS {
            check(input);
            check(&owned(input));
        }
    }

    #[test]
    fn deref_target_implements_string_bytes_and_borrows_the_same_slice() {
        fn check<T>(storage: &T, expected: &str)
        where
            T: StringBytes + Deref + ?Sized,
            T::Target: StringBytes,
        {
            for _ in 0..3 {
                let bytes = StringBytes::as_bytes(storage);
                let target_bytes = StringBytes::as_bytes(T::deref(storage));
                assert!(std::ptr::eq(bytes, target_bytes));
                assert_eq!(target_bytes, expected.as_bytes());
            }
        }

        for input in INPUTS {
            check(&owned(input), input);
            check(&Cow::Borrowed(input), input);
            check(&Cow::<'_, str>::Owned(owned(input)), input);
            check(&Cow::Borrowed(&owned(input)), input);
            check(&Cow::<'_, String>::Owned(owned(input)), input);
        }
    }
}
