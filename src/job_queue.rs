//use thiserror::Error;
use tokio::sync::mpsc::{self, Receiver, Sender};

use crate::prelude::*;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Queue is already running")]
    AlreadyRunning,
    #[error("Invalid message queue size")]
    InvalidMessageQueueSize,
    #[error("Invalid thread pool size")]
    InvalidThreadPoolSize,
    #[error(transparent)]
    Join(#[from] tokio::task::JoinError),
    #[error("Missing channel for communicating with thread")]
    MissingChannel,
    #[error("Missing thread's join handle")]
    MissingJoinHandle,
    #[error("Queue is not started")]
    NotStarted,
    #[error("Queue is not stopping")]
    NotStopping,
    #[error("Queue is stopped")]
    Stopped,
    #[error(transparent)]
    MessageSend(#[from] tokio::sync::mpsc::error::SendError<Message>),
}

pub enum Message {
    Command(Cmd),
    Job(Job),
}

pub enum Cmd {
    Stop,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum State {
    #[default]
    Idle,
    Running,
    Stopping,
    Stopped,
}

#[derive(Debug, Default)]
pub struct JobQueue {
    message_queue_size: usize,
    _thread_pool_size: usize,
    state: State,
    tx: Option<Sender<Message>>,
    join_handle: Option<tokio::task::JoinHandle<()>>,
}

impl JobQueue {
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
            ..Default::default()
        })
    }

    /// Gets the state of the queue.
    ///
    /// # Returns
    /// A value of the enum `State`.
    pub fn state(&self) -> State {
        self.state
    }

    /// Starts the job queue with async support.
    ///
    /// # Errors
    /// One of `Error` enum.
    pub fn start(&mut self) -> Result<(), Error> {
        self.try_starting()?;

        let mut rx = self.setup_channel();

        let handle = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if JobQueue::process_msg(msg) {
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
    pub async fn enqueue(&self, job: Job) -> Result<(), Error> {
        if let Some(tx) = &self.tx {
            tx.send(Message::Job(job)).await.map_err(Into::into)
        } else {
            Err(Error::MissingChannel)
        }
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

    /// Setups the channels for communicatio with the thread of the queue.
    ///
    /// # Returns
    /// The receiver channel use to start the consumer thread.
    fn setup_channel(&mut self) -> Receiver<Message> {
        let (tx, rx) = mpsc::channel(self.message_queue_size);
        self.tx = Some(tx);
        rx
    }

    /// Processes a message (can be a command or job).
    ///
    /// # Arguments
    /// * `msg` - Message to be processed.
    ///
    /// # Returns
    /// `True` if the queue must be stopped, `False` otherwise.
    fn process_msg(msg: Message) -> bool {
        match msg {
            Message::Command(Cmd::Stop) => true,
            Message::Job(job) => {
                JobQueue::process_job(job);
                false
            }
        }
    }

    /// Processes a job.
    ///
    /// # Arguments
    /// * `job` - Job to be processed.
    ///
    /// TODO: run in thread pool
    fn process_job(job: Job) {
        job.run();
    }
}
