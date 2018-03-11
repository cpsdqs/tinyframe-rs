use checksum::Checksum;
use std::rc::{Rc, Weak};
use std::{cmp, fmt, mem};
use number::GenericNumber;

/// Peer types.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum Peer {
    Slave = 0,
    Master = 1,
}

impl Default for Peer {
    fn default() -> Peer {
        Peer::Master
    }
}

/// Event listener results.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum ListenerResult {
    /// Will do nothing.
    Next = 0,

    /// Will do nothing.
    Stay = 1,

    /// Will renew an ID listener's timeout.
    Renew = 2,

    /// Will remove the event listener.
    Close = 3,
}

impl Default for ListenerResult {
    fn default() -> ListenerResult {
        ListenerResult::Stay
    }
}

/// A TinyFrame message.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Msg<ID, Type> {
    /// The message ID.
    pub frame_id: ID,

    /// Whether or not this message is a response.
    pub is_response: bool,

    /// The message type.
    pub msg_type: Type,

    /// The message data.
    pub data: Vec<u8>,
}

impl<ID, Type> Msg<ID, Type>
where
    ID: GenericNumber,
    Type: GenericNumber,
{
    /// Creates a new message.
    pub fn new(msg_type: Type, data: &[u8]) -> Msg<ID, Type> {
        Msg {
            frame_id: ID::default(),
            is_response: false,
            msg_type,
            data: data.into(),
        }
    }

    /// Creates a response to this message.
    pub fn create_response(&self, data: &[u8]) -> Msg<ID, Type> {
        Msg {
            frame_id: self.frame_id,
            is_response: true,
            msg_type: self.msg_type,
            data: data.into(),
        }
    }
}

impl<I: GenericNumber, T: GenericNumber> From<Vec<u8>> for Msg<I, T> {
    fn from(data: Vec<u8>) -> Msg<I, T> {
        Msg {
            frame_id: I::default(),
            is_response: false,
            msg_type: T::default(),
            data,
        }
    }
}

impl<'a, I: GenericNumber, T: GenericNumber> From<&'a [u8]> for Msg<I, T> {
    fn from(data: &'a [u8]) -> Msg<I, T> {
        Msg {
            frame_id: I::default(),
            is_response: false,
            msg_type: T::default(),
            data: data.to_vec(),
        }
    }
}

impl<I, T> Into<Vec<u8>> for Msg<I, T> {
    fn into(self) -> Vec<u8> {
        self.data
    }
}

/// An event listener.
pub type Listener<L, I, T> = Fn(&mut TinyFrame<L, I, T>, &Msg<I, T>) -> ListenerResult;

/// Tick type.
pub type Ticks = u32;

/// Parser states.
#[derive(Debug, PartialEq)]
enum ParserState {
    Sof = 0,
    Len,
    HeadCksum,
    ID,
    Type,
    Data,
    DataCksum,
}

/// Listener IDs.
type ListenerID = u64;

/// A wrapper around an ID listener. Dropping this removes the listener.
pub struct IDListener<L, ID, T> {
    /// The listener's unique ID.
    uid: ListenerID,

    /// The message ID for which this listener will be called.
    pub id: ID,

    /// The callback function.
    pub listener: Box<Listener<L, ID, T>>,

    /// The timeout to which this listener can be reset to. If this is `None`,
    /// the ID listener will stay indefinitely.
    pub timeout_max: Option<Ticks>,
}

impl<L, ID, T> IDListener<L, ID, T> {
    /// Renews this listener's timeout.
    pub fn renew(&self, tf: &mut TinyFrame<L, ID, T>) {
        tf.renew_id_listener(self);
    }
}

impl<L, ID: fmt::Debug, T> fmt::Debug for IDListener<L, ID, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "IDListener {{ uid: {:?}, id: {:?}, listener: fn, timeout_max: {:?} }}",
            self.uid, self.id, self.timeout_max
        )
    }
}

/// A wrapper around a type listener. Dropping this removes the listener.
pub struct TypeListener<L, I, Type> {
    /// The message type for which this listener will be called.
    pub msg_type: Type,

