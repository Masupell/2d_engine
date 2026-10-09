use std::{future::Future, pin::pin, sync::Arc, task::{Context as TaskContext, Poll, Wake, Waker}, thread::{self, Thread}};

struct ThreadWaker(Thread);

impl Wake for ThreadWaker
{
    fn wake(self: Arc<Self>)
    {
        self.0.unpark();
    }
}

// Native only
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn block_on<F: Future>(future: F) -> F::Output
{
    let mut future = pin!(future);
    let waker = Waker::from(Arc::new(ThreadWaker(thread::current())));
    let mut cx = TaskContext::from_waker(&waker);

    loop
    {
        match future.as_mut().poll(&mut cx)
        {
            Poll::Ready(value) => return value,
            Poll::Pending => thread::park()
        }
    }
}
