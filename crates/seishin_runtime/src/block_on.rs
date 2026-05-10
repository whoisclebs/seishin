use std::{
    future::Future,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
    thread::{self, Thread},
};

pub(crate) fn block_on<F: Future>(future: F) -> F::Output {
    let notifier = Arc::new(ThreadNotifier {
        thread: thread::current(),
        notified: AtomicBool::new(false),
    });
    let waker = unsafe { Waker::from_raw(raw_waker(Arc::clone(&notifier))) };
    let mut context = Context::from_waker(&waker);
    let mut future = Box::pin(future);

    loop {
        match Pin::as_mut(&mut future).poll(&mut context) {
            Poll::Ready(output) => return output,
            Poll::Pending => {
                while !notifier.notified.swap(false, Ordering::AcqRel) {
                    thread::park();
                }
            }
        }
    }
}

struct ThreadNotifier {
    thread: Thread,
    notified: AtomicBool,
}

unsafe fn raw_waker(notifier: Arc<ThreadNotifier>) -> RawWaker {
    RawWaker::new(Arc::into_raw(notifier).cast(), &VTABLE)
}

unsafe fn clone_waker(data: *const ()) -> RawWaker {
    let notifier = Arc::from_raw(data.cast::<ThreadNotifier>());
    let cloned = Arc::clone(&notifier);
    let _ = Arc::into_raw(notifier);
    raw_waker(cloned)
}

unsafe fn wake(data: *const ()) {
    let notifier = Arc::from_raw(data.cast::<ThreadNotifier>());
    notifier.notified.store(true, Ordering::Release);
    notifier.thread.unpark();
}

unsafe fn wake_by_ref(data: *const ()) {
    let notifier = Arc::from_raw(data.cast::<ThreadNotifier>());
    notifier.notified.store(true, Ordering::Release);
    notifier.thread.unpark();
    let _ = Arc::into_raw(notifier);
}

unsafe fn drop_waker(data: *const ()) {
    drop(Arc::from_raw(data.cast::<ThreadNotifier>()));
}

static VTABLE: RawWakerVTable = RawWakerVTable::new(clone_waker, wake, wake_by_ref, drop_waker);
