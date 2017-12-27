//! TinyFrame is a simple library for building and parsing frames to be sent
//! over a serial interface (e.g. UART, telnet etc.).
//!
//! TinyFrame is suitable for a wide range of applications, including
//! inter-microcontroller communication, as a protocol for FTDI-based PC
//! applications or for messaging through UDP packets.
//!
//! Frames can be protected by a checksum (~XOR, CRC16 or CRC32) and contain a
//! unique ID field which can be used for chaining related messages. The highest
//! bit of the generated IDs is different in each peer to avoid collisions. Peers are functionally equivalent and can send messages to each other (the names "master" and "slave" are used only for convenience and have special meaning in the demos).
//!
//! The library lets you register listeners (callback functions) to wait for
//! (1) any frame, (2) a particular frame Type, or (3) a specific message ID.
//! This high-level API lets you easily implement various async communication
//! patterns.
//!
//! TinyFrame is re-entrant and supports creating multiple instances.
//!
//! ## Frame Structure
//!
//! All fields in the message frame have a configurable size, depending on
//! which type you choose.
//!
//! For example, you don't need 4 bytes (`u32`) for the length field if
//! your payloads are 20 bytes long, using a 1-byte field (`u8`) will save
//! 3 bytes. This may be significant if you need high throughput.
//!
//! ```text
//! ,-----+-----+-----+------+------------+- - - -+-------------,
//! | SOF | ID  | LEN | TYPE | HEAD_CKSUM | DATA  | DATA_CKSUM  |
//! | 0-1 | 1-8 | 1-8 | 1-8  | 0-4        | ...   | 0-4         | <- size (bytes)
//! '-----+-----+-----+------+------------+- - - -+-------------'
//!
//! SOF ......... start of frame, usually 0x01 (optional, configurable)
//! ID  ......... the frame ID (MSb is the peer bit)
//! LEN ......... number of data bytes in the frame
//! TYPE ........ message type (used to run Type Listeners, pick any values you like)
//! HEAD_CKSUM .. header checksum
//!
//! DATA ........ LEN bytes of data (can be 0, in which case DATA_CKSUM is omitted as well)
//! DATA_CKSUM .. data checksum
//! ```
//!
//! # Examples
//!
//! ```
//! # use std::mem;
//! # use tiny_frame::{Peer, TinyFrame, ListenerResult, Msg};
//! # fn main() {
//! let mut tf: TinyFrame<u8, u8, u8> = TinyFrame::new(Peer::Master);
//!
//! // Implement the write function
//! tf.write = Some(Box::new(|tf, buf| {
//!     println!("frame: {:?}", buf);
//!
//!     // send the message back
//!     tf.accept(&Vec::from(buf));
//! }));
//!
//! // Listener needs to be kept around such that it isn't dropped
//! let listener = tf.add_generic_listener(Box::new(|_, msg| {
//!     println!("Message received: {}", String::from_utf8_lossy(&msg.data[..]));
//!     ListenerResult::Stay
//! }));
//!
//! // send a message
//! tf.send(Msg::new(b"Hello TinyFrame"));
//! # mem::forget(listener);
//! # }
//! ```

extern crate num;

mod checksum;
mod tiny_frame;

pub use checksum::Checksum;
pub use tiny_frame::*;

#[cfg(test)]
mod tests {
    use std::mem;

    #[test]
    fn basic_test() {
        use super::tiny_frame::{TinyFrame, Msg, Peer, ListenerResult};

        let mut tf: TinyFrame<u8, u8, u8> = TinyFrame::new(Peer::Master);
        tf.write = Some(Box::new(|tf, buf| {
            println!("frame: {:?}", buf);
            tf.accept(&Vec::from(buf));
        }));

        #[allow(non_upper_case_globals)]
        static mut first_msg: bool = true;

        #[allow(non_upper_case_globals)]
        static mut generic_calls: u32 = 0;

        let listener = tf.add_generic_listener(Box::new(|_, msg| {
            println!("Generic listener! Message: {}", String::from_utf8_lossy(&msg.data[..]));

            if unsafe { first_msg } {
                assert_eq!(&msg.data[..], b"Hello TinyFrame");
                unsafe { first_msg = false };
            }

            unsafe { generic_calls += 1 };

            ListenerResult::Stay
        }));

        mem::forget(listener);

        tf.send(Msg::new(b"Hello TinyFrame"));

        #[allow(non_upper_case_globals)]
        static mut query_calls: u32 = 0;

        tf.query(Msg::new(b"Query message"), Box::new(|_, msg| {
            println!("Query result: {}", String::from_utf8_lossy(&msg.data[..]));
            unsafe { query_calls += 1 };
            ListenerResult::Close
        }), None);

        if unsafe { generic_calls } != 2 {
            panic!("Generic listener was called an incorrect number of times: {} (should be {})", unsafe { query_calls }, 2)
        }
        if unsafe { query_calls } != 1 {
            panic!("Query listener was called an incorrect number of times: {} (should be {})", unsafe { query_calls }, 1)
        }
    }
}
