use irondash_engine_context::EngineContext;
use once_cell::sync::OnceCell;

use crate::{
    platform::{self, PlatformThreadId},
    Result, RunLoop, RunLoopSender,
};

pub enum MainThreadFacilitator {
    EngineContext,
    Manual {
        thread_id: PlatformThreadId,
        sender: RunLoopSender,
    },
}

static MAIN_THREAD_FACILITATOR: OnceCell<MainThreadFacilitator> = OnceCell::new();

impl MainThreadFacilitator {
    pub fn set_for_current_thread() {
        match MAIN_THREAD_FACILITATOR.try_insert(MainThreadFacilitator::Manual {
            thread_id: platform::get_system_thread_id(),
            sender: RunLoop::current().new_sender(),
        }) {
            Ok(_) => {}
            Err((exiting, _)) => match exiting {
                MainThreadFacilitator::EngineContext => {
                    panic!("RunLoop::set_as_main_thread() was called after other RunLoop methods.");
                }
                MainThreadFacilitator::Manual {
                    thread_id,
                    sender: _,
                } => {
                    if *thread_id != platform::get_system_thread_id() {
                        panic!(
                            "RunLoop::set_as_main_thread() was already called on another thread."
                        );
                    }
                }
            },
        }
    }

    pub fn get() -> &'static Self {
        MAIN_THREAD_FACILITATOR.get_or_init(|| MainThreadFacilitator::EngineContext)
    }

    pub fn is_main_thread(&self) -> Result<bool> {
        match self {
            MainThreadFacilitator::EngineContext => Ok(EngineContext::is_main_thread()?),
            MainThreadFacilitator::Manual {
                thread_id,
                sender: _,
            } => Ok(*thread_id == platform::get_system_thread_id()),
        }
    }

    pub fn perform_on_main_thread(&self, f: impl FnOnce() + Send + 'static) -> Result<()> {
        match self {
            MainThreadFacilitator::EngineContext => Ok(EngineContext::perform_on_main_thread(f)?),
            MainThreadFacilitator::Manual {
                thread_id: _,
                sender,
            } => {
                sender.send(f);
                Ok(())
            }
        }
    }
}
