pub struct Job {
    routine: Box<dyn FnOnce() + Send>,
}

impl Job {
    pub fn new(routine: Box<dyn FnOnce() + Send>) -> Self {
        Self { routine }
    }

    pub fn run(self) {
        (self.routine)();
    }
}
