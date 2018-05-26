extern crate libc;

use std::sync::mpsc;
use std::thread;
use std::sync::Arc;
use std::sync::Mutex;
use std::time;
use libc::c_char;
use std::ffi::CString;
use std::ffi::CStr;

// FFI into C functions
extern "C" {
    fn say_something(phrase: *const c_char);
    fn shout_something(phrase: *const c_char);
}

pub struct Threadpool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Job>,
}

pub struct Worker {
    thread: thread::JoinHandle<()>,
    id: i8,
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
    fn new(receiver: Arc<Mutex<mpsc::Receiver<Job>>>, id: i8) -> Worker {
        let thread = thread::spawn(move || loop {
            let job = receiver.lock().unwrap().recv().unwrap();
            print!("Worker {} processing cells \n", id);
            job.call_box();
        });

        Worker { thread, id }
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

        self.sender.send(job).unwrap();
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

    for n in cell_work_queue {
        pool.execute(move || { handle(n); });
    }
}
