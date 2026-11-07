#![doc = include_str!("../README.md")]

use std::borrow::Cow;
use std::rc::Rc;
use std::str::{self, Utf8Error};
use std::string::FromUtf8Error;
use std::sync::Arc;

#[cfg(feature = "bytes")]
use bytes::{Bytes, BytesMut};

mod sealed;

/// A trait for byte sequences and UTF-8 strings.
///
/// Types implementing `StringBytes` can be used as the storage parameter of
/// generic string and byte-buffer types. Implementations expose a consistent
/// byte sequence and preserve it when converting to `str` or [`String`].
///
/// The bytes need not be valid UTF-8. [`as_bytes`](Self::as_bytes) borrows them
/// directly, while [`try_borrow_str`](Self::try_borrow_str) checks UTF-8 when
/// needed. To obtain an owned [`String`], use [`try_to_string`](Self::try_to_string)
/// to copy the bytes or [`try_into_string`](Self::try_into_string) to consume the
/// value and reuse its allocation where possible.
///
/// This trait is sealed and cannot be implemented outside this crate.
///
/// # Guarantees
///
/// All implementations obey the following rules:
///
/// - [`Self::as_bytes`] returns the same bytes unless the value is modified
///   through mutable or owned access. Interior mutability must not change the
///   bytes exposed by this method, `AsRef<[u8]>`, or `AsMut<[u8]>`.
/// - If `AsRef<[u8]>` is implemented, it borrows the same slice as `as_bytes`.
/// - If `AsMut<[u8]>` is implemented, it mutably borrows that same slice.
/// - Successful calls to [`Self::try_borrow_str`], [`Self::try_to_string`], and
///   [`Self::try_into_string`] preserve the bytes returned by `as_bytes`.
///
/// The rules for `AsRef` and `AsMut` apply only when those traits are
/// implemented; they are not required by `StringBytes`.
///
/// # Dyn compatibility
///
/// This trait is [dyn compatible].
///
/// In older versions of Rust, dyn compatibility was called "object safety".
///
/// [dyn compatible]: https://doc.rust-lang.org/reference/items/traits.html#dyn-compatibility
pub trait StringBytes: sealed::Sealed {
    /// Returns a borrowed view of the underlying bytes.
    ///
    /// This method does not allocate, copy, or validate UTF-8. The returned
    /// slice may contain any byte sequence.
    ///
    /// # Examples
    ///
    /// ```
    /// use stringbytes::StringBytes;
    ///
    /// let bytes = [b'a', 0xff];
    /// assert_eq!(bytes.as_bytes(), b"a\xff");
    /// ```
    fn as_bytes(&self) -> &[u8];

    /// Returns a string slice if the bytes are valid UTF-8.
    ///
    /// The slice borrows directly from `self`; no allocation or copying is
    /// needed. Byte sequences are validated on each call. Types that already
    /// guarantee UTF-8 skip validation and always return `Ok`.
    ///
    /// # Errors
    ///
    /// Returns [`Utf8Error`] if the bytes are not valid UTF-8.
    ///
    /// # Examples
    ///
    /// ```
    /// use stringbytes::StringBytes;
    ///
    /// let bytes = b"hello";
    /// assert_eq!(bytes.try_borrow_str()?, "hello");
    /// # Ok::<(), std::str::Utf8Error>(())
    /// ```
    #[inline]
    fn try_borrow_str(&self) -> Result<&str, Utf8Error> {
        str::from_utf8(self.as_bytes())
    }

    /// Copies the bytes into a [`String`], leaving `self` unchanged.
    ///
    /// Byte sequences are validated once per call. Types that already
    /// guarantee UTF-8 are copied without validation and always return `Ok`.
    ///
    /// # Errors
    ///
    /// Returns [`FromUtf8Error`] containing the copied bytes if they are not
    /// valid UTF-8.
    ///
    /// # Examples
    ///
    /// ```
    /// use stringbytes::StringBytes;
    ///
    /// let bytes = vec![b'h', b'i'];
    /// let text = bytes.try_to_string()?;
    /// assert_eq!(text, "hi");
    /// assert_eq!(bytes, b"hi");
    /// # Ok::<(), std::string::FromUtf8Error>(())
    /// ```
    #[inline]
    fn try_to_string(&self) -> Result<String, FromUtf8Error> {
        String::from_utf8(self.as_bytes().to_vec())
    }

