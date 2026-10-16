use std::{collections::VecDeque, panic::{AssertUnwindSafe, catch_unwind, resume_unwind}, ptr, sync::{Arc, Condvar, Mutex, atomic::{AtomicBool, AtomicPtr, AtomicUsize, Ordering}, mpsc::{self, TryRecvError}}, thread, time::{Duration, Instant}};

type Job = Box<dyn FnOnce() + Send + 'static>;

const IDLE_TIMEOUT: Duration = Duration::from_secs(10);
const SPIN_TIME: Duration = Duration::from_micros(50); // time a worker looks before new job before waiting

struct Queue
{
    jobs: VecDeque<Job>,
    sleeping: usize,
    workers: usize,
    shutdown: bool
}

struct Shared
{
    queue: Mutex<Queue>,
    signal: Condvar, // to wake sleeping workers
    queued: AtomicUsize, // same as queue.jobs.len(), but can be read without lock
    spinning: AtomicUsize, // looking for work without the lock
    max_workers: usize,

    // parallel_for does not use the queue
    current: AtomicPtr<ParallelFor<'static>>, // running 'parallel_for', null if none
    generation: AtomicUsize, // +1 for every parallel_for
    helping: AtomicUsize, // workers that might be using current right now
    claimed: AtomicBool, // only one parallel_for at a time can use current
    finished: Mutex<()>, // to sleep until helping is 0, if the workers take longer
    finished_signal: Condvar
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
            self.shared.queued.store(0, Ordering::Relaxed);
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
// Max amount of workers is cores-1 (main thread has one core), none on wasm
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
        let max_workers = if cfg!(target_arch = "wasm32") { 0 } else { cores.saturating_sub(1).max(1) };

