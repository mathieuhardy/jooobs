use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::sync::Mutex;
use std::thread::JoinHandle;
use tokio::runtime::{Builder, Runtime};
use tokio::sync::Mutex as AsyncMutex;

use crate::memory_backend::*;
use crate::prelude::*;

/// Type of messages that can be sent to the job queue.
#[derive(PartialEq)]
pub enum Message {
    /// Command message that change the state of the queue.
    Command(Cmd),

    /// Job message used to push a new job to be processed.
    Job(Job),
}

/// Commands handled by the thread of the job queue.
#[derive(PartialEq)]
pub enum Cmd {
    /// Set current step for a job.
    SetStep(Uuid, u64),

    /// Set number of steps for a job.
    SetSteps(Uuid, u64),

    /// Stop the job queue.
    Stop,
}

/// States of the tread running the job queue.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum State {
    /// Idle (i.e. waiting to be started).
    #[default]
    Idle,

    /// Running and processing jobs.
    Running,

    /// Going to stop.
    Stopping,
}

/// Type used to share the runtime instance across threads.
pub type SharedRuntime = Arc<Mutex<Runtime>>;

/// Type used to share the error handler across threads.
pub type SharedErrorHandler = Arc<dyn Fn(Error) + Send + Sync>;

/// Type used to share the message channel.
pub type SharedMessageChannel = Arc<Mutex<Sender<Message>>>;

pub struct JobQueue<RoutineType> {
    /// State of the job queue.
    state: State,

    /// Channel used to send messages to the thread of the job queue.
    tx: SharedMessageChannel,

    /// Channel used to receive messages from the thread of the job queue.
    rx: Arc<Mutex<Receiver<Message>>>,

    /// Join handle used to wait the thread of the job queue.
    join_handle: Option<JoinHandle<()>>,

    /// Backend used to store the list of jobs with their results.
    backend: SharedBackend<RoutineType>,

    /// Tokio runtime instance with dedicated thread pool.
    runtime: SharedRuntime,

    /// Error handler function
    error_handler: SharedErrorHandler,
}

