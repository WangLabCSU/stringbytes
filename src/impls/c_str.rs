use std::ffi::{CStr, CString};
use std::rc::Rc;
use std::string::FromUtf8Error;
use std::sync::Arc;

use super::{impl_pointer_conversions_for_sized, impl_string_bytes};
use crate::{sealed, StringBytes};

// Exclude the trailing NUL, matching CStr::to_bytes. C strings need not be
// UTF-8, so the default string conversions still validate their contents.
impl_string_bytes!(CStr {
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        CStr::to_bytes(self)
    }

    #[inline]
    fn box_into_bytes(self: Box<Self>) -> Vec<u8> {
        // Both conversions transfer the allocation. Removing the terminator
        // changes the length, leaving its byte available as spare capacity.
        self.into_c_string().into_bytes()
    }
});

impl_string_bytes!(CString {
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        CString::as_bytes(self)
    }

    #[inline]
    fn into_bytes(self) -> Vec<u8> {
        // Remove the terminator while retaining the owned buffer.
        CString::into_bytes(self)
    }

    impl_pointer_conversions_for_sized!();
});

#[cfg(test)]
mod tests {
    use std::ffi::{CStr, CString};
    use std::rc::Rc;
    use std::string::FromUtf8Error;
    use std::sync::Arc;

    use crate::test_utils::{
        assert_borrow_str_unchecked_preserves_bytes, assert_deref_matches_as_bytes,
        assert_into_bytes_preserves_bytes, assert_into_bytes_reuses_allocation,
        assert_into_string_lossy_matches_std, assert_into_string_lossy_reuses_valid_allocation,
        assert_into_string_unchecked_preserves_bytes,
        assert_into_string_unchecked_reuses_allocation, assert_stable_bytes,
        assert_to_bytes_preserves_bytes, assert_to_string_lossy_matches_std,
        assert_to_string_unchecked_preserves_bytes, assert_try_borrow_str_preserves_bytes,
        assert_try_into_string_preserves_bytes, assert_try_into_string_reuses_allocation,
        assert_try_to_string_preserves_bytes,
    };
    use crate::StringBytes;

    // Each input has exactly one NUL, including the empty C string. Invalid
    // UTF-8 includes incomplete, overlong, and surrogate encodings.
    const INPUTS: [&[u8]; 8] = [
        b"\0",
        b"text\0",
        "é说明\0".as_bytes(),
        b"e\xcc\x81\0",
        b"ok\xff\0",
        b"ok\xe2\x82\0",
        b"\xc0\x80\0",
        b"\xed\xa0\x80\0",
    ];

    #[test]
    fn c_str_guarantees() {
        for input in INPUTS {
            let storage = CStr::from_bytes_with_nul(input).unwrap();
            let expected = &input[..input.len() - 1];
            assert!(std::ptr::eq(StringBytes::as_bytes(storage), expected));
            assert_stable_bytes(storage);
            assert_to_bytes_preserves_bytes(storage);
            let _ = assert_try_borrow_str_preserves_bytes(storage);
            let _ = assert_try_to_string_preserves_bytes(storage);
            assert_eq!(storage.to_bytes_with_nul(), input);
        }
    }

