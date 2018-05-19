use std::sync::mpsc;
use std::thread;
use std::sync::Arc;
use std::sync::Mutex;
use std::time;

pub struct Threadpool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Job>,
}

pub struct Worker {
    thread: thread::JoinHandle<()>,
}

// Tell Rust explicitely that we can take ownership of the value inside the Box
trait FnBox {
    fn call_box(self: Box<Self>);
}

impl<F: FnOnce()> FnBox for F {
    fn call_box(self: Box<F>) {
        (*self)()
    }
}
type Job = Box<FnBox + Send + 'static>;

impl Worker {
    fn new(receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
        let thread = thread::spawn(move || {
            loop {
                // can panic if the mutex is in a poisened state- for example, if
                // another thread panicked while holding the lock
                let job = receiver.lock().unwrap().recv().unwrap();
                job.call_box();
            }
        });


        Worker { thread }
    }
}

impl Threadpool {
    // TODO return an Result with a possible error if the size is less than 0
    pub fn new(s: usize) -> Threadpool {
        let mut workers = Vec::with_capacity(s);

        let (sender, receiver) = mpsc::channel();

        // Arc allows multiple workers to own the receiver
        // Mutex ensures that only one worker gets a job from the receiver at
        // a time.
        // Overall this allows us to share the receiving end of the channel
        // with mulitple workers in a way that is mutable by each worker.
        let receiver = Arc::new(Mutex::new(receiver));

        for _ in 0..s {
            let worker = Worker::new(Arc::clone(&receiver));
            workers.push(worker);
        }

        Threadpool { workers, sender }
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        // create a job type alias that holds the closure
        let job = Box::new(f);

        // send the job down the channel
        // call unwrap in case sendig the job failed. This can happen in the
        // case that all threads have been stopped
        self.sender.send(job).unwrap();
    }
}

fn handle(n: i8) {
    print!("Hello, world! {} \n", n);
}

fn handle_odd(n: i8, m: i8) {
    print!("Hello, world odd! {} \n", n);
}

fn main() {
    let pool = Threadpool::new(4);

    for n in 1..10 {
        let number = n;
        if n % 2 == 0 {
            pool.execute(move || { handle(number); });
        } else {
            pool.execute(move || { handle_odd(number, number); });
        }
    }

    // Allow all threads to finish
    thread::sleep(time::Duration::from_millis(5000));
}
