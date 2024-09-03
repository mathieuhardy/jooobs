use uuid::Uuid;

use crate::job::Status;
use crate::job_queue::Message;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Queue is already running")]
    AlreadyRunning,
    #[error("Cannot access backend ({0})")]
    CannotAccessBackend(String),
    #[error("{0}")]
    Custom(String),
    #[error(transparent)]
    GenericError(#[from] Box<dyn std::error::Error>),
    #[error("Invalid job status transition: {0:?}")]
    InvalidJobStatusTransition((Status, Status)),
    #[error("Invalid message queue size")]
    InvalidMessageQueueSize,
    #[error("Invalid thread pool size")]
    InvalidThreadPoolSize,
    #[error("Job with id {0} is not found")]
    JobNotFound(Uuid),
    #[error(transparent)]
    Join(#[from] tokio::task::JoinError),
    #[error(transparent)]
    JsonSerialization(#[from] serde_json::Error),
    #[error(transparent)]
    MessageSend(#[from] tokio::sync::mpsc::error::SendError<Message>),
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
}

pub type JobError = Box<dyn std::error::Error>;