    #[test]
    fn c_string_guarantees() {
        for input in INPUTS {
            let storage = CString::from_vec_with_nul(input.to_vec()).unwrap();
            assert_eq!(StringBytes::as_bytes(&storage), &input[..input.len() - 1]);
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
    fn shared_reference_guarantees() {
        for input in INPUTS {
            let storage: &CStr = CStr::from_bytes_with_nul(input).unwrap();
            let expected = &input[..input.len() - 1];
            assert_eq!(StringBytes::as_bytes(&storage), expected);
            assert_stable_bytes(&storage);
            assert_deref_matches_as_bytes(&storage);
            assert_to_bytes_preserves_bytes(&storage);
            let _ = assert_try_borrow_str_preserves_bytes(&storage);
            let _ = assert_try_to_string_preserves_bytes(&storage);
            // Consuming a reference copies the contents, without the NUL.
            let bytes = assert_into_bytes_preserves_bytes(storage);
            assert_eq!(bytes, expected);
            if !bytes.is_empty() {
                assert_ne!(bytes.as_ptr(), input.as_ptr());
            }
            let bytes = assert_try_into_string_preserves_bytes(storage)
                .map_or_else(FromUtf8Error::into_bytes, String::into_bytes);
            assert_eq!(bytes, expected);
            if !bytes.is_empty() {
                assert_ne!(bytes.as_ptr(), input.as_ptr());
            }
            assert_eq!(storage.to_bytes_with_nul(), input);
        }
    }

    #[test]
    fn mutable_reference_guarantees() {
        for input in INPUTS {
            let mut source = Box::<CStr>::from(CStr::from_bytes_with_nul(input).unwrap());
            let storage = source.as_mut();
            assert_eq!(StringBytes::as_bytes(&storage), &input[..input.len() - 1]);
            assert_stable_bytes(&storage);
            assert_deref_matches_as_bytes(&storage);
            assert_to_bytes_preserves_bytes(&storage);
            let _ = assert_try_borrow_str_preserves_bytes(&storage);
            let _ = assert_try_to_string_preserves_bytes(&storage);
            assert_into_bytes_preserves_bytes(&mut *storage);
            let _ = assert_try_into_string_preserves_bytes(&mut *storage);
            assert_eq!(source.to_bytes_with_nul(), input);
        }
    }

    #[test]
    fn box_guarantees() {
        for input in INPUTS {
            let storage = Box::<CStr>::from(CStr::from_bytes_with_nul(input).unwrap());
            assert_eq!(StringBytes::as_bytes(&storage), &input[..input.len() - 1]);
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
    fn rc_guarantees() {
        for input in INPUTS {
            let storage = Rc::<CStr>::from(CStr::from_bytes_with_nul(input).unwrap());
            assert_eq!(StringBytes::as_bytes(&storage), &input[..input.len() - 1]);
            assert_stable_bytes(&storage);
            assert_deref_matches_as_bytes(&storage);
            assert_to_bytes_preserves_bytes(&storage);
            let _ = assert_try_borrow_str_preserves_bytes(&storage);
            let _ = assert_try_to_string_preserves_bytes(&storage);
            assert_into_bytes_preserves_bytes(Rc::clone(&storage));
            let _ = assert_try_into_string_preserves_bytes(storage);
        }
    }

    #[test]
    fn arc_guarantees() {
        for input in INPUTS {
            let storage = Arc::<CStr>::from(CStr::from_bytes_with_nul(input).unwrap());
            assert_eq!(StringBytes::as_bytes(&storage), &input[..input.len() - 1]);
            assert_stable_bytes(&storage);
            assert_deref_matches_as_bytes(&storage);
            assert_to_bytes_preserves_bytes(&storage);
            let _ = assert_try_borrow_str_preserves_bytes(&storage);
            let _ = assert_try_to_string_preserves_bytes(&storage);
            assert_into_bytes_preserves_bytes(Arc::clone(&storage));
            let _ = assert_try_into_string_preserves_bytes(storage);
        }
    }

    #[test]
    fn c_str_unchecked_and_lossy_methods() {
        for input in INPUTS {
            let storage = CStr::from_bytes_with_nul(input).unwrap();
            assert_to_string_lossy_matches_std(storage);
            if std::str::from_utf8(StringBytes::as_bytes(storage)).is_ok() {
                assert_borrow_str_unchecked_preserves_bytes(storage);
                assert_to_string_unchecked_preserves_bytes(storage);
            }
        }
    }

    #[test]
    fn c_string_unchecked_and_lossy_methods() {
        for input in INPUTS {
            let storage = CString::from_vec_with_nul(input.to_vec()).unwrap();
            assert_to_string_lossy_matches_std(&storage);
            if std::str::from_utf8(StringBytes::as_bytes(&storage)).is_ok() {
                assert_borrow_str_unchecked_preserves_bytes(&storage);
                assert_to_string_unchecked_preserves_bytes(&storage);
                assert_into_string_unchecked_preserves_bytes(storage.clone());
            }
            assert_into_string_lossy_matches_std(storage);
        }
    }

    #[test]
    fn box_c_str_unchecked_and_lossy_methods() {
        for input in INPUTS {
            let source = CStr::from_bytes_with_nul(input).unwrap();
            let storage = Box::<CStr>::from(source);
            assert_to_string_lossy_matches_std(&storage);
            if std::str::from_utf8(StringBytes::as_bytes(&storage)).is_ok() {
                assert_borrow_str_unchecked_preserves_bytes(&storage);
                assert_to_string_unchecked_preserves_bytes(&storage);
                assert_into_string_unchecked_preserves_bytes(storage.clone());
            }
            assert_into_string_lossy_matches_std(storage);
        }
    }

    #[test]
    fn rc_c_str_unchecked_and_lossy_methods() {
        for input in INPUTS {
            let source = CStr::from_bytes_with_nul(input).unwrap();
            let storage = Rc::<CStr>::from(source);
            assert_to_string_lossy_matches_std(&storage);
            if std::str::from_utf8(StringBytes::as_bytes(&storage)).is_ok() {
                assert_borrow_str_unchecked_preserves_bytes(&storage);
                assert_to_string_unchecked_preserves_bytes(&storage);
                assert_into_string_unchecked_preserves_bytes(storage.clone());
            }
            assert_into_string_lossy_matches_std(storage);
        }
    }

    #[test]
    fn arc_c_str_unchecked_and_lossy_methods() {
        for input in INPUTS {
            let source = CStr::from_bytes_with_nul(input).unwrap();
            let storage = Arc::<CStr>::from(source);
            assert_to_string_lossy_matches_std(&storage);
            if std::str::from_utf8(StringBytes::as_bytes(&storage)).is_ok() {
                assert_borrow_str_unchecked_preserves_bytes(&storage);
                assert_to_string_unchecked_preserves_bytes(&storage);
                assert_into_string_unchecked_preserves_bytes(storage.clone());
            }
            assert_into_string_lossy_matches_std(storage);
        }
    }

    #[test]
    fn into_bytes_reuses_the_box_allocation() {
        for input in INPUTS {
            let source = CStr::from_bytes_with_nul(input).unwrap();
            let expected = &input[..input.len() - 1];
            let storage = Box::<CStr>::from(source);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            assert_into_bytes_reuses_allocation(storage, expected, address, input.len());

            // Erasing a sized wrapper must retain CStr's conversion behavior.
            let storage = Box::<CStr>::from(source);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let storage: Box<dyn StringBytes> = Box::new(storage);
            assert_into_bytes_reuses_allocation(storage, expected, address, input.len());
        }
    }

    #[test]
    fn into_bytes_copies_rc_and_arc_storage() {
        for input in INPUTS {
            let source = CStr::from_bytes_with_nul(input).unwrap();
            for shared in [false, true] {
                let storage = Rc::<CStr>::from(source);
                let address = StringBytes::as_bytes(&storage).as_ptr();
                let other = shared.then(|| Rc::clone(&storage));
                let bytes = assert_into_bytes_preserves_bytes(storage);
                if !bytes.is_empty() {
                    assert_ne!(bytes.as_ptr(), address);
                }
                if let Some(other) = other {
                    assert_eq!(other.to_bytes_with_nul(), input);
                }

                let storage = Arc::<CStr>::from(source);
                let address = StringBytes::as_bytes(&storage).as_ptr();
                let other = shared.then(|| Arc::clone(&storage));
                let bytes = assert_into_bytes_preserves_bytes(storage);
                if !bytes.is_empty() {
                    assert_ne!(bytes.as_ptr(), address);
                }
                if let Some(other) = other {
                    assert_eq!(other.to_bytes_with_nul(), input);
                }
            }
        }
    }

    #[test]
    fn into_bytes_reuses_c_string_allocations() {
        for input in INPUTS {
            let source = CStr::from_bytes_with_nul(input).unwrap();
            let expected = &input[..input.len() - 1];

            let storage = CString::from(source);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            assert_into_bytes_reuses_allocation(storage, expected, address, input.len());

            let storage = Box::new(CString::from(source));
            let address = StringBytes::as_bytes(&storage).as_ptr();
            assert_into_bytes_reuses_allocation(storage, expected, address, input.len());

            let storage: Box<dyn StringBytes> = Box::new(CString::from(source));
            let address = StringBytes::as_bytes(&storage).as_ptr();
            assert_into_bytes_reuses_allocation(storage, expected, address, input.len());

            let storage = Rc::new(CString::from(source));
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let weak = Rc::downgrade(&storage);
            assert_into_bytes_reuses_allocation(storage, expected, address, input.len());
            assert!(weak.upgrade().is_none());

            let storage: Rc<dyn StringBytes> = Rc::new(CString::from(source));
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let weak = Rc::downgrade(&storage);
            assert_into_bytes_reuses_allocation(storage, expected, address, input.len());
            assert!(weak.upgrade().is_none());

            let storage = Arc::new(CString::from(source));
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let weak = Arc::downgrade(&storage);
            assert_into_bytes_reuses_allocation(storage, expected, address, input.len());
            assert!(weak.upgrade().is_none());

            let storage: Arc<dyn StringBytes + Send + Sync> = Arc::new(CString::from(source));
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let weak = Arc::downgrade(&storage);
            assert_into_bytes_reuses_allocation(storage, expected, address, input.len());
            assert!(weak.upgrade().is_none());
        }
    }

    #[test]
    fn into_bytes_copies_shared_c_string_storage() {
        for input in INPUTS {
            let source = CStr::from_bytes_with_nul(input).unwrap();

            let storage = Rc::new(CString::from(source));
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let bytes = assert_into_bytes_preserves_bytes(Rc::clone(&storage));
            assert_eq!(bytes, &input[..input.len() - 1]);
            if !bytes.is_empty() {
                assert_ne!(bytes.as_ptr(), address);
            }
            assert_eq!(storage.as_bytes_with_nul(), input);

            let storage = Arc::new(CString::from(source));
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let bytes = assert_into_bytes_preserves_bytes(Arc::clone(&storage));
            assert_eq!(bytes, &input[..input.len() - 1]);
            if !bytes.is_empty() {
                assert_ne!(bytes.as_ptr(), address);
            }
            assert_eq!(storage.as_bytes_with_nul(), input);
        }
    }

    #[test]
    fn try_into_string_reuses_the_box_allocation() {
        for input in INPUTS {
            let source = CStr::from_bytes_with_nul(input).unwrap();
            let expected = &input[..input.len() - 1];
            let storage = Box::<CStr>::from(source);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            // On invalid UTF-8, the error retains this allocation too.
            assert_try_into_string_reuses_allocation(storage, expected, address, input.len());

            let storage = Box::<CStr>::from(source);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let storage: Box<dyn StringBytes> = Box::new(storage);
            assert_try_into_string_reuses_allocation(storage, expected, address, input.len());
        }
    }

    #[test]
    fn try_into_string_copies_rc_and_arc_storage() {
        for input in INPUTS {
            let source = CStr::from_bytes_with_nul(input).unwrap();
            for shared in [false, true] {
                let storage = Rc::<CStr>::from(source);
                let address = StringBytes::as_bytes(&storage).as_ptr();
                let other = shared.then(|| Rc::clone(&storage));
                let bytes = assert_try_into_string_preserves_bytes(storage)
                    .map_or_else(FromUtf8Error::into_bytes, String::into_bytes);
                if !bytes.is_empty() {
                    assert_ne!(bytes.as_ptr(), address);
                }
                if let Some(other) = other {
                    assert_eq!(other.to_bytes_with_nul(), input);
                }

                let storage = Arc::<CStr>::from(source);
                let address = StringBytes::as_bytes(&storage).as_ptr();
                let other = shared.then(|| Arc::clone(&storage));
                let bytes = assert_try_into_string_preserves_bytes(storage)
                    .map_or_else(FromUtf8Error::into_bytes, String::into_bytes);
                if !bytes.is_empty() {
                    assert_ne!(bytes.as_ptr(), address);
                }
                if let Some(other) = other {
                    assert_eq!(other.to_bytes_with_nul(), input);
                }
            }
        }
    }

    #[test]
    fn try_into_string_reuses_c_string_allocations() {
        for input in INPUTS {
            let source = CStr::from_bytes_with_nul(input).unwrap();
            let expected = &input[..input.len() - 1];

            let storage = CString::from(source);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            assert_try_into_string_reuses_allocation(storage, expected, address, input.len());

            let storage = Box::new(CString::from(source));
            let address = StringBytes::as_bytes(&storage).as_ptr();
            assert_try_into_string_reuses_allocation(storage, expected, address, input.len());

            let storage: Box<dyn StringBytes> = Box::new(CString::from(source));
            let address = StringBytes::as_bytes(&storage).as_ptr();
            assert_try_into_string_reuses_allocation(storage, expected, address, input.len());

            let storage = Rc::new(CString::from(source));
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let weak = Rc::downgrade(&storage);
            assert_try_into_string_reuses_allocation(storage, expected, address, input.len());
            assert!(weak.upgrade().is_none());

            let storage: Rc<dyn StringBytes> = Rc::new(CString::from(source));
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let weak = Rc::downgrade(&storage);
            assert_try_into_string_reuses_allocation(storage, expected, address, input.len());
            assert!(weak.upgrade().is_none());

            let storage = Arc::new(CString::from(source));
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let weak = Arc::downgrade(&storage);
            assert_try_into_string_reuses_allocation(storage, expected, address, input.len());
            assert!(weak.upgrade().is_none());

            let storage: Arc<dyn StringBytes + Send + Sync> = Arc::new(CString::from(source));
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let weak = Arc::downgrade(&storage);
            assert_try_into_string_reuses_allocation(storage, expected, address, input.len());
            assert!(weak.upgrade().is_none());
        }
    }

    #[test]
    fn try_into_string_copies_shared_c_string_storage() {
        for input in INPUTS {
            let source = CStr::from_bytes_with_nul(input).unwrap();

            let storage = Rc::new(CString::from(source));
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let bytes = assert_try_into_string_preserves_bytes(Rc::clone(&storage))
                .map_or_else(FromUtf8Error::into_bytes, String::into_bytes);
            assert_eq!(bytes, &input[..input.len() - 1]);
            if !bytes.is_empty() {
                assert_ne!(bytes.as_ptr(), address);
            }
            assert_eq!(storage.as_bytes_with_nul(), input);

            let storage = Arc::new(CString::from(source));
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let bytes = assert_try_into_string_preserves_bytes(Arc::clone(&storage))
                .map_or_else(FromUtf8Error::into_bytes, String::into_bytes);
            assert_eq!(bytes, &input[..input.len() - 1]);
            if !bytes.is_empty() {
                assert_ne!(bytes.as_ptr(), address);
            }
            assert_eq!(storage.as_bytes_with_nul(), input);
        }
    }

    #[test]
    fn into_string_lossy_reuses_valid_the_box_allocation() {
        for input in INPUTS {
            let source = CStr::from_bytes_with_nul(input).unwrap();
            let expected = &input[..input.len() - 1];
            let storage = Box::<CStr>::from(source);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            // Valid UTF-8 retains the buffer, including the terminator's capacity.
            assert_into_string_lossy_reuses_valid_allocation(
                storage,
                expected,
                address,
                input.len(),
            );

            let storage = Box::<CStr>::from(source);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let storage: Box<dyn StringBytes> = Box::new(storage);
            assert_into_string_lossy_reuses_valid_allocation(
                storage,
                expected,
                address,
                input.len(),
            );
        }
    }

    #[test]
    fn into_string_lossy_reuses_valid_c_string_allocations() {
        for input in INPUTS {
            let source = CStr::from_bytes_with_nul(input).unwrap();
            let expected = &input[..input.len() - 1];

            let storage = CString::from(source);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            assert_into_string_lossy_reuses_valid_allocation(
                storage,
                expected,
                address,
                input.len(),
            );

            let storage = Box::new(CString::from(source));
            let address = StringBytes::as_bytes(&storage).as_ptr();
            assert_into_string_lossy_reuses_valid_allocation(
                storage,
                expected,
                address,
                input.len(),
            );

            let storage: Box<dyn StringBytes> = Box::new(CString::from(source));
            let address = StringBytes::as_bytes(&storage).as_ptr();
            assert_into_string_lossy_reuses_valid_allocation(
                storage,
                expected,
                address,
                input.len(),
            );

            let storage = Rc::new(CString::from(source));
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let weak = Rc::downgrade(&storage);
            assert_into_string_lossy_reuses_valid_allocation(
                storage,
                expected,
                address,
                input.len(),
            );
            assert!(weak.upgrade().is_none());

            let storage: Rc<dyn StringBytes> = Rc::new(CString::from(source));
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let weak = Rc::downgrade(&storage);
            assert_into_string_lossy_reuses_valid_allocation(
                storage,
                expected,
                address,
                input.len(),
            );
            assert!(weak.upgrade().is_none());

            let storage = Arc::new(CString::from(source));
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let weak = Arc::downgrade(&storage);
            assert_into_string_lossy_reuses_valid_allocation(
                storage,
                expected,
                address,
                input.len(),
            );
            assert!(weak.upgrade().is_none());

            let storage: Arc<dyn StringBytes + Send + Sync> = Arc::new(CString::from(source));
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let weak = Arc::downgrade(&storage);
            assert_into_string_lossy_reuses_valid_allocation(
                storage,
                expected,
                address,
                input.len(),
            );
            assert!(weak.upgrade().is_none());
        }
    }

    #[test]
    fn into_string_unchecked_reuses_the_box_allocation() {
        for input in INPUTS
            .into_iter()
            .filter(|input| std::str::from_utf8(input).is_ok())
        {
            let source = CStr::from_bytes_with_nul(input).unwrap();
            let expected = &input[..input.len() - 1];
            let storage = Box::<CStr>::from(source);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            assert_into_string_unchecked_reuses_allocation(storage, expected, address, input.len());

            let storage = Box::<CStr>::from(source);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let storage: Box<dyn StringBytes> = Box::new(storage);
            assert_into_string_unchecked_reuses_allocation(storage, expected, address, input.len());
        }
    }

    #[test]
    fn into_string_unchecked_reuses_c_string_allocations() {
        for input in INPUTS
            .into_iter()
            .filter(|input| std::str::from_utf8(input).is_ok())
        {
            let source = CStr::from_bytes_with_nul(input).unwrap();
            let expected = &input[..input.len() - 1];

            let storage = CString::from(source);
            let address = StringBytes::as_bytes(&storage).as_ptr();
            assert_into_string_unchecked_reuses_allocation(storage, expected, address, input.len());

            let storage = Box::new(CString::from(source));
            let address = StringBytes::as_bytes(&storage).as_ptr();
            assert_into_string_unchecked_reuses_allocation(storage, expected, address, input.len());

            let storage: Box<dyn StringBytes> = Box::new(CString::from(source));
            let address = StringBytes::as_bytes(&storage).as_ptr();
            assert_into_string_unchecked_reuses_allocation(storage, expected, address, input.len());

            let storage = Rc::new(CString::from(source));
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let weak = Rc::downgrade(&storage);
            assert_into_string_unchecked_reuses_allocation(storage, expected, address, input.len());
            assert!(weak.upgrade().is_none());

            let storage: Rc<dyn StringBytes> = Rc::new(CString::from(source));
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let weak = Rc::downgrade(&storage);
            assert_into_string_unchecked_reuses_allocation(storage, expected, address, input.len());
            assert!(weak.upgrade().is_none());

            let storage = Arc::new(CString::from(source));
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let weak = Arc::downgrade(&storage);
            assert_into_string_unchecked_reuses_allocation(storage, expected, address, input.len());
            assert!(weak.upgrade().is_none());

            let storage: Arc<dyn StringBytes + Send + Sync> = Arc::new(CString::from(source));
            let address = StringBytes::as_bytes(&storage).as_ptr();
            let weak = Arc::downgrade(&storage);
            assert_into_string_unchecked_reuses_allocation(storage, expected, address, input.len());
            assert!(weak.upgrade().is_none());
        }
    }
}
