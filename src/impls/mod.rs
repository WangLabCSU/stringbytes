mod bytes;
mod c_str;
mod io;
mod pointer;
mod string;

// Sized contents can delegate conversions to the inner value. Boxed arrays
// instead transfer their own allocation.
// Delegating string conversions preserves the inner type's UTF-8 fast path,
// so wrappers around String, Cow<str>, or Cow<String> do not introduce
// another check.
macro_rules! impl_pointer_conversions_for_sized {
    () => {
        impl_pointer_conversions_for_sized! {
            #[inline]
            fn box_into_bytes(self: Box<Self>) -> Vec<u8> {
                (*self).into_bytes()
            }

            #[inline]
            fn box_into_string(self: Box<Self>) -> Result<String, FromUtf8Error> {
                // Moving out preserves any buffer owned by the inner value.
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
