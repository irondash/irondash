use crate::platform;

#[derive(Eq, PartialEq, Clone, Copy, Hash, Debug)]
pub struct SystemThreadId(platform::PlatformThreadId);

/// Returns native platform thread identifier. Unlike Rust ThreadId,
/// this does not guarantee that the identifier will not be reused by
/// future threads. However Rust ThreadId can not be queried during thread
/// destruction when the thread_local has already been destroyed (and will
/// panic in that situation).
pub fn get_system_thread_id() -> SystemThreadId {
    SystemThreadId(platform::get_system_thread_id())
}
