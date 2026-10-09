use std::{collections::VecDeque, sync::{mpsc::{self, TryRecvError}, Arc, Condvar, Mutex}, thread};

type Job = Box<dyn FnOnce() + Send + 'static>;

struct Queue
{
    jobs: VecDeque<Job>,
    idle: usize,
    workers: usize,
    shutdown: bool
}

struct Shared
{
    queue: Mutex<Queue>,
    signal: Condvar, // to wake sleeping workers
    max_workers: usize
}

// Only the ThreadPool handles hold this, workers don't.
// When the last handle is dropped, the workers get told to stop.
struct Owner
{
    shared: Arc<Shared>
}

impl Drop for Owner
{
    // Workers stop after their current job. Not joined: a long job shouldn't block closing the game.
    fn drop(&mut self)
    {
        if let Ok(mut queue) = self.shared.queue.lock()
        {
            queue.shutdown = true;
            queue.jobs.clear();
        }
        self.shared.signal.notify_all();
    }
}

// Cloning give handle to same pool, because of arc
// Max amount of workers is cores-1 (main thread has one core)
#[derive(Clone)]
pub struct ThreadPool
{
    inner: Arc<Owner>
}

impl ThreadPool
{
    pub(crate) fn new() -> Self
    {
        let cores = thread::available_parallelism().map(|n| n.get()).unwrap_or(2);

        let shared = Arc::new(Shared
        {
            queue: Mutex::new(Queue { jobs: VecDeque::new(), idle: 0, workers: 0, shutdown: false }),
            signal: Condvar::new(),
            max_workers: cores.saturating_sub(1).max(1)
        });

        Self { inner: Arc::new(Owner { shared }) }
    }

    // Does stuff on a worker thread
    // has to own everything it uses, no borrowing
    pub fn spawn<T, F>(&self, work: F) -> Task<T>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static
    {
        let (sender, receiver) = mpsc::channel();
        self.execute(Box::new(move || { let _ = sender.send(work()); })); // If work panic, sender is dropped and task reports Failed
        Task { receiver: Some(receiver), failed: false }
    }

    // No result, so something that needs none, like saving a file
    pub fn spawn_detached<F>(&self, work: F)
    where
        F: FnOnce() + Send + 'static
    {
        self.execute(Box::new(work));
    }

    pub fn worker_count(&self) -> usize
    {
        self.inner.shared.queue.lock().map(|queue| queue.workers).unwrap_or(0)
    }

    pub fn max_workers(&self) -> usize
    {
        self.inner.shared.max_workers
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn execute(&self, job: Job)
    {
        let shared = &self.inner.shared;

        let mut queue = shared.queue.lock().unwrap();
        queue.jobs.push_back(job);

        let needs_worker = queue.jobs.len() > queue.idle && queue.workers < shared.max_workers;
        let worker_id = queue.workers;
        if needs_worker
        {
            queue.workers += 1;
        }
        drop(queue);

        if needs_worker
        {
            let shared = Arc::clone(shared);
            thread::Builder::new().name(format!("engine-worker-{worker_id}")).spawn(move || worker_loop(shared)).expect("failed to spawn worker thread");
        }
        shared.signal.notify_one();
    }

    // no threads on wasm
    #[cfg(target_arch = "wasm32")]
    fn execute(&self, job: Job)
    {
        job();
    }
}

// Loops continously
// If there is a job, it takes it, stops if shutdown is true
// Otherwise it increases idle, and waits until it gets the signal from Condvar
// Once woken up, it checks for the job again, in case any other worker took it already
// lock is then released, so other workers can grab jobs now too
// Then the job runs, if it panics it gets caught so the worker survives
#[cfg(not(target_arch = "wasm32"))]
fn worker_loop(shared: Arc<Shared>)
{
    loop
    {
        let job =
        {
            let mut queue = shared.queue.lock().unwrap();
            loop
            {
                if let Some(job) = queue.jobs.pop_front() { break job; }
                if queue.shutdown { return; }

                queue.idle += 1;
                queue = shared.signal.wait(queue).unwrap();
                queue.idle -= 1;
            }
        };

        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(job));
    }
}

pub enum TaskPoll<T>
{
    Pending,
    Ready(T),
    Failed
}

// Contains possible result of a worker, ready returns once
pub struct Task<T>
{
    receiver: Option<mpsc::Receiver<T>>,
    failed: bool
}

impl<T> Task<T>
{
    // to handle Failed, unlike try_take
    pub fn poll(&mut self) -> TaskPoll<T>
    {
        let Some(receiver) = &self.receiver else
        {
            return if self.failed { TaskPoll::Failed } else { TaskPoll::Pending };
        };

        match receiver.try_recv()
        {
            Ok(value) =>
            {
                self.receiver = None;
                TaskPoll::Ready(value)
            }
            Err(TryRecvError::Empty) => TaskPoll::Pending,
            Err(TryRecvError::Disconnected) =>
            {
                self.receiver = None;
                self.failed = true;
                TaskPoll::Failed
            }
        }
    }

    // Some(value) if it is done otherwise None (if failed or still running)
    pub fn try_take(&mut self) -> Option<T>
    {
        match self.poll()
        {
            TaskPoll::Ready(value) => Some(value),
            _ => None
        }
    }

    // Blocks uthread until done
    // For example, when closing the game, while a save is running
    // Or to make a couple workers work in parallel (like in setup, instead of one after the other), can be used inside of a worker
    pub fn wait(mut self) -> Option<T>
    {
        self.receiver.take()?.recv().ok()
    }
}
