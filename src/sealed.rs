/// Keeps StringBytes implementations within this crate and excludes trait objects.
///
/// StringBytes cannot be used as a trait object:
///
/// ```compile_fail,E0038
/// use stringbytes::StringBytes;
///
/// let bytes = Vec::<u8>::new();
/// let _: &dyn StringBytes = &bytes;
/// ```
///
/// A downstream subtrait cannot introduce a trait-object implementation either:
///
/// ```compile_fail,E0038
/// use stringbytes::StringBytes;
///
/// trait LocalStorage: StringBytes {}
/// impl LocalStorage for Vec<u8> {}
///
/// let bytes = Vec::<u8>::new();
/// let _: &dyn LocalStorage = &bytes;
/// ```
pub trait Sealed {
    // An associated constant prevents dyn compatibility without requiring Sized.
    // This keeps str, [u8], and CStr supported while excluding downstream trait
    // objects whose AsRef, AsMut, or Deref implementations we cannot control.
    const NON_DYN: () = ();
}