    /// Converts `self` into a [`String`], consuming the value.
    ///
    /// Reuses the existing allocation where possible and copies the bytes
    /// otherwise. When `self` is a reference, the bytes are copied and the
    /// referenced value is left unchanged.
    ///
    /// Byte sequences are validated once. Types that already guarantee UTF-8
    /// skip validation and always return `Ok`.
    ///
    /// # Allocation reuse
    ///
    /// - [`Vec<u8>`] and [`String`] keep their buffer and capacity.
    /// - `Box<[u8]>`, `Box<[u8; N]>`, and `Box<str>` reuse their allocation.
    ///   Other boxed values are moved out and converted according to their type.
    /// - [`Cow`] reuses owned storage and copies borrowed storage.
    /// - [`Rc<T>`](Rc) and [`Arc<T>`](Arc), where `T: Sized`, take ownership of
    ///   the inner value when only one strong reference remains, then use `T`'s
    ///   conversion. Otherwise, they copy the bytes. Weak references do not
    ///   prevent this. `Rc<[u8]>`, `Arc<[u8]>`, `Rc<str>`, and `Arc<str>` always copy.
    /// - References and unboxed byte arrays are copied.
    /// - With the `bytes` feature, `Bytes` and `BytesMut` use their conversion to
    ///   `Vec<u8>`. Static data, shared buffers, and `Bytes::from_owner` storage
    ///   are copied. Reusing a sliced buffer may move bytes within its allocation.
    ///
    /// Nested wrappers and trait objects follow the same conversion rules.
    ///
    /// # Errors
    ///
    /// Returns [`FromUtf8Error`] for invalid UTF-8. The error keeps the buffer
    /// used for conversion, following the same allocation rules as a successful
    /// result. Use [`into_bytes`](FromUtf8Error::into_bytes) to recover the bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use stringbytes::StringBytes;
    ///
    /// let bytes = Box::<[u8]>::from("hello".as_bytes());
    /// assert_eq!(bytes.try_into_string()?, "hello");
    ///
    /// let invalid = vec![0xff];
    /// let error = invalid.try_into_string().unwrap_err();
    /// assert_eq!(error.utf8_error().valid_up_to(), 0);
    /// assert_eq!(error.into_bytes(), vec![0xff]);
    /// # Ok::<(), std::string::FromUtf8Error>(())
    /// ```
    #[inline]
    fn try_into_string(self) -> Result<String, FromUtf8Error>
    where
        Self: Sized,
    {
        // Types that can reuse an allocation or skip validation override this.
        String::from_utf8(self.as_bytes().to_vec())
    }

    // Owned pointer receivers preserve the concrete type's conversion through
    // dyn StringBytes.

    /// Converts the contents of a [`Box`] into a [`String`].
    ///
    /// Boxed byte slices, byte arrays, and strings reuse their allocation.
    /// Other boxed values are moved out and converted with [`Self::try_into_string`].
    /// This also supports `Box<dyn StringBytes>`.
    ///
    /// # Errors
    ///
    /// Returns [`FromUtf8Error`] if the stored bytes are not valid UTF-8.
    #[doc(hidden)]
    #[inline]
    fn box_into_string(self: Box<Self>) -> Result<String, FromUtf8Error> {
        // Unsized types cannot be moved out of a Box. Implementations that can
        // reuse the allocation override this copying fallback.
        (*self).try_to_string()
    }

    /// Converts the contents of an [`Rc`] into a [`String`].
    ///
    /// With one strong owner, sized contents are moved out and converted with
    /// [`Self::try_into_string`]. Shared contents, slices, and `str` are copied.
    /// Trait objects use their concrete type's conversion.
    ///
    /// # Errors
    ///
    /// Returns [`FromUtf8Error`] if the stored bytes are not valid UTF-8.
    #[doc(hidden)]
    #[inline]
    fn rc_into_string(self: Rc<Self>) -> Result<String, FromUtf8Error> {
        // Rc::try_unwrap needs Sized; slices and str use this copying fallback.
        (*self).try_to_string()
    }

    /// Converts the contents of an [`Arc`] into a [`String`].
    ///
    /// With one strong owner, sized contents are moved out and converted with
    /// [`Self::try_into_string`]. Shared contents, slices, and `str` are copied.
    /// Trait objects use their concrete type's conversion.
    ///
    /// # Errors
    ///
    /// Returns [`FromUtf8Error`] if the stored bytes are not valid UTF-8.
    #[doc(hidden)]
    #[inline]
    fn arc_into_string(self: Arc<Self>) -> Result<String, FromUtf8Error> {
        // Like Rc, Arc cannot unwrap a slice or str.
        (*self).try_to_string()
    }
}

