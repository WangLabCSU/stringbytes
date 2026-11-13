// These types do not require UTF-8.
mod array;
mod slice;
mod vec;

mod cow_array;
mod cow_slice;
mod cow_vec;

mod c_str;
mod c_string;

mod io_slice;
mod io_slice_mut;

#[cfg(feature = "bytes")]
mod bytes_crate;
