use std::io::{self, Write};
use std::mem;

/// A number type that can be written to a buffer using big endian encoding.
pub trait BufferWritable {
    /// Appends the big endian byte value to the buffer.
    ///
    /// # Examples
    /// ```
    /// # use tiny_frame::number::BufferWritable;
    /// let mut buffer: Vec<u8> = Vec::new();
    /// 12u8.write_to_buf(&mut buffer); // 12
    /// 280u16.write_to_buf(&mut buffer); // 1 * 256 + 24
    /// assert_eq!(buffer, vec![12, 1, 24]);
    /// ```
    fn write_to_buf<W: Write>(&self, buf: &mut W) -> io::Result<()>;
}

macro_rules! buffer_writable_impl {
    ($type:ty) => {
        impl BufferWritable for $type {
            fn write_to_buf<W: Write>(&self, buf: &mut W) -> io::Result<()> {
                let mut bytes = [0; mem::size_of::<$type>()];
                unsafe {
                    *(bytes.as_mut_ptr() as *mut $type) = self.to_be();
                }
                buf.write_all(&bytes)
            }
        }
    };
}

impl BufferWritable for () {
    fn write_to_buf<W: Write>(&self, _: &mut W) -> io::Result<()> {
        Ok(())
    }
}

buffer_writable_impl!(u8);
buffer_writable_impl!(u16);
buffer_writable_impl!(u32);
buffer_writable_impl!(u64);
buffer_writable_impl!(u128);
buffer_writable_impl!(i8);
buffer_writable_impl!(i16);
buffer_writable_impl!(i32);
buffer_writable_impl!(i64);
buffer_writable_impl!(i128);

/// A number type that can be read from a buffer using big endian encoding.
pub trait BufferReadable {
    /// Appends one byte to the number's binary representation.
    fn add_be_byte(&self, byte: u8) -> Self;

    /// Returns the size of this type.
    fn size() -> usize;
}

macro_rules! buffer_readable_byte_impl {
    ($type:ty) => {
        impl BufferReadable for $type {
            fn add_be_byte(&self, byte: u8) -> Self {
                byte as $type
            }
            fn size() -> usize {
                mem::size_of::<$type>()
            }
        }
    };
}

impl BufferReadable for () {
    fn add_be_byte(&self, _: u8) {}
    fn size() -> usize {
        0
    }
}

buffer_readable_byte_impl!(u8);
buffer_readable_byte_impl!(i8);

macro_rules! buffer_readable_impl {
    ($type:ty) => {
        impl BufferReadable for $type {
            fn add_be_byte(&self, byte: u8) -> Self {
                (*self << 8) | byte as $type
            }

            fn size() -> usize {
                mem::size_of::<$type>()
            }
        }
    };
}

buffer_readable_impl!(u16);
buffer_readable_impl!(u32);
buffer_readable_impl!(u64);
buffer_readable_impl!(u128);
buffer_readable_impl!(i16);
buffer_readable_impl!(i32);
buffer_readable_impl!(i64);
buffer_readable_impl!(i128);

/// A generic number trait.
pub trait GenericNumber: BufferReadable + BufferWritable + Default + Copy + PartialEq {
    /// Increments this ID.
    fn increment_id(&mut self);

    /// Adds the master peer bit to this ID.
    fn add_master_peer_bit(&mut self);

    /// Converts a `usize` to this length type.
    fn from_usize(size: usize) -> Option<Self>;
}

macro_rules! generic_number_impl {
    ($type:ty, $type2:ident) => {
        impl GenericNumber for $type {
            fn increment_id(&mut self) {
                *self = self.wrapping_add(1) & ($type2::max_value() >> 1);
            }
            fn add_master_peer_bit(&mut self) {
                *self |= 1 << mem::size_of::<$type>() * 8 - 1;
            }
            fn from_usize(size: usize) -> Option<Self> {
                if size > $type2::max_value() as usize {
                    None
                } else {
                    Some(size as $type)
                }
            }
        }
    };
}

impl GenericNumber for () {
    fn increment_id(&mut self) {}
    fn add_master_peer_bit(&mut self) {}
    fn from_usize(_: usize) -> Option<()> {
        None
    }
}

generic_number_impl!(u8, u8);
generic_number_impl!(i8, i8);
generic_number_impl!(u16, u16);
generic_number_impl!(i16, i16);
generic_number_impl!(u32, u32);
generic_number_impl!(i32, i32);
generic_number_impl!(u64, u64);
generic_number_impl!(i64, i64);
