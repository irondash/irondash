use std::{fmt::Display, thread};

use crate::{get_system_thread_id, RunLoopSender, SystemThreadId};

// Thread bound capsule; Allows retrieving the value only on the thread
// where it was stored.
pub struct Capsule<T>
where
    T: 'static,
{
    value: Option<T>,
    thread_id: SystemThreadId,
    sender: Option<RunLoopSender>,
}

#[derive(Debug)]
pub enum CapsuleError {
    CapsuleEmpty,
    WrongThread,
}

impl Display for CapsuleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CapsuleError::CapsuleEmpty => write!(f, "capsule is empty"),
            CapsuleError::WrongThread => write!(f, "capsule retrieved on wrong thread"),
        }
    }
}

impl std::error::Error for CapsuleError {}

#[allow(dead_code)]
impl<T> Capsule<T>
where
    T: 'static,
{
    // Creates new capsule; If the value is not taken out of capsule, the
    // capsule must be dropped on same thread as it was created, otherwise
    // it will panic
    pub fn new(value: T) -> Self {
        Self {
            value: Some(value),
            thread_id: get_system_thread_id(),
            sender: None,
        }
    }

    // Creates new capsule, If the value is not taken out of capsule and the
    // capsule is dropped on different thread than where it was created, it will
    // be sent to the sender and dropped on the run loop thread
    pub fn new_with_sender(value: T, sender: RunLoopSender) -> Self {
        Self {
            value: Some(value),
            thread_id: get_system_thread_id(),
            sender: Some(sender),
        }
    }

    pub fn get_ref(&self) -> Result<&T, CapsuleError> {
        if self.thread_id == get_system_thread_id() {
            self.value.as_ref().ok_or(CapsuleError::CapsuleEmpty)
        } else {
            Err(CapsuleError::WrongThread)
        }
    }

    pub fn get_mut(&mut self) -> Result<&mut T, CapsuleError> {
        if self.thread_id == get_system_thread_id() {
            self.value.as_mut().ok_or(CapsuleError::CapsuleEmpty)
        } else {
            Err(CapsuleError::WrongThread)
        }
    }

    pub fn take(&mut self) -> Result<T, CapsuleError> {
        if self.thread_id == get_system_thread_id() {
            self.value.take().ok_or(CapsuleError::CapsuleEmpty)
        } else {
            Err(CapsuleError::WrongThread)
        }
    }
}

impl<T> Drop for Capsule<T> {
    fn drop(&mut self) {
        // we still have value and capsule was dropped in other thread
        if self.value.is_some() && self.thread_id != get_system_thread_id() {
            if let Some(sender) = self.sender.as_ref() {
                let carry = Carry(self.value.take().unwrap());
                let thread_id = self.thread_id;
                sender.send(move || {
                    // make sure that sender sent us back to initial thread
                    if thread_id != get_system_thread_id() {
                        panic!("Capsule was created on different thread than sender target")
                    }
                    let _ = carry;
                });
            } else if !thread::panicking() {
                panic!("Capsule was dropped on wrong thread with data still in it!");
            }
        }
    }
}

impl<T: Clone> Clone for Capsule<T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            thread_id: self.thread_id,
            sender: self.sender.clone(),
        }
    }
}

unsafe impl<T> Send for Capsule<T> {}
unsafe impl<T> Sync for Capsule<T> {}

struct Carry<T>(T);

unsafe impl<T> Send for Carry<T> {}
