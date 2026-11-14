mod bytes;
mod pointer;
mod str;

// Delegate conversions to the inner value, moving it out when ownership allows.
// This preserves its allocation reuse and UTF-8 validation behavior.
macro_rules! impl_pointer_conversions_for_sized {
    () => {
        impl_pointer_conversions_for_sized! {
            /// Moves out the boxed value and uses [`Self::into_bytes`].
            #[inline]
            fn box_into_bytes(self: Box<Self>) -> Vec<u8> {
                (*self).into_bytes()
            }

            /// Moves out the boxed value and uses [`Self::try_into_string`].
            #[inline]
            fn box_into_string(self: Box<Self>) -> Result<String, FromUtf8Error> {
                // Moving out preserves any buffer owned by the inner value.
                (*self).try_into_string()
            }
        }
    };

    ($($box_conversion:item)+) => {
        $($box_conversion)+

        /// Uses [`Self::into_bytes`] when this is the only strong owner,
        /// or [`Self::to_bytes`] when the value is shared.
        #[inline]
        fn rc_into_bytes(self: Rc<Self>) -> Vec<u8> {
            match Rc::try_unwrap(self) {
                Ok(value) => value.into_bytes(),
                Err(shared) => (*shared).to_bytes(),
            }
        }

        /// Uses [`Self::into_bytes`] when this is the only strong owner,
        /// or [`Self::to_bytes`] when the value is shared.
        #[inline]
        fn arc_into_bytes(self: Arc<Self>) -> Vec<u8> {
            match Arc::try_unwrap(self) {
                Ok(value) => value.into_bytes(),
                Err(shared) => (*shared).to_bytes(),
            }
        }

        /// Uses [`Self::try_into_string`] when this is the only strong owner,
        /// or [`Self::try_to_string`] when the value is shared.
        #[inline]
        fn rc_into_string(self: Rc<Self>) -> Result<String, FromUtf8Error> {
            // Weak references do not prevent taking ownership of the value.
            match Rc::try_unwrap(self) {
                Ok(value) => value.try_into_string(),
                Err(shared) => (&*shared).try_to_string(),
            }
        }

        /// Uses [`Self::try_into_string`] when this is the only strong owner,
        /// or [`Self::try_to_string`] when the value is shared.
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
// written at the call site. Attributes document the StringBytes implementation.
// Generic parameters go in brackets before the type,
// as in [const N: usize]; [u8; N].
macro_rules! impl_string_bytes {
    ($(#[$attr:meta])* [$($($generics:tt)+)?]; $storage:ty { $($methods:item)* }) => {
        impl $(<$($generics)+>)? sealed::Sealed for $storage {}

        $(#[$attr])*
        impl $(<$($generics)+>)? StringBytes for $storage {
            $($methods)*
        }
    };
    ($(#[$attr:meta])* $storage:ty { $($methods:item)* }) => {
        impl_string_bytes!($(#[$attr])* []; $storage { $($methods)* });
    };
}

// Import shared macros explicitly so child module order does not matter.
use impl_pointer_conversions_for_sized;
use impl_string_bytes;