// Implement consuming pointer methods for sized types. Arrays override the
// Box method to reuse their allocation instead of moving the bytes out.
macro_rules! impl_pointer_into_string_for_sized {
    () => {
        impl_pointer_into_string_for_sized! {
            #[inline]
            fn box_into_string(self: Box<Self>) -> Result<String, FromUtf8Error> {
                // Moving out of the Box preserves the inner value's buffer.
                (*self).try_into_string()
            }
        }
    };

    ($box_into_string:item) => {
        $box_into_string

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

// A blanket Deref implementation could expose different bytes on each call.
// Limit delegation to the pointer types below.
macro_rules! impl_string_bytes_for_pointer {
    ($($pointer:ty => $convert:ident),+ $(,)?) => {
        $(
            impl<T: StringBytes + ?Sized> sealed::Sealed for $pointer {}

            impl<T: StringBytes + ?Sized> StringBytes for $pointer {
                #[inline]
                fn as_bytes(&self) -> &[u8] {
                    // Call T's implementation to avoid recursing through this wrapper.
                    T::as_bytes(&**self)
                }

                #[inline]
                fn try_borrow_str(&self) -> Result<&str, Utf8Error> {
                    // Preserve T's fast path for text that is already valid UTF-8.
                    T::try_borrow_str(&**self)
                }

                #[inline]
                fn try_to_string(&self) -> Result<String, FromUtf8Error> {
                    T::try_to_string(&**self)
                }

                #[inline]
                fn try_into_string(self) -> Result<String, FromUtf8Error> {
                    T::$convert(self)
                }

                // Support nested owners such as Rc<Box<str>>.
                impl_pointer_into_string_for_sized!();
            }
        )+
    };
}

impl_string_bytes_for_pointer!(&T => try_to_string);
impl_string_bytes_for_pointer!(&mut T => try_to_string);
impl_string_bytes_for_pointer!(Box<T> => box_into_string);
impl_string_bytes_for_pointer!(Rc<T> => rc_into_string);
impl_string_bytes_for_pointer!(Arc<T> => arc_into_string);

// Keep Sealed and StringBytes implementations together. Generic parameters go
// in brackets before the type, as in [const N: usize]; [u8; N].
macro_rules! impl_string_bytes {
    ([$($($generics:tt)+)?]; $storage:ty => $view:path $({ $($methods:tt)* })? $(,)?) => {
        impl $(<$($generics)+>)? sealed::Sealed for $storage {}

        impl $(<$($generics)+>)? StringBytes for $storage {
            #[inline]
            fn as_bytes(&self) -> &[u8] {
                $view(self)
            }

            $($($methods)*)?
        }
    };
    ($storage:ty => $view:path $({ $($methods:tt)* })? $(,)?) => {
        impl_string_bytes!([]; $storage => $view $({ $($methods)* })?);
    };
}

// str already guarantees UTF-8, so none of its conversions need validation.
impl_string_bytes!(
    str => str::as_bytes {
        #[inline]
        fn try_borrow_str(&self) -> Result<&str, Utf8Error> {
            Ok(self)
        }

        #[inline]
        fn try_to_string(&self) -> Result<String, FromUtf8Error> {
            Ok(self.to_owned())
        }

        #[inline]
        fn box_into_string(self: Box<Self>) -> Result<String, FromUtf8Error> {
            // String::from takes over the Box's allocation.
            Ok(String::from(self))
        }
    },
);

// String also guarantees UTF-8 and can be returned directly when consumed.
impl_string_bytes!(
    String => String::as_bytes {
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

        impl_pointer_into_string_for_sized!();
    },
);

// Arrays store bytes inline, but boxed arrays can give their allocation to
// the resulting String.
impl_string_bytes!(
    [const N: usize]; [u8; N] => std::convert::identity {
        #[inline]
        fn try_to_string(&self) -> Result<String, FromUtf8Error> {
            String::from_utf8(self.to_vec())
        }

        #[inline]
        fn try_into_string(self) -> Result<String, FromUtf8Error> {
            self.try_to_string()
        }

        impl_pointer_into_string_for_sized! {
            #[inline]
            fn box_into_string(self: Box<Self>) -> Result<String, FromUtf8Error> {
                // Passing self as an argument lets Box<[u8; N]> coerce to Box<[u8]>.
                // A call to self.into_vec() would not perform that coercion.
                String::from_utf8(<[u8]>::into_vec(self))
            }
        }
    },
);

impl_string_bytes!(
    [u8] => std::convert::identity {
        #[inline]
        fn box_into_string(self: Box<Self>) -> Result<String, FromUtf8Error> {
            // into_vec keeps the boxed allocation.
            String::from_utf8(self.into_vec())
        }
    },
);

impl_string_bytes!(
    Vec<u8> => Vec::as_slice {
        #[inline]
        fn try_into_string(self) -> Result<String, FromUtf8Error> {
            // Both String and FromUtf8Error keep this buffer and its capacity.
            String::from_utf8(self)
        }

        impl_pointer_into_string_for_sized!();
    },
);

#[cfg(feature = "bytes")]
impl_string_bytes!(
    Bytes => Bytes::as_ref {
        #[inline]
        fn try_into_string(self) -> Result<String, FromUtf8Error> {
            // Let bytes decide whether its backing storage can be reused.
            String::from_utf8(Vec::from(self))
        }

        impl_pointer_into_string_for_sized!();
    },
);

#[cfg(feature = "bytes")]
impl_string_bytes!(
    BytesMut => BytesMut::as_ref {
        #[inline]
        fn try_into_string(self) -> Result<String, FromUtf8Error> {
            // Split buffers may still share an allocation. Vec::from handles
            // that sharing and any offset into the original buffer.
            String::from_utf8(Vec::from(self))
        }

        impl_pointer_into_string_for_sized!();
    },
);

// Restrict Cow to byte slices and str so both variants satisfy the trait's
// guarantees.
impl_string_bytes!(
    Cow<'_, [u8]> => Cow::as_ref {
        #[inline]
        fn try_into_string(self) -> Result<String, FromUtf8Error> {
            // Reuse an owned Vec; copy a borrowed slice.
            String::from_utf8(self.into_owned())
        }

        impl_pointer_into_string_for_sized!();
    },
);

impl_string_bytes!(
    Cow<'_, str> => str::as_bytes {
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
            // Both Cow variants already contain valid UTF-8.
            Ok(self.into_owned())
        }

        impl_pointer_into_string_for_sized!();
    },
);

#[cfg(test)]
mod tests {
    use std::borrow::Cow;
    use std::rc::Rc;
    use std::sync::Arc;

    #[cfg(feature = "bytes")]
    use bytes::{Bytes, BytesMut};

    use super::StringBytes;

    fn assert_bytes<T: StringBytes + ?Sized>(value: &T, expected: &[u8]) {
        // The contents must stay the same; the address need not.
        for _ in 0..3 {
            assert_eq!(value.as_bytes(), expected);
        }
    }