    /// The callback function.
    pub listener: Box<Listener<L, I, Type>>,
}

impl<L, I, Type: fmt::Debug> fmt::Debug for TypeListener<L, I, Type> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "TypeListener {{ msg_type: {:?}, listener: fn }}",
            self.msg_type
        )
    }
}

/// A wrapper around a generic listener. Dropping this removes the listener.
pub struct GenericListener<L, I, T> {
    /// The callback function.
    pub listener: Box<Listener<L, I, T>>,
}

impl<L, I, T> fmt::Debug for GenericListener<L, I, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "GenericListener {{ listener: fn }}")
    }
}

/// A weak reference to an `IDListener`.
#[derive(Clone)]
struct IDListenerRef<L, ID, T> {
    uid: ListenerID,
    inner: Weak<IDListener<L, ID, T>>,
}

// TODO: drop listener refs if Rc is dropped
impl<L, ID, T> IDListenerRef<L, ID, T>
where
    ID: PartialEq,
{
    /// Calls the ID listener if it exists and if the ID matches.
    fn call_if_id(&self, id: ID, tf: &mut TinyFrame<L, ID, T>, msg: &Msg<ID, T>) {
        if let Some(listener) = self.inner.upgrade() {
            if listener.id == id {
                match (listener.listener)(tf, msg) {
                    ListenerResult::Renew => {
                        listener.renew(tf);
                    }
                    ListenerResult::Close => {
                        tf.remove_id_listener(self);
                    }
                    _ => (),
                }
            }
        }
    }
}

/// A weak reference to a `TypeListener`.
#[derive(Clone)]
struct TypeListenerRef<L, I, Type> {
    uid: ListenerID,
    inner: Weak<TypeListener<L, I, Type>>,
}

impl<L, I, Type> TypeListenerRef<L, I, Type>
where
    Type: PartialEq,
{
    /// Calls the type listener if it exists and if the type matches.
    fn call_if_type(&self, msg_type: Type, tf: &mut TinyFrame<L, I, Type>, msg: &Msg<I, Type>) {
        if let Some(listener) = self.inner.upgrade() {
            if listener.msg_type == msg_type {
                match (listener.listener)(tf, msg) {
                    ListenerResult::Close => {
                        tf.remove_type_listener(self);
                    }
                    _ => (),
                }
            }
        }
    }
}

/// A weak reference to a `GenericListener`.
#[derive(Clone)]
struct GenericListenerRef<L, I, T> {
    uid: ListenerID,
    inner: Weak<GenericListener<L, I, T>>,
}

impl<L, I, T> GenericListenerRef<L, I, T> {
    /// Calls the generic listener if it exists.
    fn call(&self, tf: &mut TinyFrame<L, I, T>, msg: &Msg<I, T>) {
        if let Some(listener) = self.inner.upgrade() {
            match (listener.listener)(tf, msg) {
                ListenerResult::Close => {
                    tf.remove_generic_listener(self);
                }
                _ => (),
            }
        }
    }
}

/// Errors that can occur when sending a message.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum SendError {
    /// The message data is too long
    TooLong,

    /// The `write` function is not implemented
    NoWrite,
}

impl fmt::Display for SendError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            SendError::TooLong => write!(f, "The message data is too long"),
            SendError::NoWrite => write!(f, "The write function is not set"),
        }
    }
}

/// A TinyFrame instance.
///
/// `Len` is the length field type, `ID` is the ID field type, and `Type` is the
/// type field type. You may use custom types, but in general you should pick
/// from `u8`, `u16`, `u32`, and `u64`.
///
/// You should probably not change any of the configuration fields while the
/// TinyFrame instance is reading or writing a message.
///
/// You should ensure that the `Len` type is sufficiently large to handle all
/// messages you might send or receive. If not, the method will return a
/// [`SendError`](enum.SendError.html).
///
/// Also note that message IDs wrap around, so if you choose a small type (e.g.
/// `u8`) and exchange lots of messages, you might encounter duplicates. Because
/// the most significant bit is reserved for the peer bit, you only have half
/// as many unique IDs available as you would with all bits.
pub struct TinyFrame<Len, ID, Type> {
    /// The peer bit.
    pub peer_bit: Peer,

