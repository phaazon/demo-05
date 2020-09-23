//! The System crate.
//!
//! This crate provides a very simple mechanism to create _systems_, which can send messages to each other, spawn new
//! systems and perform local state mutation and I/O.

use std::fmt;
use std::sync::mpsc;

use rand::{thread_rng, Rng as _};

pub mod resource;

/// Systems.
///
/// A _system_ is a special kind of object that has an _address_ ([`Addr`]) that other systems can use to send messages
/// to.
///
/// A _message_ can be anything, but most of the time, systems will expect a protocol to be implemented when sending
/// messages to efficiently _move_ messages without having to serialize / deserialize them.
pub trait System<M, E = M>
where
  E: Clone,
{
  /// Get the address of this system.
  fn system_addr(&self) -> Addr<M>;

  /// Send a message to myself.
  fn send_msg_self(&self, msg: M) -> Result<(), SystemError> {
    self.system_addr().send_msg(msg)
  }

  /// Run the system and return its [`Addr`] so that other systems can use it.
  fn startup(self);

  /// Emit an event.
  ///
  /// The difference between [`System::send_msg`] and [`System::emit_event`] is that the former requires an explicit
  /// address, while the second will send to all “subscribers”.
  fn publish(&self, event: E);

  /// Subscribe a system that will receive events.
  fn subscribe(&mut self, addr: Addr<E>);
}

/// UID of a system.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SystemUID(u16);

impl SystemUID {
  pub fn new() -> Self {
    SystemUID(thread_rng().gen())
  }
}

impl fmt::Display for SystemUID {
  fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
    self.0.fmt(f)
  }
}

/// An address of a [`System`] that allows sending messages of type `T`.
#[derive(Debug)]
pub struct Addr<T> {
  sender: mpsc::Sender<T>,
}

impl<T> Addr<T> {
  pub fn send_msg(&self, msg: T) -> Result<(), SystemError> {
    self.sender.send(msg).map_err(|_| SystemError::CannotSend)
  }
}

impl<T> Clone for Addr<T> {
  fn clone(&self) -> Self {
    Addr {
      sender: self.sender.clone(),
    }
  }
}

/// Errors that might occur with [`System`] operations.
#[derive(Debug, Eq, Hash, PartialEq)]
pub enum SystemError {
  /// Cannot send a message.
  CannotSend,
}

impl fmt::Display for SystemError {
  fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
    match *self {
      SystemError::CannotSend => write!(f, "cannot send message"),
    }
  }
}

/// A message queue, which systems can use to see what messages they have received.
#[derive(Debug)]
pub struct MsgQueue<T> {
  receiver: mpsc::Receiver<T>,
}

impl<T> MsgQueue<T> {
  /// Wait until a message gets available.
  pub fn recv(&self) -> Option<T> {
    self.receiver.recv().ok()
  }
}

/// Default implementation of a system initialization procedure.
///
/// When creating a [`System`], the first thing one wants to do is to create all the required state to be able to:
///
/// - Look at received messages.
/// - Present oneself to others by handing out an [`Addr`].
///
/// This method is supposed to be used by systems’ implementations to ease creating the internal state of a system.
pub fn system_init<T>() -> (Addr<T>, MsgQueue<T>) {
  let (sender, receiver) = mpsc::channel();

  (Addr { sender }, MsgQueue { receiver })
}
