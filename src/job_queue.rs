use std::sync::Arc;
use tokio::sync::mpsc::{self, Sender};
use tokio::sync::Mutex;

use crate::memory_backend::*;
use crate::prelude::*;

/// Type of messages that can be sent to the job queue.
pub enum Message {
    /// Command message that change the state of the queue.
    Command(Cmd),

    /// Job message used to push a new job to be processed.
    Job(Job),
}

/// Commands handled by the thread of the job queue.
pub enum Cmd {
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

    /// Stopped indefinitely.
    Stopped,
}

pub struct JobQueue<RoutineType> {
    /// Maximum number of messages that can be in the queue at the same time.
    message_queue_size: usize,

    /// Number of threads to spawn in the pool.
    _thread_pool_size: usize,

    /// State of the job queue.
    state: State,

    /// Channel used to send messages to the thread of the job queue.
    tx: Option<Sender<Message>>,

    /// Join handle used to wait the thread of the job queue.
    join_handle: Option<tokio::task::JoinHandle<()>>,

    /// Backend used to store the list of jobs with their results.
    backend: SharedBackend<RoutineType>,
}

impl<RoutineType> JobQueue<RoutineType>
where
    RoutineType: Routine + Sync + 'static,
{
    /// Creates a new job queue.
    ///
    /// # Arguments
    /// * `message_queue_size` - Length of the internal message queue.
    /// * `thread_pool_size` - Number of thread to allocate in the internal thread pool.
    ///
    /// # Returns
    /// An instance of `JobQueue`.
    pub fn new(message_queue_size: usize, _thread_pool_size: usize) -> Result<Self, Error> {
        if message_queue_size == 0 {
            return Err(Error::InvalidMessageQueueSize);
        }

        if _thread_pool_size == 0 {
            return Err(Error::InvalidThreadPoolSize);
        }

        Ok(Self {
            message_queue_size,
            _thread_pool_size,
            state: State::default(),
            tx: None,
            join_handle: None,
            backend: Arc::new(Mutex::new(Box::new(MemoryBackend::new()))),
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
        self.backend = Arc::new(Mutex::new(Box::new(backend)));
    }

    /// Starts the job queue with async support.
    ///
    /// # Errors
    /// One of `Error` enum.
    pub fn start(&mut self) -> Result<(), Error> {
        self.try_starting()?;

        let (tx, mut rx) = mpsc::channel(self.message_queue_size);
        self.tx = Some(tx);

        let backend = self.backend.clone();

        let handle = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if JobQueue::process_message(backend.clone(), msg).await {
                    break;
                }
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
    pub async fn join(&mut self) -> Result<(), Error> {
        self.try_joining()?;

        if let Some(handle) = &mut self.join_handle {
            handle.await?;
            self.join_handle = None;
            self.state = State::Stopped;
        } else {
            return Err(Error::MissingJoinHandle);
        }

        Ok(())
    }

    /// Send aa stop command to the queue.
    /// There's no garantee that it will be processed but we'll do our best.
    ///
    /// # Errors
    /// One of `Error` enum.
    pub async fn stop(&mut self) -> Result<(), Error> {
        self.try_stopping()?;

        if let Some(tx) = &self.tx {
            self.state = State::Stopping;

            tx.send(Message::Command(Cmd::Stop))
                .await
                .map_err(Into::into)
        } else {
            Err(Error::MissingChannel)
        }
    }

    /// Push a new job to be processed in the queue.
    ///
    /// # Arguments
    /// * `job` - Job to be enqueued.
    ///
    /// # Errors
    /// One of `Error` enum.
    pub async fn enqueue(&self, job: Job) -> Result<Uuid, Error> {
        if let Some(tx) = &self.tx {
            let job_id = job.id();

            tx.send(Message::Job(job))
                .await
                .map_err(Into::<Error>::into)?;

            Ok(job_id)
        } else {
            Err(Error::MissingChannel)
        }
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

    /// Checks if the current state allows to start the queue.
    ///
    /// # Errors
    /// One of `Error` enum.
    fn try_starting(&self) -> Result<(), Error> {
        match self.state {
            State::Running => Err(Error::AlreadyRunning),
            State::Stopped | State::Stopping => Err(Error::Stopped),
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
            State::Stopped | State::Stopping => Err(Error::Stopped),
            _ => Ok(()),
        }
    }

    /// Processes a message (can be a command or job).
    ///
    /// # Arguments
    /// * `msg` - Message to be processed.
    ///
    /// # Returns
    /// `True` if the queue must be stopped, `False` otherwise.
    async fn process_message(backend: SharedBackend<RoutineType>, msg: Message) -> bool {
        match msg {
            Message::Command(Cmd::Stop) => true,
            Message::Job(job) => {
                let _ = JobQueue::process_job(backend, job).await;
                false
            }
        }
    }

    /// Processes a job.
    ///
    /// # Arguments
    /// * `backend` - Backend instance used to process the jobs.
    /// * `job` - Job to be processed.
    async fn process_job(backend: SharedBackend<RoutineType>, job: Job) -> Result<(), Error> {
        let mut backend = backend.lock().await;

        let job_id = job.id();
        backend.schedule(job)?;
        backend.set_status(&job_id, Status::Ready)?;

        backend.set_status(&job_id, Status::Running)?;
        backend.run(&job_id).await?;
        backend.set_status(&job_id, Status::Finished)?;

        Ok(())
    }
}