    /// The next frame ID.
    next_id: ID,

    /// The next listener ID.
    next_listener_id: ListenerID,

    /// The parser state.
    state: ParserState,

    /// The number of ticks since the last input.
    parser_timeout_ticks: Ticks,

    /// The parser timeout after which the parser will be reset.
    pub parser_timeout: Option<Ticks>,

    /// The current length of the current section that is being parsed.
    part_len: usize,

    /// The message ID of the message being parsed.
    id: ID,

    /// The payload length of the message being parsed.
    len: Len,

    /// The message type of the message being parsed.
    recv_type: Type,

    /// The checksum of the current section of the message being parsed.
    recv_cksum: u32,

    /// The current message payload.
    data: Vec<u8>,

    /// The optional start-of-header byte.
    pub sof_byte: Option<u8>,

    /// The chunk size. 1024 by default.
    ///
    /// `write` will be called multiple times consecutively for messages
    /// larger than this size.
    pub chunk_size: usize,

    /// The checksum type. Xor by default.
    pub cksum: Checksum,

    id_listeners: Vec<(IDListenerRef<Len, ID, Type>, Option<Ticks>)>,
    type_listeners: Vec<TypeListenerRef<Len, ID, Type>>,
    generic_listeners: Vec<GenericListenerRef<Len, ID, Type>>,

    /// A function called every time something is written. This must be
    /// implemented.
    pub write: Option<Box<Fn(&mut TinyFrame<Len, ID, Type>, &[u8])>>,

    /// A function called before writing, for claiming the TX interface.
    pub claim_tx: Option<Box<Fn(&TinyFrame<Len, ID, Type>)>>,

    /// A function called after writing, for releasing the TX interface.
    pub release_tx: Option<Box<Fn(&TinyFrame<Len, ID, Type>)>>,
}