    #[test]
    fn shared_access_preserves_the_bytes() {
        // Storage can be empty or contain invalid UTF-8.
        for input in [b"".as_slice(), b"AC", b"A\xff"] {
            assert_bytes(input, input); // T = [u8].
            assert_bytes(&input, input); // T = &[u8].

            let mut vector = Vec::from(input);
            // An extra borrow exercises the reference's StringBytes implementation.
            assert_bytes(&vector, input); // T = Vec<u8>.
            assert_bytes(&&vector, input); // T = &Vec<u8>.
            assert_bytes(&&mut vector, input); // T = &mut Vec<u8>.
            assert_bytes(&Box::<[u8]>::from(input), input);
            assert_bytes(&Rc::<[u8]>::from(input), input);
            assert_bytes(&Arc::<[u8]>::from(input), input);
            assert_bytes(&Cow::Borrowed(input), input);
            assert_bytes(&Cow::<'_, [u8]>::Owned(Vec::from(input)), input);

            #[cfg(feature = "bytes")]
            {
                assert_bytes(&Bytes::from_static(input), input);
                assert_bytes(&BytesMut::from(input), input);
            }
        }

        assert_bytes(&[0_u8; 0], b"");
        assert_bytes(&[0_u8; 4], &[0; 4]);
        assert_bytes(&[0xff_u8; 64], &[0xff; 64]);

        for input in ["", "AC", "é说明"] {
            let expected = input.as_bytes();
            assert_bytes(input, expected);
            assert_bytes(&input, expected);

            let mut text = String::from(input);
            assert_bytes(&text, expected);
            assert_bytes(&&mut text, expected); // T = &mut String.
            assert_bytes(&Box::<str>::from(input), expected);
            assert_bytes(&Rc::<str>::from(input), expected);
            assert_bytes(&Arc::<str>::from(input), expected);
            assert_bytes(&Cow::Borrowed(input), expected);
            assert_bytes(&Cow::<'_, str>::Owned(text), expected);

            // Check that delegation also works through nested pointers.
            let nested = Rc::new(Arc::new(Box::new(String::from(input))));
            assert_bytes(&nested, expected);
        }
    }

    #[test]
    fn as_ref_borrows_the_same_slice_as_as_bytes() {
        fn check<T: StringBytes + AsRef<[u8]> + ?Sized>(value: &T) {
            // Slice pointer equality checks both address and length.
            for _ in 0..3 {
                assert!(std::ptr::eq(value.as_bytes(), value.as_ref()));
            }
        }

        for input in [b"".as_slice(), b"AC", b"A\xff"] {
            check(input);
            check(&input);
            check(&Vec::from(input));
            check(&Box::<[u8]>::from(input));
            check(&Rc::<[u8]>::from(input));
            check(&Arc::<[u8]>::from(input));
            check(&Cow::Borrowed(input));
            check(&Cow::<'_, [u8]>::Owned(Vec::from(input)));

            #[cfg(feature = "bytes")]
            {
                check(&Bytes::from_static(input));
                check(&BytesMut::from(input));
            }
        }

        check(&[0_u8; 0]);
        check(&[0xff_u8; 64]);
        check("é");
        check(&String::from("é"));
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
            assert_bytes(value, &expected);
        }

        for input in [b"".as_slice(), b"AC", b"A\xff"] {
            let mut vector = Vec::from(input);
            // Extra borrows exercise the reference implementations too.
            check_mut(vector.as_mut_slice()); // T = [u8].
            check_mut(&mut vector.as_mut_slice()); // T = &mut [u8].
            check_mut(&mut vector); // T = Vec<u8>.
            check_mut(&mut &mut vector); // T = &mut Vec<u8>.
            check_mut(&mut Box::<[u8]>::from(input));

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
            check::<_, &[u8]>(input);
            check::<_, Vec<u8>>(input);
            check::<_, Box<[u8]>>(input);
            check::<_, Rc<[u8]>>(input);
            check::<_, Arc<[u8]>>(input);
            check::<_, Cow<'_, [u8]>>(input);
            check::<_, Box<&[u8]>>(input);
            check::<_, Rc<&[u8]>>(input);
            check::<_, Arc<&[u8]>>(input);

            // Cover owned sources as well as borrowed slices.
            check::<_, Box<[u8]>>(Vec::from(input));
            check::<_, Rc<[u8]>>(Vec::from(input));
            check::<_, Arc<[u8]>>(Vec::from(input));
            check::<_, Cow<'_, [u8]>>(Vec::from(input));
            check::<_, Vec<u8>>(Box::<[u8]>::from(input));
            check::<_, Rc<[u8]>>(Box::<[u8]>::from(input));
            check::<_, Arc<[u8]>>(Box::<[u8]>::from(input));

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
        check::<_, Box<[u8]>>([0xff; 2]);

        // Text conversions must preserve embedded NULs and decomposed Unicode.
        for input in ["", "é说明", "\0e\u{301}"] {
            check::<_, String>(input);
            check::<_, Box<str>>(String::from(input));
            check::<_, String>(Box::<str>::from(input));
            check::<_, Vec<u8>>(String::from(input));
            check::<_, Cow<'_, str>>(String::from(input));

            #[cfg(feature = "bytes")]
            check::<_, Bytes>(String::from(input));
        }
    }

    #[test]
    fn string_conversions_preserve_the_storage_bytes() {
        fn check<T: StringBytes>(storage: T) {
            // Check contents here; allocation reuse is tested separately.
            let expected = storage.as_bytes().to_vec();
            let copied = storage.try_to_string().unwrap();
            assert_eq!(copied.as_bytes(), expected);
            assert_eq!(storage.as_bytes(), expected);

            let converted = storage.try_into_string().unwrap();
            assert_eq!(converted.as_bytes(), expected);
        }

        // Include embedded NUL and decomposed Unicode to catch any normalization.
        for input in ["", "text", "é说明", "\0e\u{301}"] {
            check(input);
            let mut text = String::from(input);
            check(&text);
            check(&mut text);
            check(text.as_mut_str());
            check(text);
            check(Box::<str>::from(input));
            check(Rc::<str>::from(input));
            check(Arc::<str>::from(input));
            check(Cow::Borrowed(input));
            check(Cow::<'_, str>::Owned(String::from(input)));

            check(input.as_bytes());
            check(Vec::from(input.as_bytes()));
            check(Box::<[u8]>::from(input.as_bytes()));
            check(Rc::<[u8]>::from(input.as_bytes()));
            check(Arc::<[u8]>::from(input.as_bytes()));
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
    fn string_conversions_reject_invalid_utf8() {
        fn check<T: StringBytes>(storage: T) {
            let expected = storage.as_bytes().to_vec();
            let error = storage.try_to_string().unwrap_err();
            assert_eq!(error.as_bytes(), expected);
            assert_eq!(storage.as_bytes(), expected);
            let error = storage.try_into_string().unwrap_err();
            assert_eq!(error.as_bytes(), expected);
        }

        // Reject invalid, truncated, and overlong UTF-8 without lossy replacement.
        for input in [b"\xff".as_slice(), b"\xe2\x82", b"\xc0\x80"] {
            check(input);
            check(Vec::from(input));
            check(Box::<[u8]>::from(input));
            check(Rc::<[u8]>::from(input));
            check(Arc::<[u8]>::from(input));
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
    fn try_borrow_str_borrows_the_same_bytes() {
        fn check<T: StringBytes + ?Sized>(storage: &T, expected: &str) {
            let text = storage.try_borrow_str().unwrap();
            assert_eq!(text, expected);
            assert!(std::ptr::eq(text.as_bytes(), storage.as_bytes()));
        }

        for input in ["", "text", "é说明", "\0e\u{301}"] {
            check(input, input);
            check(input.as_bytes(), input);
            check(&String::from(input), input);
            check(&Box::<str>::from(input), input);
            check(&Rc::<str>::from(input), input);
            check(&Arc::<str>::from(input), input);
            check(&Cow::Borrowed(input), input);
            check(&Cow::<'_, str>::Owned(String::from(input)), input);
            check(&Rc::new(Arc::new(Box::new(String::from(input)))), input);
            check(&Vec::from(input.as_bytes()), input);
            check(&Box::<[u8]>::from(input.as_bytes()), input);
            check(&Rc::<[u8]>::from(input.as_bytes()), input);
            check(&Arc::<[u8]>::from(input.as_bytes()), input);
            check(&Cow::Borrowed(input.as_bytes()), input);

            #[cfg(feature = "bytes")]
            {
                check(&Bytes::from_static(input.as_bytes()), input);
                check(&BytesMut::from(input.as_bytes()), input);
            }
        }
    }

    #[test]
    fn try_borrow_str_reports_invalid_utf8() {
        // Cover an invalid leading byte, a truncated sequence, an overlong
        // encoding, and a surrogate. None marks an incomplete sequence.
        for (input, valid_up_to, error_len) in [
            (b"\xff".as_slice(), 0, Some(1)),
            (b"ok\xe2\x82", 2, None),
            (b"\xc0\x80", 0, Some(1)),
            (b"\xed\xa0\x80", 0, Some(1)),
        ] {
            let error = input.try_borrow_str().unwrap_err();
            assert_eq!(error.valid_up_to(), valid_up_to);
            assert_eq!(error.error_len(), error_len);
        }
    }

    #[test]
    fn string_conversions_do_not_call_try_borrow_str() {
        struct ByteView;

        impl crate::sealed::Sealed for ByteView {}

        impl StringBytes for ByteView {
            fn as_bytes(&self) -> &[u8] {
                "étext".as_bytes()
            }

            fn try_borrow_str(&self) -> Result<&str, std::str::Utf8Error> {
                // String conversions must validate their owned buffer directly.
                panic!("try_into_string must not call try_borrow_str")
            }
        }

        fn check<T: StringBytes>(storage: T) {
            assert_eq!(storage.try_to_string().unwrap(), "étext");
            assert_eq!(storage.try_into_string().unwrap(), "étext");
        }

        let mut text = ByteView;
        check(&text);
        check(&mut text);
        check(text);
        check(Box::new(ByteView));

        let rc = Rc::new(ByteView);
        check(Rc::clone(&rc));
        let arc = Arc::new(ByteView);
        check(Arc::clone(&arc));

        // A second owner forces nested conversions to copy.
        let nested = Rc::new(Arc::new(Box::new(ByteView)));
        check(Rc::clone(&nested));
        let nested = Arc::new(Arc::new(Box::new(ByteView)));
        check(Arc::clone(&nested));

        let boxed: Box<dyn StringBytes> = Box::new(ByteView);
        check(&*boxed);
        check(boxed);
        let rc: Rc<dyn StringBytes> = Rc::new(ByteView);
        check(Rc::clone(&rc));
        let arc: Arc<dyn StringBytes + Send + Sync> = Arc::new(ByteView);
        check(Arc::clone(&arc));
    }

    #[test]
    fn reference_conversions_delegate_to_try_to_string() {
        struct TextView;

        impl crate::sealed::Sealed for TextView {}

        impl StringBytes for TextView {
            fn as_bytes(&self) -> &[u8] {
                // Reference conversions must call try_to_string directly.
                panic!("reference conversion must call try_to_string")
            }

            fn try_to_string(&self) -> Result<String, std::string::FromUtf8Error> {
                Ok(String::from("étext"))
            }
        }

        fn check<T: StringBytes>(storage: T) {
            assert_eq!(storage.try_to_string().unwrap(), "étext");
            assert_eq!(storage.try_into_string().unwrap(), "étext");
        }

        let mut text = TextView;
        check(&text);
        check(&mut text);
        check(Box::new(TextView));

        let nested = Rc::new(Arc::new(Box::new(TextView)));
        check(Rc::clone(&nested));
        let nested = Arc::new(Arc::new(Box::new(TextView)));
        check(Arc::clone(&nested));

        let boxed: Box<dyn StringBytes> = Box::new(TextView);
        check(&*boxed);
        check(boxed);
        let rc: Rc<dyn StringBytes> = Rc::new(TextView);
        check(Rc::clone(&rc));
        let arc: Arc<dyn StringBytes + Send + Sync> = Arc::new(TextView);
        check(Arc::clone(&arc));
    }

    #[test]
    fn try_into_string_reuses_owned_storage() {
        fn check<T: StringBytes>(storage: T, expected: &str, capacity: usize) {
            let pointer = storage.as_bytes().as_ptr();
            let Ok(text) = storage.try_into_string() else {
                panic!("supported owned storage should convert successfully");
            };
            assert_eq!(text, expected);
            // Check for both an unnecessary copy and a loss of spare capacity.
            assert_eq!(text.as_ptr(), pointer);
            assert_eq!(text.capacity(), capacity);
        }

        for input in ["", "text", "é说明", "\0e\u{301}"] {
            // Keep spare capacity even when the contents are empty.
            let mut vector = Vec::with_capacity(input.len() + 16);
            vector.extend_from_slice(input.as_bytes());
            let capacity = vector.capacity();
            check(vector, input, capacity);
            check(Box::<[u8]>::from(input.as_bytes()), input, input.len());

            let mut text = String::with_capacity(input.len() + 16);
            text.push_str(input);
            let capacity = text.capacity();
            check(text, input, capacity);
            check(Box::<str>::from(input), input, input.len());

            let bytes = Vec::from(input.as_bytes());
            let capacity = bytes.capacity();
            check(Cow::<'_, [u8]>::Owned(bytes), input, capacity);
            let text = String::from(input);
            let capacity = text.capacity();
            check(Cow::<'_, str>::Owned(text), input, capacity);

            // A sole owner can pass its inner buffer to the String.
            check(Box::new(Vec::from(input.as_bytes())), input, input.len());
            check(Rc::new(Vec::from(input.as_bytes())), input, input.len());
            check(Arc::new(String::from(input)), input, input.len());
            check(
                Rc::new(Arc::new(Box::new(String::from(input)))),
                input,
                input.len(),
            );

            // Type erasure should preserve allocation reuse.
            let boxed: Box<dyn StringBytes> = Box::new(Vec::from(input.as_bytes()));
            assert_eq!(boxed.try_borrow_str().unwrap(), input);
            check(boxed, input, input.len());

            #[cfg(feature = "bytes")]
            {
                check(Bytes::from(Vec::from(input.as_bytes())), input, input.len());
                let mut bytes = BytesMut::with_capacity(input.len() + 16);
                bytes.extend_from_slice(input.as_bytes());
                let capacity = bytes.capacity();
                check(bytes, input, capacity);
            }
        }

        // Cover empty boxed arrays and allocation reuse through nested owners
        // and trait objects.
        check(Box::new([0_u8; 0]), "", 0);
        check(Box::new(*b"text"), "text", 4);
        check(Rc::new(Box::new(*b"text")), "text", 4);
        let boxed: Box<dyn StringBytes> = Box::new(*b"text");
        check(boxed, "text", 4);
    }

    fn assert_reuses_allocation<T: StringBytes>(
        storage: T,
        expected: &[u8],
        address: *const u8,
        capacity: usize,
    ) {
        let result = storage.try_into_string();
        let bytes = if std::str::from_utf8(expected).is_ok() {
            result.unwrap().into_bytes()
        } else {
            result.unwrap_err().into_bytes()
        };
        assert_eq!(bytes, expected);
        assert_eq!(bytes.as_ptr(), address);
        assert_eq!(bytes.capacity(), capacity);
    }

    #[test]
    fn allocation_reuse_survives_wrappers_and_type_erasure() {
        fn check<T: StringBytes>(storage: T, expected: &[u8], capacity: usize) {
            let address = storage.as_bytes().as_ptr();
            assert_reuses_allocation(storage, expected, address, capacity);
        }

        fn check_owners<T: StringBytes + Send + Sync + 'static>(
            make: impl Fn() -> (T, usize),
            expected: &[u8],
        ) {
            let (storage, capacity) = make();
            check(storage, expected, capacity);
            let (storage, capacity) = make();
            check(Box::new(storage), expected, capacity);
            let (storage, capacity) = make();
            let boxed: Box<dyn StringBytes> = Box::new(storage);
            check(boxed, expected, capacity);

            // Weak references must not force a copy, even after type erasure.
            let (storage, capacity) = make();
            let rc = Rc::new(storage);
            let weak = Rc::downgrade(&rc);
            check(rc, expected, capacity);
            assert!(weak.upgrade().is_none());
            let (storage, capacity) = make();
            let rc: Rc<dyn StringBytes> = Rc::new(storage);
            let weak = Rc::downgrade(&rc);
            check(rc, expected, capacity);
            assert!(weak.upgrade().is_none());

            let (storage, capacity) = make();
            let arc = Arc::new(storage);
            let weak = Arc::downgrade(&arc);
            check(arc, expected, capacity);
            assert!(weak.upgrade().is_none());
            let (storage, capacity) = make();
            let arc: Arc<dyn StringBytes + Send + Sync> = Arc::new(storage);
            let weak = Arc::downgrade(&arc);
            check(arc, expected, capacity);
            assert!(weak.upgrade().is_none());

            let (storage, capacity) = make();
            let nested: Box<dyn StringBytes> = Box::new(Rc::new(Arc::new(Box::new(storage))));
            check(nested, expected, capacity);
        }

        for input in [b"".as_slice(), b"text", "é说明".as_bytes(), b"ok\xff"] {
            let make_vec = || {
                let mut bytes = Vec::with_capacity(input.len() + 32);
                bytes.extend_from_slice(input);
                let capacity = bytes.capacity();
                (bytes, capacity)
            };
            check_owners(make_vec, input);
            check_owners(|| (Box::<[u8]>::from(input), input.len()), input);
            check_owners(
                || {
                    let (bytes, capacity) = make_vec();
                    (Cow::<'static, [u8]>::Owned(bytes), capacity)
                },
                input,
            );

            if let Ok(text) = std::str::from_utf8(input) {
                let make_string = || {
                    let mut value = String::with_capacity(text.len() + 32);
                    value.push_str(text);
                    let capacity = value.capacity();
                    (value, capacity)
                };
                check_owners(make_string, input);
                check_owners(|| (Box::<str>::from(text), text.len()), input);
                check_owners(
                    || {
                        let (text, capacity) = make_string();
                        (Cow::<'static, str>::Owned(text), capacity)
                    },
                    input,
                );
            }

            #[cfg(feature = "bytes")]
            {
                check_owners(
                    || {
                        let (bytes, capacity) = make_vec();
                        (Bytes::from(bytes), capacity)
                    },
                    input,
                );
                check_owners(
                    || (Bytes::from(Box::<[u8]>::from(input)), input.len()),
                    input,
                );
                let make_mut = || {
                    let mut bytes = BytesMut::with_capacity(input.len() + 32);
                    bytes.extend_from_slice(input);
                    let capacity = bytes.capacity();
                    (bytes, capacity)
                };
                check_owners(make_mut, input);
                check_owners(
                    || {
                        let (bytes, capacity) = make_mut();
                        (bytes.freeze(), capacity)
                    },
                    input,
                );
            }
        }

        check_owners(|| (Box::new(*b"text"), 4), b"text");
        check_owners(|| (Box::new([b'a', 0xff]), 2), b"a\xff");
        check_owners(|| (Box::new([0_u8; 0]), 0), b"");
    }

    #[cfg(feature = "bytes")]
    #[test]
    fn sliced_bytes_reuse_the_allocation_after_other_views_are_dropped() {
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
            assert_reuses_allocation(slice, input, address, capacity);

            let mut original = BytesMut::with_capacity(64);
            original.extend_from_slice(b"prefix");
            original.extend_from_slice(input);
            let address = original.as_ptr();
            let capacity = original.capacity();
            let prefix = original.split_to(6);
            assert_ne!(original.as_ptr(), address);
            drop(prefix);
            assert_reuses_allocation(original, input, address, capacity);
        }
    }

    #[test]
    fn pointer_helpers_use_the_concrete_type_through_trait_objects() {
        macro_rules! check {
            ($pointer:ident, $convert:ident) => {
                let mut bytes = Vec::with_capacity(32);
                bytes.extend_from_slice("étext".as_bytes());
                let address = bytes.as_ptr();
                let capacity = bytes.capacity();
                let storage: $pointer<dyn StringBytes> = $pointer::new(bytes);

                // Exercise the pointer methods directly through the trait object.
                let text = storage.$convert().unwrap();
                assert_eq!(text, "étext");
                assert_eq!(text.as_ptr(), address);
                assert_eq!(text.capacity(), capacity);

                let bytes = vec![b'a', 0xff];
                let address = bytes.as_ptr();
                let storage: $pointer<dyn StringBytes> = $pointer::new(bytes);
                let error = storage.$convert().unwrap_err();
                assert_eq!(error.utf8_error().valid_up_to(), 1);
                let recovered = error.into_bytes();
                assert_eq!(recovered, b"a\xff");
                assert_eq!(recovered.as_ptr(), address);
            };
        }

        check!(Box, box_into_string);
        check!(Rc, rc_into_string);
        check!(Arc, arc_into_string);

        let rc: Rc<dyn StringBytes> = Rc::new(String::from("text"));
        let text = Rc::clone(&rc).rc_into_string().unwrap();
        assert_eq!(text, "text");
        assert_eq!(rc.as_bytes(), b"text");
        assert_ne!(text.as_ptr(), rc.as_bytes().as_ptr());

        let arc: Arc<dyn StringBytes + Send + Sync> = Arc::new(String::from("text"));
        let text = Arc::clone(&arc).arc_into_string().unwrap();
        assert_eq!(text, "text");
        assert_eq!(arc.as_bytes(), b"text");
        assert_ne!(text.as_ptr(), arc.as_bytes().as_ptr());
    }

    #[test]
    fn try_into_string_from_references_preserves_the_source() {
        let mut source = String::from("étext");
        let copy = source.try_to_string().unwrap();
        assert_eq!(copy, source);
        assert_ne!(copy.as_ptr(), source.as_ptr());

        let copy = (&source).try_into_string().unwrap();
        assert_eq!(copy, source);
        assert_ne!(copy.as_ptr(), source.as_ptr());

        let copy = (&mut source).try_into_string().unwrap();
        assert_eq!(copy, source);
        assert_ne!(copy.as_ptr(), source.as_ptr());

        // On failure, the error owns a copy and the source remains usable.
        let mut source = vec![b'a', 0xff];
        let error = source.try_to_string().unwrap_err();
        assert_eq!(error.utf8_error().valid_up_to(), 1);
        assert_eq!(error.as_bytes(), source);
        assert_ne!(error.as_bytes().as_ptr(), source.as_ptr());

        let error = (&source).try_into_string().unwrap_err();
        assert_eq!(error.utf8_error().valid_up_to(), 1);
        assert_eq!(error.as_bytes(), source);
        assert_ne!(error.as_bytes().as_ptr(), source.as_ptr());

        let error = (&mut source).try_into_string().unwrap_err();
        assert_eq!(error.utf8_error().valid_up_to(), 1);
        assert_eq!(error.as_bytes(), source);
        assert_ne!(error.as_bytes().as_ptr(), source.as_ptr());
    }

    #[test]
    fn try_into_string_error_preserves_owned_bytes() {
        fn check_shared<T: StringBytes + Clone>(storage: T, expected: &[u8]) {
            // Keep another owner alive to force the error to hold a copy.
            let address = storage.as_bytes().as_ptr();
            let error = storage.clone().try_into_string().unwrap_err();
            assert_eq!(error.as_bytes(), expected);
            assert_ne!(error.as_bytes().as_ptr(), address);
            assert_eq!(storage.as_bytes(), expected);
            assert_eq!(storage.as_bytes().as_ptr(), address);
        }

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

            // Cover both sized contents and slices behind shared pointers.
            check_shared(Rc::new(Vec::from(input)), input);
            check_shared(Arc::new(Vec::from(input)), input);
            check_shared(Rc::<[u8]>::from(input), input);
            check_shared(Arc::<[u8]>::from(input), input);
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

    #[test]
    fn try_into_string_copies_borrowed_and_shared_storage() {
        fn check<T: StringBytes>(storage: T) {
            let pointer = storage.as_bytes().as_ptr();
            let text = storage.try_into_string().unwrap();
            assert_eq!(text, "text");
            // Nonempty, borrowed or shared data needs a separate allocation.
            assert_ne!(text.as_ptr(), pointer);
        }

        check("text");
        check(b"text".as_slice());
        check(Cow::Borrowed("text"));
        check(Cow::Borrowed(b"text".as_slice()));

        let rc = Rc::<[u8]>::from(b"text".as_slice());
        check(Rc::clone(&rc));
        assert_eq!(&*rc, b"text");
        check(rc);
        let arc = Arc::<[u8]>::from(b"text".as_slice());
        check(Arc::clone(&arc));
        assert_eq!(&*arc, b"text");
        check(arc);

        let rc = Rc::<str>::from("text");
        check(Rc::clone(&rc));
        assert_eq!(&*rc, "text");
        check(rc);
        let arc = Arc::<str>::from("text");
        check(Arc::clone(&arc));
        assert_eq!(&*arc, "text");
        check(arc);

        // Shared ownership forces a copy even when the contents are sized.
        let rc = Rc::new(Vec::from(b"text".as_slice()));
        check(Rc::clone(&rc));
        assert_eq!(&**rc, b"text");
        let arc = Arc::new(String::from("text"));
        check(Arc::clone(&arc));
        assert_eq!(&**arc, "text");

        // Converting an inline array copies its bytes into the String.
        assert_eq!((*b"text").try_into_string().unwrap(), "text");

        #[cfg(feature = "bytes")]
        {
            check(Bytes::from_static(b"text"));
            check(Bytes::from_owner(Vec::from(b"text".as_slice())));
            let bytes = Bytes::from(Vec::from(b"text".as_slice()));
            check(bytes.clone());
            assert_eq!(&bytes[..], b"text");

            let mut bytes = BytesMut::from(b"texttail".as_slice());
            check(bytes.split_to(4));
            assert_eq!(&bytes[..], b"tail");

            // Reusing a sliced buffer may move its bytes back to the start.
            let bytes = Bytes::from(Vec::from(b"prefixtext".as_slice())).slice(6..);
            assert_eq!(bytes.try_into_string().unwrap(), "text");
        }
    }
}