        let shared = Arc::new(Shared
        {
            queue: Mutex::new(Queue { jobs: VecDeque::new(), sleeping: 0, workers: 0, shutdown: false }),
            signal: Condvar::new(),
            queued: AtomicUsize::new(0),
            spinning: AtomicUsize::new(0),
            max_workers,
            current: AtomicPtr::new(ptr::null_mut()),
            generation: AtomicUsize::new(0),
            helping: AtomicUsize::new(0),
            claimed: AtomicBool::new(false),
            finished: Mutex::new(()),
            finished_signal: Condvar::new()
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
    // Only one at a time uses the workers
    // Nested or from a second threas
    pub fn parallel_for<F>(&self, count: usize, work: F)
    where
        F: Fn(usize) + Sync
    {
        if count == 0 { return; }

        // ~4 chunks per thread
        let threads = self.max_workers() + 1;
        let chunk = (count / (threads * 4)).max(1);
        let helpers = self.max_workers().min(count.div_ceil(chunk) - 1);

        let shared = &self.inner.shared;
        if helpers == 0 || shared.claimed.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed).is_err()
        {
            (0..count).for_each(work);
            return;
        }

        let job = ParallelFor { work: &work, count, chunk, next: AtomicUsize::new(0), panicked: AtomicBool::new(false) };

        // The cast only removes the lifetime, finish_parallel_for makes sure nothing uses it after this function
        shared.current.store(&job as *const ParallelFor as *mut ParallelFor<'static>, Ordering::SeqCst);
        shared.generation.fetch_add(1, Ordering::SeqCst);
        self.wake_helpers(helpers);

        // So the caller does not just sit around idle, it helps as well, caught so it waits for the workers
        let result = catch_unwind(AssertUnwindSafe(|| job.run_chunks()));

        finish_parallel_for(shared);
        shared.claimed.store(false, Ordering::Release);

        if let Err(payload) = result { resume_unwind(payload); }
        if job.panicked.load(Ordering::Relaxed) { panic!("parallel_for: work panicked on a worker thread"); }
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

    fn wake_helpers(&self, helpers: usize)
    {
        let shared = &self.inner.shared;

        let mut queue = shared.queue.lock().unwrap();
        let spawn = helpers.saturating_sub(queue.workers);
        let first_id = queue.workers;
        queue.workers += spawn;
        let wake = helpers.min(queue.sleeping);
        let all = wake == queue.sleeping;
        drop(queue);

        spawn_workers(shared, first_id, spawn);
        if all { if wake > 0 { shared.signal.notify_all(); } }
        else { for _ in 0..wake { shared.signal.notify_one(); } }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn execute(&self, job: Job)
    {
        let shared = &self.inner.shared;

        let mut queue = shared.queue.lock().unwrap();
        queue.jobs.push_back(job);
        shared.queued.fetch_add(1, Ordering::Relaxed);

        let idle = queue.sleeping + shared.spinning.load(Ordering::Relaxed);
        let spawn = usize::from(queue.jobs.len() > idle && queue.workers < shared.max_workers);
        let first_id = queue.workers;
        queue.workers += spawn;
        let wake = queue.sleeping > 0;
        drop(queue);

        spawn_workers(shared, first_id, spawn);
        if wake { shared.signal.notify_one(); }
    }

    pub fn status(&self) -> PoolStatus
    {
        let shared = &self.inner.shared;
        let queue = shared.queue.lock().unwrap();
        let idle = queue.sleeping + shared.spinning.load(Ordering::Relaxed);
        PoolStatus { busy: queue.workers.saturating_sub(idle), alive: queue.workers, max: shared.max_workers, queued: queue.jobs.len() }
    }

    // no threads on wasm
    #[cfg(target_arch = "wasm32")]
    fn execute(&self, job: Job)
    {
        job();
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn spawn_workers(shared: &Arc<Shared>, first_id: usize, count: usize)
{
    for worker_id in first_id..first_id + count
    {
        let shared = Arc::clone(shared);
        thread::Builder::new().name(format!("engine-worker-{worker_id}")).spawn(move || worker_loop(shared)).expect("failed to spawn worker thread");
    }
}

#[cfg(target_arch = "wasm32")]
fn spawn_workers(_shared: &Arc<Shared>, _first_id: usize, _count: usize) {}


struct ParallelFor<'a>
{
    work: &'a (dyn Fn(usize) + Sync),
    count: usize,
    chunk: usize,
    next: AtomicUsize,
    panicked: AtomicBool
}

impl ParallelFor<'_>
{
    fn run_chunks(&self)
    {
        loop
        {
            let start = self.next.fetch_add(self.chunk, Ordering::Relaxed);
            if start >= self.count { return; }

            let end = (start + self.chunk).min(self.count);
            for index in start..end
            {
                (self.work)(index);
            }
        }
    }
}

// 'helping' goes up before and down after
// if current is null, 'parallel_for' is already finishing
#[cfg(not(target_arch = "wasm32"))]
fn help_parallel_for(shared: &Shared)
{
    shared.helping.fetch_add(1, Ordering::SeqCst);

    let job = shared.current.load(Ordering::SeqCst);
    if !job.is_null()
    {
        let job = unsafe { &*job };
        if catch_unwind(AssertUnwindSafe(|| job.run_chunks())).is_err()
        {
            job.panicked.store(true, Ordering::Relaxed);
        }
    }

    // Last one wakes caller, can't miss because of lock
    if shared.helping.fetch_sub(1, Ordering::SeqCst) == 1
    {
        drop(shared.finished.lock().unwrap());
        shared.finished_signal.notify_all();
    }
}

// 'SeqCst', a worker raised helping before 'current' became null (then caller waits) or it comes after and sees null
fn finish_parallel_for(shared: &Shared)
{
    shared.current.store(ptr::null_mut(), Ordering::SeqCst);
    if shared.helping.load(Ordering::SeqCst) == 0 { return; }

    let start = Instant::now();
    while shared.helping.load(Ordering::SeqCst) > 0 && start.elapsed() < SPIN_TIME
    {
        std::hint::spin_loop();
    }

    let mut guard = shared.finished.lock().unwrap();
    while shared.helping.load(Ordering::SeqCst) > 0
    {
        guard = shared.finished_signal.wait(guard).unwrap();
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
// If a new generation, this comies first (from 'parallel_for')
// If not, then a job from the queue
// Otherwise it spins for 50us without a lock, reading only the atomics
// If nothing now, like befre, it sleeps until it gets a signal from condvar, after 10s, the worker exits
#[cfg(not(target_arch = "wasm32"))]
fn worker_loop(shared: Arc<Shared>)
{
    let mut seen = 0;
    let mut spun = false;

    loop
    {
        let generation = shared.generation.load(Ordering::SeqCst);
        if generation != seen
        {
            seen = generation;
            help_parallel_for(&shared);
            spun = false;
            continue;
        }

        let mut queue = shared.queue.lock().unwrap();
        if let Some(job) = queue.jobs.pop_front()
        {
            shared.queued.fetch_sub(1, Ordering::Relaxed);
            drop(queue);
            let _ = catch_unwind(AssertUnwindSafe(job));
            spun = false;
            continue;
        }
        if queue.shutdown { return; }

        if !spun
        {
            spun = true;
            drop(queue);
            shared.spinning.fetch_add(1, Ordering::Relaxed);
            spin_for_work(&shared, seen);
            shared.spinning.fetch_sub(1, Ordering::Relaxed);
            continue;
        }

        if shared.generation.load(Ordering::SeqCst) != seen { continue; }

        queue.sleeping += 1;
        let (mut queue, wait) = shared.signal.wait_timeout(queue, IDLE_TIMEOUT).unwrap();
        queue.sleeping -= 1;

        if wait.timed_out() && queue.jobs.is_empty() && shared.generation.load(Ordering::SeqCst) == seen
        {
            queue.workers -= 1;
            return;
        }
    }
}

// Waking from condvar takes os a while (like 10-50us, not crazy, but for many uses, it can add up)
// And work often comes right after each other (like parallel_for, called multiple times a frame)
#[cfg(not(target_arch = "wasm32"))]
fn spin_for_work(shared: &Shared, seen: usize)
{
    let start = Instant::now();
    while start.elapsed() < SPIN_TIME
    {
        if shared.queued.load(Ordering::Relaxed) > 0 || shared.generation.load(Ordering::Relaxed) != seen { return; }
        std::hint::spin_loop();
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