// TODO: see if more methods can be moved out of this very strict Len/ID/Type impl
impl<Len, ID, Type> TinyFrame<Len, ID, Type>
where
    Len: GenericNumber,
    ID: GenericNumber,
    Type: GenericNumber,
{
    /// Creates a new TinyFrame with the specified peer bit.
    pub fn new(peer_bit: Peer) -> TinyFrame<Len, ID, Type> {
        TinyFrame {
            peer_bit,
            next_id: ID::default(),
            next_listener_id: 0,
            state: ParserState::Sof,
            parser_timeout_ticks: 0,
            parser_timeout: None,
            part_len: 0,
            id: ID::default(),
            len: Len::default(),
            recv_type: Type::default(),
            recv_cksum: 0,
            data: Vec::new(),
            sof_byte: None,
            chunk_size: 1024,
            cksum: Checksum::Xor,
            id_listeners: Vec::new(),
            type_listeners: Vec::new(),
            generic_listeners: Vec::new(),
            write: None,
            claim_tx: None,
            release_tx: None,
        }
    }

    /// Resets the parser.
    pub fn reset_parser(&mut self) {
        self.state = ParserState::Sof;
    }

    /// Returns the next frame ID.
    fn next_id(&mut self) -> ID {
        let id = self.next_id;
        self.next_id.increment_id();
        id
    }

    /// Returns the next listener ID.
    fn next_listener_id(&mut self) -> ListenerID {
        let id = self.next_listener_id;
        self.next_listener_id += 1;
        id
    }

    /// Adds an ID listener.
    ///
    /// The listener will be called if a message with the specified ID is
    /// received. If `timeout` is not `None`, the listener will expire after the
    /// specified number of ticks.
    ///
    /// Note that if the returned IDListener is dropped, the listener is too.
    pub fn add_id_listener(
        &mut self,
        id: ID,
        cb: Box<Listener<Len, ID, Type>>,
        timeout: Option<Ticks>,
    ) -> Rc<IDListener<Len, ID, Type>> {
        let listener = Rc::new(IDListener {
            uid: self.next_listener_id(),
            id,
            listener: cb,
            timeout_max: timeout,
        });

        self.id_listeners.push((
            IDListenerRef {
                uid: listener.uid,
                inner: Rc::downgrade(&listener),
            },
            timeout,
        ));

        listener
    }

    /// Adds a type listener.
    ///
    /// The listener will be called if a message with the specified type is
    /// received.
    ///
    /// Note that if the returned TypeListener is dropped, the listener is too.
    pub fn add_type_listener(
        &mut self,
        msg_type: Type,
        cb: Box<Listener<Len, ID, Type>>,
    ) -> Rc<TypeListener<Len, ID, Type>> {
        let listener = Rc::new(TypeListener {
            msg_type,
            listener: cb,
        });

        let uid = self.next_listener_id();

        self.type_listeners.push(TypeListenerRef {
            uid,
            inner: Rc::downgrade(&listener),
        });

        listener
    }

    /// Adds a generic listener.
    ///
    /// The listener will be called every time a message is received.
    ///
    /// Note that if the returned GenericListener is dropped, the listener is
    /// too.
    pub fn add_generic_listener(
        &mut self,
        cb: Box<Listener<Len, ID, Type>>,
    ) -> Rc<GenericListener<Len, ID, Type>> {
        let listener = Rc::new(GenericListener { listener: cb });

        let uid = self.next_listener_id();

        self.generic_listeners.push(GenericListenerRef {
            uid,
            inner: Rc::downgrade(&listener),
        });

        listener
    }

    /// Composes a message header.
    ///
    /// # Errors
    /// This method will error if the message length is too large for the length
    /// type.
    fn compose_head(&mut self, msg: &mut Msg<ID, Type>) -> Result<Vec<u8>, SendError> {
        let mut id = if msg.is_response {
            msg.frame_id.clone()
        } else {
            self.next_id()
        };

        if self.peer_bit == Peer::Master {
            id.add_master_peer_bit()
        }

        msg.frame_id = id;

        let mut buf = Vec::with_capacity(
            1 + mem::size_of::<ID>() + mem::size_of::<Len>() + mem::size_of::<Type>(),
        );

        if let Some(sof_byte) = self.sof_byte {
            buf.push(sof_byte);
        }

        id.write_to_buf(&mut buf);
        match Len::from_usize(msg.data.len()) {
            Some(a) => a,
            None => return Err(SendError::TooLong),
        }.write_to_buf(&mut buf);
        msg.msg_type.write_to_buf(&mut buf);

        self.cksum.append_sum(&mut buf);

        Ok(buf)
    }

    /// Sends a frame.
    ///
    /// If `msg.is_response` is true, the message's frame ID will not be
    /// changed.
    ///
    /// # Errors
    /// This method will error if
    ///
    /// - the message length is too large for the length type
    /// - [`write`](#structfield.write) is `None`
    fn send_frame(
        &mut self,
        mut msg: Msg<ID, Type>,
        listener: Option<Box<Listener<Len, ID, Type>>>,
        timeout: Option<Ticks>,
    ) -> Result<Option<Rc<IDListener<Len, ID, Type>>>, SendError> {
        if let Some(ref claim_tx) = self.claim_tx {
            claim_tx(self);
        }

        let mut message = match self.compose_head(&mut msg) {
            Ok(head) => head,
            Err(err) => return Err(err),
        };

        let listener = if let Some(listener) = listener {
            Some(self.add_id_listener(msg.frame_id, listener, timeout))
        } else {
            None
        };

        // TODO: don't clone msg data
        let mut body_buf = msg.data.clone();

        if !body_buf.is_empty() {
            self.cksum.append_sum(&mut body_buf);
        }

        message.append(&mut body_buf);

        let mut cursor = 0;
        let message_len = message.len();

        let mut local_write = None;

        // swap with None so a mutable TinyFrame can be passed to write
        mem::swap(&mut self.write, &mut &mut local_write);

        {
            let write = match local_write {
                Some(ref write) => write,
                None => return Err(SendError::NoWrite),
            };

            while cursor < message_len {
                let chunk_size = cmp::min(message_len - cursor, self.chunk_size);

                write(self, &message[cursor..cursor + chunk_size]);
                cursor += chunk_size;
            }
        }

        // swap back
        mem::swap(&mut self.write, &mut &mut local_write);

        if let Some(ref release_tx) = self.release_tx {
            release_tx(self);
        }

        Ok(listener)
    }

    /// Sends a message.
    ///
    /// If `msg.is_response` is true, the message's frame ID will not be
    /// changed.
    ///
    /// # Errors
    /// This method will error if
    ///
    /// - the message length is too large for the length type
    /// - [`write`](#structfield.write) is `None`
    pub fn send(&mut self, msg: Msg<ID, Type>) -> Result<(), SendError> {
        match self.send_frame(msg, None, None) {
            Ok(_) => Ok(()),
            Err(err) => Err(err),
        }
    }

    /// Sends a message and binds an ID listener to listen for the response.
    ///
    /// Note that if the returned IDListener is dropped, the listener is too.
    ///
    /// # Errors
    /// This method will error if
    ///
    /// - the message length is too large for the length type
    /// - [`write`](#structfield.write) is `None`
    pub fn query(
        &mut self,
        msg: Msg<ID, Type>,
        listener: Box<Listener<Len, ID, Type>>,
        timeout: Option<Ticks>,
    ) -> Result<Rc<IDListener<Len, ID, Type>>, SendError> {
        match self.send_frame(msg, Some(listener), timeout) {
            Ok(msg) => Ok(msg.unwrap()),
            Err(err) => Err(err),
        }
    }

    /// Sends a response.
    ///
    /// This will set `msg.is_response` to true before sending the message.
    ///
    /// # Errors
    /// This method will error if
    ///
    /// - the message length is too large for the length type
    /// - [`write`](#structfield.write) is `None`
    pub fn respond(&mut self, mut msg: Msg<ID, Type>) -> Result<(), SendError> {
        msg.is_response = true;
        self.send(msg)
    }

    /// Reads a buffer. This is just a small wrapper for `accept_byte`.
    pub fn accept(&mut self, buffer: &[u8]) {
        for b in buffer {
            self.accept_byte(*b);
        }
    }

    /// Reads one byte.
    pub fn accept_byte(&mut self, byte: u8) {
        if let Some(parser_timeout) = self.parser_timeout {
            if self.parser_timeout_ticks > parser_timeout {
                self.reset_parser();
            }
        }

        self.parser_timeout_ticks = 0;

        macro_rules! begin_frame {
            () => {
                self.state = ParserState::ID;
                self.part_len = 0;
                self.id = ID::default();
                self.len = Len::default();
                self.recv_type = Type::default();
                self.recv_cksum = 0;
                self.data = Vec::new();
            }
        }

        if self.sof_byte.is_none() && self.state == ParserState::Sof {
            begin_frame!();
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
            }
        }

        macro_rules! collect_cksum {
            ($full:block) => {
                if match self.cksum {
                    Checksum::None | Checksum::Xor => {
                        self.recv_cksum = byte as u32;
                        true
                    }
                    Checksum::Crc16 => {
                        self.recv_cksum = self.recv_cksum << 8 | byte as u32;
                        self.part_len == mem::size_of::<u16>()
                    }
                    Checksum::Crc32 => {
                        self.recv_cksum = self.recv_cksum << 8 | byte as u32;
                        self.part_len == mem::size_of::<u32>()
                    }
                } {
                    self.part_len = 0;
                    $full;
                }
            }
        }

        match self.state {
            ParserState::Sof => {
                if let Some(sof_byte) = self.sof_byte {
                    if byte == sof_byte {
                        begin_frame!();
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
                    dest: self.recv_type,
                    type: Type,
                    byte: byte,
                    finish: {
                        if self.cksum == Checksum::None {
                            self.state = ParserState::Data;
                        } else {
                            self.state = ParserState::HeadCksum;
                            self.recv_cksum = 0;
                        }
                    },
                    debug: "type"
                );
            }
            ParserState::HeadCksum => {
                collect_cksum!({
                    if self.cksum.sum(&self.data) != self.recv_cksum {
                        self.reset_parser();
                        return;
                    }

                    self.data = Vec::new();

                    if self.len == Len::default() {
                        self.handle_received();
                        self.reset_parser();
                        return;
                    }

                    self.state = ParserState::Data;
                });
            }
            ParserState::Data => {
                self.data.push(byte);
                self.part_len += 1;

                if self.len == Len::from_usize(self.part_len).unwrap() {
                    if self.cksum == Checksum::None {
                        self.handle_received();
                        self.reset_parser();
                    } else {
                        self.state = ParserState::DataCksum;
                        self.part_len = 0;
                        self.recv_cksum = 0;
                    }
                }
            }
            ParserState::DataCksum => {
                collect_cksum!({
                    if self.cksum.sum(&self.data) == self.recv_cksum {
                        self.handle_received();
                    }

                    self.reset_parser();
                });
            }
        }
    }

    /// Handles a received message.
    fn handle_received(&mut self) {
        let msg = Msg {
            frame_id: self.id,
            is_response: false,
            msg_type: self.recv_type,
            data: self.data.clone(),
        };

        let mut id_listeners = mem::replace(&mut self.id_listeners, Vec::new());
        let mut type_listeners = mem::replace(&mut self.type_listeners, Vec::new());
        let mut generic_listeners = mem::replace(&mut self.generic_listeners, Vec::new());

        for listener in &id_listeners {
            listener.0.call_if_id(msg.frame_id, self, &msg);
        }

        for listener in &type_listeners {
            listener.call_if_type(msg.msg_type, self, &msg);
        }

        for listener in &generic_listeners {
            listener.call(self, &msg);
        }

        id_listeners.append(&mut self.id_listeners);
        type_listeners.append(&mut self.type_listeners);
        generic_listeners.append(&mut self.generic_listeners);

        mem::replace(&mut self.id_listeners, id_listeners);
        mem::replace(&mut self.type_listeners, type_listeners);
        mem::replace(&mut self.generic_listeners, generic_listeners);
    }
}

