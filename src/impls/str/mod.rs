// These storage types guarantee UTF-8 and override string conversions to
// skip validation. Their pointer helpers delegate to these overrides,
// preserving the same behavior through smart pointers.
// into_string_lossy uses to_string_lossy's fast path; the unchecked defaults
// go through byte conversions and never validate UTF-8.

mod slice;
mod string;

mod cow_str;
mod cow_string;
