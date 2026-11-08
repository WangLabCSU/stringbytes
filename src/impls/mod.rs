mod bytes;
mod pointer;
mod string;

// Sized contents can delegate conversions to the inner value. Boxed arrays
// instead transfer their own allocation.
// Delegating string conversions preserves the inner type's UTF-8 fast path,
// so wrappers around String or Cow<str> do not introduce another check.
macro_rules! impl_pointer_conversions_for_sized {
    () => {
        impl_pointer_conversions_for_sized! {
            #[inline]
            fn box_into_bytes(self: Box<Self>) -> Vec<u8> {
                (*self).into_bytes()
            }

            #[inline]
            fn box_into_string(self: Box<Self>) -> Result<String, FromUtf8Error> {
                // Moving out of the Box preserves the inner value's buffer.
                (*self).try_into_string()
            }
        }
    };

    ($($box_conversion:item)+) => {
        $($box_conversion)+

        #[inline]
        fn rc_into_bytes(self: Rc<Self>) -> Vec<u8> {
            match Rc::try_unwrap(self) {
                Ok(value) => value.into_bytes(),
                Err(shared) => (*shared).to_bytes(),
            }
        }

        #[inline]
        fn arc_into_bytes(self: Arc<Self>) -> Vec<u8> {
            match Arc::try_unwrap(self) {
                Ok(value) => value.into_bytes(),
                Err(shared) => (*shared).to_bytes(),
            }
        }

        #[inline]
        fn rc_into_string(self: Rc<Self>) -> Result<String, FromUtf8Error> {
            // Weak references do not prevent taking ownership of the value.
            match Rc::try_unwrap(self) {
                Ok(value) => value.try_into_string(),
                Err(shared) => (&*shared).try_to_string(),
            }
        }

        #[inline]
        fn arc_into_string(self: Arc<Self>) -> Result<String, FromUtf8Error> {
            // Keep our Arc alive while copying if another strong owner remains.
            match Arc::try_unwrap(self) {
                Ok(value) => value.try_into_string(),
                Err(shared) => (&*shared).try_to_string(),
            }
        }
    };
}

// Keep Sealed and StringBytes implementations together, with every method
// written at the call site. Generic parameters go in brackets before the type,
// as in [const N: usize]; [u8; N].
macro_rules! impl_string_bytes {
    ([$($($generics:tt)+)?]; $storage:ty { $($methods:item)* }) => {
        impl $(<$($generics)+>)? sealed::Sealed for $storage {}

        impl $(<$($generics)+>)? StringBytes for $storage {
            $($methods)*
        }
    };
    ($storage:ty { $($methods:item)* }) => {
        impl_string_bytes!([]; $storage { $($methods)* });
    };
}

// Import shared macros explicitly so child module order does not matter.
use impl_pointer_conversions_for_sized;
use impl_string_bytes;

#[cfg(test)]
mod test_support {
    use crate::StringBytes;

    pub(super) fn assert_reuses_allocation<T: StringBytes>(
        storage: T,
        expected: &[u8],
        address: *const u8,
        capacity: usize,
        into_bytes: bool,
    ) {
        let bytes = if into_bytes {
            storage.into_bytes()
        } else {
            let result = storage.try_into_string();
            if std::str::from_utf8(expected).is_ok() {
                result.unwrap().into_bytes()
            } else {
                result.unwrap_err().into_bytes()
            }
        };
        assert_eq!(bytes, expected);
        assert_eq!(bytes.as_ptr(), address);
        assert_eq!(bytes.capacity(), capacity);
    }
}
