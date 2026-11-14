// These storage types guarantee UTF-8 and override string conversions to
// skip validation. Pointer conversions use the borrowed string override before
// taking ownership, so they skip validation too.
// into_string_lossy uses to_string_lossy's fast path; the unchecked defaults
// go through byte conversions and never validate UTF-8.

mod slice;
mod string;

mod cow_str;
mod cow_string;
