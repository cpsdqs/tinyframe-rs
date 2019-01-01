use crate::number::{BufferReadable, BufferWritable, GenericNumber};
use std::io::{self, Write};
use std::mem;

pub mod checksum;
pub mod number;

pub use self::checksum::*;

/// A TinyFrame message.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Msg<ID, Type> {
    /// The message ID.
    pub id: ID,

    /// Whether or not this message is a response.
    pub is_response: bool,

    /// The message type.
    pub msg_type: Type,

    /// The message data.
    pub data: Vec<u8>,
}

/// A TinyFrame message encoder.
///
/// This will keep track of the next message ID, and contains the Start-of-Frame byte and whether
/// or not this is the master peer.
///
/// See [Message::encode] for actual encoding.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MsgEncoder<ID> {
    next_id: ID,

    /// The start-of-frame byte. If set, will be prepended to every encoded message.
    pub sof_byte: Option<u8>,

    /// Should be set to true if this is the master peer.
    pub is_master: bool,
}

impl<ID> MsgEncoder<ID>
where
    ID: GenericNumber,
{
    /// Creates a new MsgEncoder.
    pub fn new() -> MsgEncoder<ID> {
        MsgEncoder {
            next_id: ID::default(),
            sof_byte: None,
            is_master: false,
        }
    }

    /// Returns the next message ID.
    pub fn next_id(&mut self) -> ID {
        let mut id = self.next_id;
        self.next_id.increment_id();
        if self.is_master {
            id.add_master_peer_bit();
        }
        id
    }

    /// Resets the ID counter.
    pub fn reset(&mut self) {
        self.next_id = ID::default();
    }
}

impl<ID, Type> Msg<ID, Type>
where
    ID: GenericNumber,
    Type: BufferWritable,
{
    fn encode_head<W, Len, Cksum>(
        &mut self,
        out: &mut W,
        encoder: &mut MsgEncoder<ID>,
    ) -> io::Result<()>
    where
        W: Write,
        Len: GenericNumber,
        Cksum: Checksum,
    {
        let mut buf = Vec::with_capacity(512);

        let id = if self.is_response {
            self.id
        } else {
            encoder.next_id()
        };

        self.id = id;

        if let Some(sof_byte) = encoder.sof_byte {
            buf.write_all(&[sof_byte])?;
        }

        id.write_to_buf(&mut buf)?;

        match Len::from_usize(self.data.len()) {
            Some(a) => a.write_to_buf(&mut buf)?,
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "Length type cannot hold message length",
                ))
            }
        }

        self.msg_type.write_to_buf(&mut buf)?;

        Cksum::sum(&buf).write_to_buf(&mut buf)?;

        out.write_all(&buf)?;

        Ok(())
    }

    /// Encodes this message into the given [Write] implementor with the given encoder.
    /// If this message is not a response, a new ID will be assigned by the encoder.
    ///
    /// This will consume the message to prevent potentially unnecessary allocation.
    ///
    /// Note that this uses [Write::write_all] so it may block in some cases.
    ///
    /// # Examples
    /// ```
    /// # use tiny_frame::*;
    /// let msg: Msg<u8, u8> = Msg {
    ///     id: 0, // will be set when encoding since this is not a response
    ///     is_response: false,
    ///     msg_type: 0,
    ///     data: b"hello world!".to_vec(),
    /// };
    /// let mut encoder: MsgEncoder<u8> = MsgEncoder::new();
    /// encoder.sof_byte = Some(1); // set the sof byte to 1
    ///
    /// let mut bytes = Vec::new();
    /// msg.encode::<_, u8, XorSum>(&mut bytes, &mut encoder).expect("Failed to encode");
    ///
    /// assert_eq!(bytes[0], 1); // sof byte is 1
    /// assert_eq!(bytes[1], 0); // message ID is 0 (first message by the encoder)
    /// assert_eq!(bytes[2], "hello world!".len() as u8); // message length
    /// assert_eq!(bytes[3], 0); // message type
    /// // byte 4 is the Xor checksum of the message header
    /// assert_eq!(&bytes[5..17], b"hello world!"); // message content
    /// ```
    pub fn encode<W, Len, Cksum>(
        mut self,
        out: &mut W,
        encoder: &mut MsgEncoder<ID>,
    ) -> io::Result<()>
    where
        W: Write,
        Len: GenericNumber,
        Cksum: Checksum,
    {
        self.encode_head::<W, Len, Cksum>(out, encoder)?;

        Cksum::sum(&self.data).write_to_buf(&mut self.data)?;
        out.write_all(&self.data)?;

        Ok(())
    }

    /// Creates a response message to this message.
    pub fn create_response(&self, ty: Type, data: Vec<u8>) -> Msg<ID, Type> {
        Msg {
            id: self.id,
            is_response: true,
            msg_type: ty,
            data,
        }
    }
}

/// Parser states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ParserState {
    Sof,
    Len,
    HeadCksum,
    ID,
    Type,
    Data,
    DataCksum,
}

/// A TinyFrame message decoder.
pub struct MsgDecoder<ID, Len, Type, Cksum>
where
    Cksum: Checksum,
{
    /// The start-of-frame byte. If set, frames will have to start with this byte.
    pub sof_byte: Option<u8>,
    state: ParserState,
    part_len: usize,
    id: ID,
    len: Len,
    ty: Type,
    cksum: Cksum::Output,
    data: Vec<u8>,
}

