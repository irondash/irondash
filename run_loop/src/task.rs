use std::{
    cell::{RefCell, UnsafeCell},
    marker::PhantomData,
    sync::Arc,
    task::Poll,
};

use futures::{
    future::LocalBoxFuture,
    task::{waker_ref, ArcWake},
    Future, FutureExt,
};

use crate::RunLoopSender;

pub struct Task<T> {
    sender: RunLoopSender,
    future: UnsafeCell<LocalBoxFuture<'static, T>>,
    value: RefCell<Option<T>>,
    waker: RefCell<Option<std::task::Waker>>,
}

// Tasks can only be spawned on run loop thread and will only be executed
// on run loop thread. ArcWake however doesn't know this.
unsafe impl<T> Send for Task<T> {}
unsafe impl<T> Sync for Task<T> {}

impl<T: 'static> Task<T> {
    pub(crate) fn new<F>(sender: RunLoopSender, future: F) -> Self
    where
        F: Future<Output = T> + 'static,
        T: 'static,
    {
        let future = future.boxed_local();
        Self {
            sender,
            future: UnsafeCell::new(future),
            value: RefCell::new(None),
            waker: RefCell::new(None),
        }
    }

    fn poll(self: &std::sync::Arc<Self>) -> Poll<T> {
        let waker = waker_ref(self).clone();
        let context = &mut core::task::Context::from_waker(&waker);
        unsafe {
            let future = &mut *self.future.get();
            future.as_mut().poll(context)
        }
    }
}

impl<T: 'static> ArcWake for Task<T> {
    fn wake_by_ref(arc_self: &std::sync::Arc<Self>) {
        let arc_self = arc_self.clone();
        let sender = arc_self.sender.clone();
        sender.send(move || {
            if arc_self.value.borrow().is_none() {
                if let Poll::Ready(value) = arc_self.poll() {
                    *arc_self.value.borrow_mut() = Some(value);
                }
            }
            if arc_self.value.borrow().is_some() {
                if let Some(waker) = arc_self.waker.borrow_mut().take() {
                    waker.wake();
                }
            }
        });
    }
}

pub struct JoinHandle<T> {
    task: Arc<Task<T>>,
    // Task has unsafe `Send` and `Sync`, but that is only because we know
    // it will not be polled from another thread. This is to ensure that
    // JoinHandle is neither Send nor Sync.
    _data: PhantomData<*const ()>,
}

impl<T: 'static> JoinHandle<T> {
    pub(crate) fn new(task: Arc<Task<T>>) -> Self {
        Self {
            task,
            _data: PhantomData,
        }
    }
}

impl<T: 'static> Future for JoinHandle<T> {
    type Output = T;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let value = self.task.value.borrow_mut().take();
        match value {
            Some(value) => Poll::Ready(value),
            None => {
                self.task
                    .waker
                    .borrow_mut()
                    .get_or_insert_with(|| cx.waker().clone());
                Poll::Pending
            }
        }
    }
}
