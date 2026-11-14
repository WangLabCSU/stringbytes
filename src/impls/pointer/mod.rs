mod reference;
mod reference_mut;

mod arc;
mod boxed;
mod rc;

// A blanket Deref implementation could expose different bytes on each call.
// Limit delegation to the pointer types below.
macro_rules! impl_string_bytes_for_pointer {
    ($(#[$attr:meta])* $pointer:ty => $into_bytes:ident $(, $into_string:ident)? $(,)?) => {
        impl<T: StringBytes + ?Sized> sealed::Sealed for $pointer {}

        $(#[$attr])*
        impl<T: StringBytes + ?Sized> StringBytes for $pointer {
            /// Borrows the bytes exposed by the pointed-to value.
            #[inline]
            fn as_bytes(&self) -> &[u8] {
                // Call T's implementation to avoid recursing through this wrapper.
                T::as_bytes(&**self)
            }

            /// Copies the pointed-to value's bytes, leaving it unchanged.
            #[inline]
            fn to_bytes(&self) -> Vec<u8> {
                T::to_bytes(&**self)
            }

            /// Consumes the pointer and converts its contents into bytes.
            ///
            /// Allocation reuse follows this pointer's conversion rules.
            #[inline]
            fn into_bytes(self) -> Vec<u8> {
                T::$into_bytes(self)
            }

            /// Borrows a string slice using the pointed-to value's UTF-8 checks.
            ///
            /// # Errors
            ///
            /// Returns [`Utf8Error`] if the bytes are not valid UTF-8.
            #[inline]
            fn try_borrow_str(&self) -> Result<&str, Utf8Error> {
                // Preserve T's fast path for text that is already valid UTF-8.
                T::try_borrow_str(&**self)
            }

            /// Copies a string using the pointed-to value's UTF-8 checks.
            ///
            /// # Errors
            ///
            /// Returns [`Utf8Error`] for invalid UTF-8, without copying the bytes.
            #[inline]
            fn try_to_string(&self) -> Result<String, Utf8Error> {
                T::try_to_string(&**self)
            }

            /// Converts the pointed-to value lossily without consuming it.
            ///
            /// Valid UTF-8 is borrowed; invalid UTF-8 produces an owned string.
            #[inline]
            fn to_string_lossy(&self) -> Cow<'_, str> {
                T::to_string_lossy(&**self)
            }

            $(
            /// Consumes the pointer and converts its contents into a string.
            ///
            /// Allocation reuse and UTF-8 checks follow the pointed-to value's
            /// conversion for this pointer.
            ///
            /// # Errors
            ///
            /// Returns [`FromUtf8Error`] containing the original pointer if the
            /// bytes are not valid UTF-8.
            #[inline]
            fn try_into_string(self) -> Result<String, FromUtf8Error<Self>> {
                T::$into_string(self)
            }
            )?

            // Delegate conversions when this pointer is itself owned by another.
            impl_pointer_conversions_for_sized!();
        }
    };
}

use impl_string_bytes_for_pointer;
