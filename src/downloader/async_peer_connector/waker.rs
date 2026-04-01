use std::{
    sync::{Arc, Mutex},
    task::Wake,
};

pub struct TaskWaker {
    task_id: usize,
    ready_queue: Arc<Mutex<Vec<usize>>>,
}

impl TaskWaker {
    pub fn new(task_id: usize, ready_queue: Arc<Mutex<Vec<usize>>>) -> Self {
        Self {
            task_id,
            ready_queue,
        }
    }
}

impl Wake for TaskWaker {
    fn wake(self: Arc<Self>) {
        self.ready_queue.lock().unwrap().push(self.task_id);
    }
}
