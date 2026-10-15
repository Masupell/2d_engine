use std::{collections::VecDeque, panic::{AssertUnwindSafe, catch_unwind, resume_unwind}, sync::{Arc, Condvar, Mutex, atomic::{AtomicUsize, Ordering}, mpsc::{self, TryRecvError}}, thread, time::Duration};

type Job = Box<dyn FnOnce() + Send + 'static>;

const IDLE_TIMEOUT: Duration = Duration::from_secs(10);

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


pub struct PoolStatus
{
    pub busy: usize,   // running a job right now
    pub alive: usize,  // spawned and not exited
    pub max: usize,
    pub queued: usize  // jobs waiting for a free worker
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

    // Calls work as many times as count, spread out over workers and calling thread.
    // Block caller until everything is done, so it can borrow data
    pub fn parallel_for<F>(&self, count: usize, work: F)
    where
        F: Fn(usize) + Sync
    {
        if count == 0 { return; }

        // ~4 chunks per thread
        let threads = self.max_workers() + 1;
        let chunk = (count / (threads * 4)).max(1);
        let helpers = self.max_workers().min(count.div_ceil(chunk) - 1);

        if helpers == 0
        {
            (0..count).for_each(work);
            return;
        }

        // Is safe, nothing uses 'ParallelFor', after this function returns
        let work: &(dyn Fn(usize) + Sync) = &work;
        let work: *const (dyn Fn(usize) + Sync + 'static) = unsafe { std::mem::transmute(work) };

        let job = Arc::new(ParallelFor
        {
            work,
            count,
            chunk,
            next: AtomicUsize::new(0),
            state: Mutex::new(ForState { running: 0, closed: false, panicked: false }),
            done: Condvar::new()
        });

        for _ in 0..helpers
        {
            let job = Arc::clone(&job);
            self.execute(Box::new(move || job.help()));
        }

        // So the caller does not just sit around idle, it helps as well, caught so it waits for the workers
        let result = catch_unwind(AssertUnwindSafe(|| job.run_chunks()));

        let mut state = job.state.lock().unwrap();
        state.closed = true;
        while state.running > 0
        {
            state = job.done.wait(state).unwrap();
        }
        let panicked = state.panicked;
        drop(state);

        if let Err(payload) = result { resume_unwind(payload); }
        if panicked { panic!("parallel_for: work panicked on a worker thread"); }
    }

    // data gets split into chunks that get processed at the same time
    // (data: &mut [0, 1, 2, 3]; chunk_length: 2, ...) => does [0, 1] and [2, 3] (not neccessarily in this order)
    pub fn parallel_for_mut<T, F>(&self, data: &mut [T], chunk_len: usize, work: F)
    where
        T: Send,
        F: Fn(usize, &mut [T]) + Sync
    {
        assert!(chunk_len > 0, "parallel_for_mut: chunk_len must be > 0");

        let len = data.len();
        let data = SlicePtr(data.as_mut_ptr());

        self.parallel_for(len.div_ceil(chunk_len), |chunk_index|
        {
            let start = chunk_index * chunk_len;
            let end = (start + chunk_len).min(len);
            // Safe, because ranges never overlap
            let chunk = unsafe { std::slice::from_raw_parts_mut(data.get().add(start), end - start) };
            work(chunk_index, chunk);
        });
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

    pub fn status(&self) -> PoolStatus
    {
        let shared = &self.inner.shared;
        let queue = shared.queue.lock().unwrap();
        PoolStatus { busy: queue.workers - queue.idle, alive: queue.workers, max: shared.max_workers, queued: queue.jobs.len() }
    }

    // no threads on wasm
    #[cfg(target_arch = "wasm32")]
    fn execute(&self, job: Job)
    {
        job();
    }
}

// state.closed makes sure, that it can't be called after the borrow to the work ends.
struct ParallelFor
{
    work: *const (dyn Fn(usize) + Sync),
    count: usize,
    chunk: usize,
    next: AtomicUsize,
    state: Mutex<ForState>,
    done: Condvar
}

struct ForState
{
    running: usize,
    closed: bool,
    panicked: bool
}

unsafe impl Send for ParallelFor {}
unsafe impl Sync for ParallelFor {}

impl ParallelFor
{
    fn run_chunks(&self)
    {
        let work = unsafe { &*self.work };
        loop
        {
            let start = self.next.fetch_add(self.chunk, Ordering::Relaxed);
            if start >= self.count { return; }

            let end = (start + self.chunk).min(self.count);
            for index in start..end
            {
                work(index);
            }
        }
    }

    fn help(&self)
    {
        {
            let mut state = self.state.lock().unwrap();
            if state.closed { return; }
            state.running += 1;
        }

        let result = catch_unwind(AssertUnwindSafe(|| self.run_chunks()));

        let mut state = self.state.lock().unwrap();
        state.panicked |= result.is_err();
        state.running -= 1;
        drop(state);
        self.done.notify_all();
    }
}

struct SlicePtr<T>(*mut T);

unsafe impl<T: Send> Send for SlicePtr<T> {}
unsafe impl<T: Send> Sync for SlicePtr<T> {}

impl<T> SlicePtr<T>
{
    fn get(&self) -> *mut T
    {
        self.0
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

                // queue.idle += 1;
                // queue = shared.signal.wait(queue).unwrap();
                // queue.idle -= 1;
                queue.idle += 1;
                let (guard, wait) = shared.signal.wait_timeout(queue, IDLE_TIMEOUT).unwrap();
                queue = guard;
                queue.idle -= 1;

                // nothing to do for a while: leave (under the lock, so execute() sees the right count and respawns when needed)
                if wait.timed_out() && queue.jobs.is_empty()
                {
                    queue.workers -= 1;
                    return;
                }
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
