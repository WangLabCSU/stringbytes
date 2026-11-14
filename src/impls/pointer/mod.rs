mod reference;
mod reference_mut;

mod arc;
mod boxed;
mod rc;

// A blanket Deref implementation could expose different bytes on each call.
// Limit delegation to the pointer types below.
macro_rules! impl_string_bytes_for_pointer {
    ($($(#[$attr:meta])* $pointer:ty => $into_bytes:ident, $into_string:ident),+ $(,)?) => {
        $(
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
                /// Returns [`FromUtf8Error`] containing a copy of the bytes if they
                /// are not valid UTF-8.
                #[inline]
                fn try_to_string(&self) -> Result<String, FromUtf8Error> {
                    T::try_to_string(&**self)
                }

                /// Converts the pointed-to value lossily without consuming it.
                ///
                /// Valid UTF-8 is borrowed; invalid UTF-8 produces an owned string.
                #[inline]
                fn to_string_lossy(&self) -> Cow<'_, str> {
                    T::to_string_lossy(&**self)
                }

                /// Consumes the pointer and converts its contents into a string.
                ///
                /// Allocation reuse and UTF-8 checks follow the pointed-to value's
                /// conversion for this pointer.
                ///
                /// # Errors
                ///
                /// Returns [`FromUtf8Error`] containing the converted bytes if they
                /// are not valid UTF-8.
                #[inline]
                fn try_into_string(self) -> Result<String, FromUtf8Error> {
                    T::$into_string(self)
                }

                // Delegate conversions when this pointer is itself owned by another.
                impl_pointer_conversions_for_sized!();
            }
        )+
    };
}

use impl_string_bytes_for_pointer;

#[cfg(test)]
mod fixtures {
    use std::borrow::Cow;
    use std::rc::Rc;
    use std::string::FromUtf8Error;
    use std::sync::Arc;

    use crate::impls::impl_pointer_conversions_for_sized;
    use crate::StringBytes;

    // Checked forwarding must not fall back to byte conversions, even if
    // they would produce the same text.
    pub(super) struct CheckedTextOverrides(pub(super) &'static str);

    impl crate::sealed::Sealed for CheckedTextOverrides {}

    impl StringBytes for CheckedTextOverrides {
        fn as_bytes(&self) -> &[u8] {
            panic!("checked forwarding must call the text override")
        }

        fn try_borrow_str(&self) -> Result<&str, std::str::Utf8Error> {
            Ok(self.0)
        }

        fn try_to_string(&self) -> Result<String, FromUtf8Error> {
            Ok(self.0.to_owned())
        }

        fn try_into_string(self) -> Result<String, FromUtf8Error> {
            Ok(self.0.to_owned())
        }

        impl_pointer_conversions_for_sized!();
    }

    // Lossy conversion borrows text through the override, then may take
    // ownership through the byte conversions.
    pub(super) struct LossyTextOverride(pub(super) &'static str);

    impl crate::sealed::Sealed for LossyTextOverride {}

    impl StringBytes for LossyTextOverride {
        fn as_bytes(&self) -> &[u8] {
            panic!("forwarding must preserve the text override")
        }

        fn to_bytes(&self) -> Vec<u8> {
            self.0.as_bytes().to_vec()
        }

        fn into_bytes(self) -> Vec<u8> {
            self.0.as_bytes().to_vec()
        }

        fn to_string_lossy(&self) -> Cow<'_, str> {
            Cow::Borrowed(self.0)
        }
    }

    pub(super) struct NoPrevalidation(pub(super) &'static [u8]);

    impl crate::sealed::Sealed for NoPrevalidation {}

    impl StringBytes for NoPrevalidation {
        fn as_bytes(&self) -> &[u8] {
            self.0
        }

        fn try_borrow_str(&self) -> Result<&str, std::str::Utf8Error> {
            panic!("owned conversion must not prevalidate through try_borrow_str")
        }
        impl_pointer_conversions_for_sized!();
    }

    pub(super) struct NoIntermediateCopy(pub(super) &'static [u8]);

    impl crate::sealed::Sealed for NoIntermediateCopy {}

    impl StringBytes for NoIntermediateCopy {
        fn as_bytes(&self) -> &[u8] {
            self.0
        }

        fn to_bytes(&self) -> Vec<u8> {
            panic!("lossy conversion must not copy into an intermediate Vec")
        }

        fn into_bytes(self) -> Vec<u8> {
            panic!("lossy conversion must not move into an intermediate Vec")
        }
    }
}
