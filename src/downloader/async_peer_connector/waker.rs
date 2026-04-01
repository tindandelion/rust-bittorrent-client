use std::{
    sync::{Arc, Mutex},
    task::Wake,
};

pub struct MyWaker {
    task_id: usize,
    ready_queue: Arc<Mutex<Vec<usize>>>,
}

impl MyWaker {
    pub fn new(task_id: usize, ready_queue: Arc<Mutex<Vec<usize>>>) -> Self {
        Self {
            task_id,
            ready_queue,
        }
    }
}

impl Wake for MyWaker {
    fn wake(self: Arc<Self>) {
        println!("Waking task {}", self.task_id);
        self.ready_queue.lock().unwrap().push(self.task_id);
    }
}
