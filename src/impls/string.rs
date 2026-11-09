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
    use std::rc::Rc;
    use std::sync::Arc;

    use crate::test_utils::{
        assert_as_ref_matches_as_bytes, assert_deref_matches_as_bytes,
        assert_into_bytes_preserves_bytes, assert_into_bytes_reuses_allocation,
        assert_stable_bytes, assert_to_bytes_preserves_bytes,
        assert_try_borrow_str_preserves_bytes, assert_try_into_string_preserves_bytes,
        assert_try_into_string_reuses_allocation, assert_try_to_string_preserves_bytes,
    };
    use crate::StringBytes;

    const INPUTS: [&str; 4] = ["", "text", "é说明", "\0e\u{301}"];

    fn owned(input: &str) -> String {
        let mut text = String::with_capacity(input.len() + 32);
        text.push_str(input);
        text
    }

    #[test]
    fn str_guarantees() {
        for input in INPUTS {
            assert_eq!(StringBytes::as_bytes(input), input.as_bytes());
            assert_stable_bytes(input);
            assert_as_ref_matches_as_bytes(input);
            assert_to_bytes_preserves_bytes(input);
            let _ = assert_try_borrow_str_preserves_bytes(input);
            let _ = assert_try_to_string_preserves_bytes(input);
        }
    }

    #[test]
    fn string_guarantees() {
        for input in INPUTS {
            let storage = owned(input);
            assert_eq!(StringBytes::as_bytes(&storage), input.as_bytes());
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

    #[test]
    fn cow_str_guarantees() {
        for input in INPUTS {
            for storage in [Cow::Borrowed(input), Cow::<'_, str>::Owned(owned(input))] {
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
    }

    #[test]
    fn cow_string_guarantees() {
        for input in INPUTS {
            let text = owned(input);
            for storage in [Cow::Borrowed(&text), Cow::<'_, String>::Owned(owned(input))] {
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
    }

    #[test]
    fn into_bytes_reuses_owned_storage() {
        for input in INPUTS {
            let text = owned(input);
            let address = text.as_ptr();
            let capacity = text.capacity();
            assert_into_bytes_reuses_allocation(text, input.as_bytes(), address, capacity);

            let text = owned(input);
            let address = text.as_ptr();
            let capacity = text.capacity();
            assert_into_bytes_reuses_allocation(
                Cow::<'_, str>::Owned(text),
                input.as_bytes(),
                address,
                capacity,
            );

            let text = owned(input);
            let address = text.as_ptr();
            let capacity = text.capacity();
            assert_into_bytes_reuses_allocation(
                Cow::<'_, String>::Owned(text),
                input.as_bytes(),
                address,
                capacity,
            );
        }
    }

    #[test]
    fn into_bytes_copies_borrowed_cow() {
        fn check<T: StringBytes>(storage: T, input: &str) {
            let bytes = assert_into_bytes_preserves_bytes(storage);
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
    fn try_into_string_reuses_owned_storage() {
        for input in INPUTS {
            let text = owned(input);
            let address = text.as_ptr();
            let capacity = text.capacity();
            assert_try_into_string_reuses_allocation(text, input.as_bytes(), address, capacity);

            let text = owned(input);
            let address = text.as_ptr();
            let capacity = text.capacity();
            assert_try_into_string_reuses_allocation(
                Cow::<'_, str>::Owned(text),
                input.as_bytes(),
                address,
                capacity,
            );

            let text = owned(input);
            let address = text.as_ptr();
            let capacity = text.capacity();
            assert_try_into_string_reuses_allocation(
                Cow::<'_, String>::Owned(text),
                input.as_bytes(),
                address,
                capacity,
            );
        }
    }

    #[test]
    fn try_into_string_copies_borrowed_cow() {
        fn check<T: StringBytes>(storage: T, input: &str) {
            let text = assert_try_into_string_preserves_bytes(storage).unwrap();
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
}
