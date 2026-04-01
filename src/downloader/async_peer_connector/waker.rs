use std::{sync::Arc, task::Wake};

pub struct MyWaker {
    task_id: usize,
}

impl MyWaker {
    pub fn new(task_id: usize) -> Self {
        Self { task_id }
    }
}

impl Wake for MyWaker {
    fn wake(self: Arc<Self>) {
        println!("Waking task {}", self.task_id);
    }
}