impl<Len, ID, Type> TinyFrame<Len, ID, Type> {
    /// This function should be called periodically to time-out partial frames
    /// and ID listeners.
    pub fn tick(&mut self) {
        self.parser_timeout_ticks += 1;

        let mut index = 0;
        let mut remove_keys = Vec::new();

        for ref mut value in &mut self.id_listeners {
            if let Some(timeout_value) = value.1 {
                if timeout_value == 1 {
                    remove_keys.push(index);
                } else {
                    value.1 = Some(timeout_value - 1);
                }
            }

            index += 1;
        }

        for key in remove_keys {
            self.id_listeners.remove(key);
        }
    }

    /// Renews an ID listener.
    fn renew_id_listener(&mut self, listener: &IDListener<Len, ID, Type>) {
        if let Some(timeout_max) = listener.timeout_max {
            if let Some(index) = self.id_listeners
                .iter()
                .position(|x| x.0.uid == listener.uid)
            {
                self.id_listeners[index].1 = Some(timeout_max);
            }
        }
    }

    /// Removes an ID listener.
    fn remove_id_listener(&mut self, listener: &IDListenerRef<Len, ID, Type>) {
        if let Some(index) = self.id_listeners
            .iter()
            .position(|x| x.0.uid == listener.uid)
        {
            self.id_listeners.remove(index);
        }
    }

    /// Removes a type listener.
    fn remove_type_listener(&mut self, listener: &TypeListenerRef<Len, ID, Type>) {
        if let Some(index) = self.type_listeners
            .iter()
            .position(|x| x.uid == listener.uid)
        {
            self.type_listeners.remove(index);
        }
    }

    /// Removes a generic listener.
    fn remove_generic_listener(&mut self, listener: &GenericListenerRef<Len, ID, Type>) {
        if let Some(index) = self.generic_listeners
            .iter()
            .position(|x| x.uid == listener.uid)
        {
            self.generic_listeners.remove(index);
        }
    }
}

impl<L, I, T> fmt::Debug for TinyFrame<L, I, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "TinyFrame")
    }
}

impl<L, I, T> fmt::Write for TinyFrame<L, I, T>
where
    L: GenericNumber,
    I: GenericNumber,
    T: GenericNumber,
{
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.accept(s.as_bytes());
        Ok(())
    }
}
