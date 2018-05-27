extern crate libc;

use std::sync::mpsc;
use std::thread;
use std::sync::Arc;
use std::sync::Mutex;
use libc::c_char;
use std::ffi::CString;

// FFI into C functions
extern "C" {
    fn say_something(phrase: *const c_char);
    fn shout_something(phrase: *const c_char);
}

pub struct Threadpool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Message>,
}

pub struct Worker {
    // This should be an option so that we can clean up the thread when shuttind down.
    thread: Option<thread::JoinHandle<()>>,
    id: i8,
}

enum Message {
    NewJob(Job),
    Terminate,
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
    fn new(receiver: Arc<Mutex<mpsc::Receiver<Message>>>, id: i8) -> Worker {
        let thread = thread::spawn(move || loop {
            let message = receiver.lock().unwrap().recv().unwrap();

            match message {
                Message::NewJob(job) => {
                    print!("Worker {} processing cells \n", id);
                    job.call_box();
                }
                Message::Terminate => {
                    print!("Worker {} terminating \n", id);
                    break;
                }
            }
        });

        Worker {
            thread: Some(thread),
            id: id,
        }
    }
}

impl Threadpool {
    pub fn new(s: usize) -> Threadpool {
        let mut workers = Vec::with_capacity(s);

        let (sender, receiver) = mpsc::channel();

        let receiver = Arc::new(Mutex::new(receiver));

        for id in 0..s {
            let worker = Worker::new(Arc::clone(&receiver), id as i8);
            workers.push(worker);
        }

        Threadpool { workers, sender }
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);

        self.sender.send(Message::NewJob(job)).unwrap();
    }
}

impl Drop for Threadpool {
    // join each thread when the thread pool goes out of scope
    fn drop(&mut self) {
        // send terminate message before joining each thread
        for _ in &mut self.workers {
            self.sender.send(Message::Terminate).unwrap();
        }

        for worker in &mut self.workers {
            println!("Shutting down worker {}", worker.id);
            if let Some(thread) = worker.thread.take() {
                // move the worker's thread from Some to None
                thread.join().unwrap();
            }
        }
    }
}

fn handle(cell_queue: Vec<Cell>) {
    for cell in cell_queue {
        execute_task(cell.task, cell.ptr);;
    }
}

enum Task {
    Say,
    Shout,
}

fn execute_task(task: Task, ptr: *const c_char) {
    match task {
        Task::Say => unsafe { say_something(ptr) },
        Task::Shout => unsafe { shout_something(ptr) },
    }
}

unsafe impl Send for Cell {}
struct Cell {
    ptr: *const c_char,
    task: Task,
}

fn main() {
    let pool = Threadpool::new(2);

    let hi_str = CString::new("Hi!").unwrap();
    let hello_str = CString::new("Hello!").unwrap();
    let howdy_str = CString::new("Howdy!").unwrap();
    let yo_str = CString::new("Yo!").unwrap();

    let cell_one = Cell {
        ptr: hi_str.as_ptr(),
        task: Task::Say,
    };
    let cell_two = Cell {
        ptr: hello_str.as_ptr(),
        task: Task::Shout,
    };
    let cell_three = Cell {
        ptr: howdy_str.as_ptr(),
        task: Task::Shout,
    };
    let cell_four = Cell {
        ptr: yo_str.as_ptr(),
        task: Task::Say,
    };

    let a = vec![cell_one];
    let b = vec![cell_two, cell_three];
    let c = vec![cell_four];

    let cell_work_queue = vec![a, b, c];

    println!("Processing cells.");

    for n in cell_work_queue {
        pool.execute(move || { handle(n); });
    }

    println!("Shutting down.");
}