impl<ID, Len, Type, Cksum> MsgDecoder<ID, Len, Type, Cksum>
where
    ID: GenericNumber,
    Len: GenericNumber,
    Type: BufferReadable + Default,
    Cksum: Checksum,
{
    /// Creates a new MsgDecoder.
    pub fn new() -> MsgDecoder<ID, Len, Type, Cksum> {
        MsgDecoder {
            sof_byte: None,
            state: ParserState::Sof,
            part_len: 0,
            id: ID::default(),
            len: Len::default(),
            ty: Type::default(),
            cksum: Cksum::Output::default(),
            data: Vec::new(),
        }
    }

    /// Resets this MsgDecoder to initial state.
    pub fn reset(&mut self) {
        self.state = ParserState::Sof;
        self.part_len = 0;
        self.id = ID::default();
        self.len = Len::default();
        self.ty = Type::default();
        self.cksum = Cksum::Output::default();
        self.data = Vec::new();
    }

    fn received_msg(&mut self) -> Msg<ID, Type> {
        Msg {
            id: self.id,
            is_response: false,
            msg_type: mem::replace(&mut self.ty, Type::default()),
            data: mem::replace(&mut self.data, Vec::new()),
        }
    }

    /// Accepts a single byte. Will return the received message if the frame has ended.
    ///
    /// # Examples
    /// ```
    /// # use tiny_frame::*;
    /// // first, encode a message
    /// let mut encoder: MsgEncoder<u8> = MsgEncoder::new();
    /// let mut msg: Msg<u8, u8> = Msg {
    ///     id: 0, // (won’t be changed by the encoder because this is the first message)
    ///     is_response: false,
    ///     msg_type: 0,
    ///     data: b"hello world!".to_vec(),
    /// };
    /// let original_msg = msg.clone(); // used below
    /// let mut bytes = Vec::new();
    /// msg.encode::<_, u8, Crc16Sum>(&mut bytes, &mut encoder).expect("Failed to encode");
    ///
    /// // bytes now contains the encoded message
    ///
    /// // decode the message
    /// let mut decoder: MsgDecoder<u8, u8, u8, Crc16Sum> = MsgDecoder::new();
    /// for byte in bytes.into_iter() {
    ///     if let Some(received) = decoder.accept(byte) {
    ///         // verify that the message was encoded and decoded successfully
    ///         assert_eq!(original_msg, received);
    ///     }
    /// };
    ///
    /// ```
    pub fn accept(&mut self, byte: u8) -> Option<Msg<ID, Type>> {
        if self.sof_byte.is_none() && self.state == ParserState::Sof {
            self.reset();
            self.state = ParserState::ID;
        }

        macro_rules! collect_number {
            (
                dest:$dest:expr,
                type:$type:ident,
                byte:$byte:ident,
                finish:$full:block,
                debug:$debug_name:expr
            ) => {
                $dest = $dest.add_be_byte(byte);
                self.part_len += 1;

                if self.part_len == mem::size_of::<$type>() {
                    self.part_len = 0;
                    $full;
                }
            };
        }

        macro_rules! collect_cksum {
            ($full:block) => {
                self.cksum = self.cksum.add_be_byte(byte);
                if self.part_len == Cksum::Output::size() {
                    self.part_len = 0;
                    $full;
                }
            };
        }

        match self.state {
            ParserState::Sof => {
                if let Some(sof_byte) = self.sof_byte {
                    if byte == sof_byte {
                        self.reset();
                        self.state = ParserState::ID;
                        self.data.push(byte);
                    }
                }
            }
            ParserState::ID => {
                self.data.push(byte);
                collect_number!(
                    dest: self.id,
                    type: ID,
                    byte: byte,
                    finish: {
                        self.state = ParserState::Len;
                    },
                    debug: "ID"
                );
            }
            ParserState::Len => {
                self.data.push(byte);
                collect_number!(
                    dest: self.len,
                    type: Len,
                    byte: byte,
                    finish: {
                        self.state = ParserState::Type;
                    },
                    debug: "length"
                );
            }
            ParserState::Type => {
                self.data.push(byte);
                collect_number!(
                    dest: self.ty,
                    type: Type,
                    byte: byte,
                    finish: {
                        if Cksum::Output::size() == 0 {
                            self.state = ParserState::Data;
                        } else {
                            self.state = ParserState::HeadCksum;
                            self.cksum = Cksum::Output::default();
                        }
                    },
                    debug: "type"
                );
            }
            ParserState::HeadCksum => {
                collect_cksum!({
                    if Cksum::sum(&self.data) != self.cksum {
                        self.reset();
                        return None;
                    }

                    self.data = Vec::new();

                    if self.len == Len::default() {
                        let msg = self.received_msg();
                        self.reset();
                        return Some(msg);
                    }

                    self.state = ParserState::Data;
                });
            }
            ParserState::Data => {
                self.data.push(byte);
                self.part_len += 1;

                if self.len == Len::from_usize(self.part_len).unwrap() {
                    if Cksum::Output::size() == 0 {
                        let msg = self.received_msg();
                        self.reset();
                        return Some(msg);
                    } else {
                        self.state = ParserState::DataCksum;
                        self.part_len = 0;
                        self.cksum = Cksum::Output::default();
                    }
                }
            }
            ParserState::DataCksum => {
                collect_cksum!({
                    let msg = if Cksum::sum(&self.data) == self.cksum {
                        Some(self.received_msg())
                    } else {
                        None
                    };

                    self.reset();
                    return msg;
                });
            }
        }

        None
    }
}