impl<RoutineType> JobQueue<RoutineType>
where
    RoutineType: Routine + Sync + 'static,
{
    /// Creates a new job queue.
    ///
    /// # Arguments
    /// * `thread_pool_size` - Number of thread to allocate in the internal thread pool.
    /// * `error_handler` - User handler used to display some error messages.
    ///
    /// # Returns
    /// An instance of `JobQueue`.
    pub fn new(
        thread_pool_size: usize,
        error_handler: impl Fn(Error) + Send + Sync + 'static,
    ) -> Result<Self, Error> {
        if thread_pool_size == 0 {
            return Err(Error::InvalidThreadPoolSize);
        }

        let (tx, rx) = std::sync::mpsc::channel();

        let runtime = Builder::new_multi_thread()
            .worker_threads(thread_pool_size)
            .build()?;

        Ok(Self {
            state: State::default(),
            tx: Arc::new(Mutex::new(tx)),
            rx: Arc::new(Mutex::new(rx)),
            join_handle: None,
            backend: Arc::new(AsyncMutex::new(Box::new(MemoryBackend::new()))),
            runtime: Arc::new(Mutex::new(runtime)),
            error_handler: Arc::new(error_handler),
        })
    }

    /// Gets the state of the queue.
    ///
    /// # Returns
    /// A value of the enum `State`.
    pub fn state(&self) -> State {
        self.state
    }

    /// Sets the backend used by the queue to store jobs and their results.
    ///
    /// # Arguments:
    /// * `backend` - Backend instance that will replace the current one.
    pub fn set_backend(&mut self, backend: impl Backend<RoutineType> + 'static) {
        self.backend = Arc::new(AsyncMutex::new(Box::new(backend)));
    }

    /// Starts the job queue with async support.
    ///
    /// # Errors
    /// One of `Error` enum.
    pub fn start(&mut self) -> Result<(), Error> {
        self.try_starting()?;

        let backend = self.backend.clone();
        let runtime = self.runtime.clone();
        let rx = self.rx.clone();
        let error_handler = self.error_handler.clone();
        let messages_channel = self.tx.clone();

        let handle = std::thread::spawn(move || {
            let rx = match rx.lock() {
                Ok(rx) => rx,
                Err(e) => {
                    error_handler(Error::CannotAccessReceiver(e.to_string()));
                    return;
                }
            };

            while let Ok(msg) = rx.recv() {
                if msg == Message::Command(Cmd::Stop) {
                    break;
                }

                JobQueue::process_message(
                    backend.clone(),
                    runtime.clone(),
                    error_handler.clone(),
                    messages_channel.clone(),
                    msg,
                );
            }
        });

        self.join_handle = Some(handle);
        self.state = State::Running;

        Ok(())
    }

    /// Tries to join the job queue waiting it to finish.
    ///
    /// # Errors
    /// One of `Error` enum.
    pub fn join(self) -> Result<(), Error> {
        self.try_joining()?;

        if let Some(handle) = self.join_handle {
            if handle.join().is_err() {
                return Err(Error::CannotJoinThread);
            }

            // TODO
            //self.runtime
            //.lock()
            //.unwrap()
            //.shutdown_timeout(std::time::Duration::from_millis(100));
        } else {
            return Err(Error::MissingJoinHandle);
        }

        Ok(())
    }

    /// Send a stop command to the queue.
    /// There's no garantee that it will be processed but we'll do our best.
    ///
    /// # Errors
    /// One of `Error` enum.
    pub fn stop(&mut self) -> Result<(), Error> {
        self.try_stopping()?;

        self.state = State::Stopping;

        self.tx
            .lock()
            .unwrap() // TODO
            .send(Message::Command(Cmd::Stop))
            .map_err(Into::<Error>::into)
    }

    /// Push a new job to be processed in the queue.
    ///
    /// # Arguments
    /// * `job` - Job to be enqueued.
    ///
    /// # Errors
    /// One of `Error` enum.
    pub fn enqueue(&self, job: Job) -> Result<Uuid, Error> {
        let job_id = job.id();

        self.tx
            .lock()
            .unwrap() // TODO
            .send(Message::Job(job))
            .map_err(Into::<Error>::into)?;

        Ok(job_id)
    }

    /// Get the status of a job.
    ///
    /// # Arguments
    /// * `id` - ID of the job to be inspected.
    ///
    /// # Returns
    /// The status of the job.
    ///
    /// # Errors
    /// One of `Error` enum.
    pub async fn job_status(&self, id: &Uuid) -> Result<Status, Error> {
        let backend = self.backend.lock().await;

        backend.status(id)
    }

    /// Get the result of a job.
    ///
    /// # Arguments
    /// * `id` - ID of the job to be inspected.
    ///
    /// # Returns
    /// The result of the job as vector of bytes.
    ///
    /// # Errors
    /// One of `Error` enum.
    pub async fn job_result(&self, id: &Uuid) -> Result<Vec<u8>, Error> {
        let backend = self.backend.lock().await;

        let value = backend.result(id)?;
        Ok(value.to_vec())
    }

    /// Get the progression of a job.
    ///
    /// # Arguments
    /// * `id` - ID of the job to be inspected.
    ///
    /// # Returns
    /// The progression of the job.
    ///
    /// # Errors
    /// One of `Error` enum.
    pub async fn job_progression(&self, id: &Uuid) -> Result<Progression, Error> {
        let backend = self.backend.lock().await;

        backend.progression(id)
    }

    /// Checks if the current state allows to start the queue.
    ///
    /// # Errors
    /// One of `Error` enum.
    fn try_starting(&self) -> Result<(), Error> {
        match self.state {
            State::Running => Err(Error::AlreadyRunning),
            State::Stopping => Err(Error::Stopped),
            _ => Ok(()),
        }
    }

    /// Checks if the current state allows to join the queue.
    ///
    /// # Errors
    /// One of `Error` enum.
    fn try_joining(&self) -> Result<(), Error> {
        match self.state {
            State::Idle => Err(Error::NotStarted),
            State::Running => Err(Error::NotStopping),
            _ => Ok(()),
        }
    }

    /// Checks if the current state allows to stop the queue.
    ///
    /// # Errors
    /// One of `Error` enum.
    fn try_stopping(&self) -> Result<(), Error> {
        match self.state {
            State::Idle => Err(Error::NotStarted),
            State::Stopping => Err(Error::Stopped),
            _ => Ok(()),
        }
    }

    /// Processes a message (can be a command or job).
    ///
    /// # Arguments
    /// * `msg` - Message to be processed.
    fn process_message(
        backend: SharedBackend<RoutineType>,
        runtime: SharedRuntime,
        error_handler: SharedErrorHandler,
        messages_channel: SharedMessageChannel,
        msg: Message,
    ) {
        match msg {
            Message::Job(job) => {
                let _ = JobQueue::process_job(
                    backend,
                    runtime,
                    error_handler.clone(),
                    messages_channel.clone(),
                    job,
                )
                .map_err(|e| error_handler(e));
            }

            Message::Command(cmd) => {
                let _ = JobQueue::process_command(backend, runtime, error_handler.clone(), cmd)
                    .map_err(|e| error_handler(e));
            }
        }
    }

    /// Processes a command.
    ///
    /// # Arguments
    /// * `backend` - Backend instance used to process the jobs.
    /// * `error_handler` - Handler for logging errors.
    /// * `cmd` - Command to be processed.
    fn process_command(
        backend: SharedBackend<RoutineType>,
        runtime: SharedRuntime,
        error_handler: SharedErrorHandler,
        cmd: Cmd,
    ) -> Result<(), Error> {
        let runtime = runtime
            .lock()
            .map_err(|e| Error::CannotAccessRuntime(e.to_string()))?;

        runtime.block_on(async {
            let mut backend = backend.lock().await;

            match cmd {
                Cmd::SetSteps(job_id, steps) => {
                    let _ = backend
                        .set_steps(&job_id, steps)
                        .map_err(|e| error_handler(e));
                }

                Cmd::SetStep(job_id, step) => {
                    let _ = backend
                        .set_step(&job_id, step)
                        .map_err(|e| error_handler(e));
                }

                _ => (),
            }
        });

        Ok(())
    }

    /// Processes a job.
    ///
    /// # Arguments
    /// * `backend` - Backend instance used to process the jobs.
    /// * `runtime` - Runtime instance used to process the jobs.
    /// * `error_handler` - Handler for logging errors.
    /// * `job` - Job to be processed.
    fn process_job(
        backend: SharedBackend<RoutineType>,
        runtime: SharedRuntime,
        error_handler: SharedErrorHandler,
        messages_channel: SharedMessageChannel,
        job: Job,
    ) -> Result<(), Error> {
        let job_id = job.id();

        let runtime = runtime
            .lock()
            .map_err(|e| Error::CannotAccessRuntime(e.to_string()))?;

        runtime.block_on(async {
            let mut backend = backend.lock().await;

            let _ = backend.schedule(job).map_err(|e| error_handler(e));

            let _ = backend
                .set_status(&job_id, Status::Ready)
                .map_err(|e| error_handler(e));
        });

        runtime.spawn(async move {
            let mut backend = backend.lock().await;

            let _ = backend
                .set_status(&job_id, Status::Running)
                .map_err(|e| error_handler(e));

            let _ = backend
                .run(&job_id, messages_channel)
                .await
                .map_err(|e| error_handler(e));

            let _ = backend
                .set_status(&job_id, Status::Finished)
                .map_err(|e| error_handler(e));
        });

        Ok(())
    }
}
