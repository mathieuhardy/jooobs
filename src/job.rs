use serde::Serialize;
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;
use std::time::SystemTime;

use crate::prelude::*;

/// List of statuses of a job.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Status {
    /// Job is not ready and not scheduled.
    NotReady,

    /// Job is not running yet but ready to be started.
    Ready,

    /// Job is currently running.
    Running,

    /// Job is finished (on success or error).
    Finished,
}

/// Structure used to store timestamps and result of the job.
pub struct Payload {
    /// Timestamps for every step of the job.
    pub timestamps: Timestamps,

    /// Result of the job.
    pub result: Value,
}

/// Timestamps of every steps of the lifecycle of a job.
pub struct Timestamps {
    /// Timestamp at which the job is enqueued.
    pub enqueued: SystemTime,

    /// Timestamp at which the job is started.
    pub started: SystemTime,

    /// Timestamp at which the job is finished.
    pub finished: SystemTime,
}

/// Routine error.
pub type RoutineResult = Result<Value, JobError>;

/// Routine that will be executed when starting the job.
type Routine =
    Box<dyn FnOnce() -> Pin<Box<dyn Future<Output = RoutineResult> + Send>> + Send + Sync>;
// TODO: remove
//Box<dyn Fn() -> Box<dyn Future<Output = RoutineResult> + Unpin + Send> + Send + Sync>;

/// Description of a job.
pub struct Job {
    /// Unique identifier of the job.
    id: Uuid,

    /// The routine called when running.
    routine: Routine,

    /// Status of the job.
    status: Status,

    /// Payload of the job where the result will be stored.
    payload: Payload,
}

impl Job {
    /// Creates a new job given a routine to be executed.
    ///
    /// # Arguments
    /// * `routine` - Routine to be called.
    ///
    /// # Returns
    /// An `Job` instance.
    pub fn new(routine: Routine) -> Self {
        Self {
            id: Uuid::now_v1(&[1, 2, 3, 4, 5, 6]),
            routine,
            status: Status::NotReady,
            payload: Payload {
                timestamps: Timestamps {
                    enqueued: SystemTime::now(),
                    started: SystemTime::UNIX_EPOCH,
                    finished: SystemTime::UNIX_EPOCH,
                },
                result: Value::default(),
            },
        }
    }

    /// Get the unique identifier of the job.
    ///
    /// # Returns
    /// The ID of the job.
    pub fn id(&self) -> Uuid {
        self.id
    }

    /// Get the status of the job.
    ///
    /// # Returns
    /// The current status of the job.
    pub fn status(&self) -> Status {
        self.status
    }

    /// Set the status of the job.
    ///
    /// # Arguments
    /// * ̀`status` - Value to be set.
    ///
    /// # Errors
    /// One of `Error` enum.
    pub fn set_status(&mut self, status: Status) -> Result<(), Error> {
        match status {
            Status::NotReady => {
                return Err(Error::InvalidJobStatusTransition((self.status, status)))
            }

            Status::Ready => {
                if self.status != Status::NotReady {
                    return Err(Error::InvalidJobStatusTransition((self.status, status)));
                } else {
                    self.payload.timestamps.enqueued = SystemTime::now();
                }
            }

            Status::Running => {
                if self.status != Status::Ready {
                    return Err(Error::InvalidJobStatusTransition((self.status, status)));
                } else {
                    self.payload.timestamps.started = SystemTime::now();
                }
            }

            Status::Finished => {
                if self.status != Status::Running {
                    return Err(Error::InvalidJobStatusTransition((self.status, status)));
                } else {
                    self.payload.timestamps.finished = SystemTime::now();
                }
            }
        }

        self.status = status;

        Ok(())
    }

    /// Set the result of the job that will be stored as `serde_json::Value`.
    ///
    /// # Arguments
    /// * `value` - Serializable value to be stored.
    ///
    /// # Errors
    /// One of `Error` enum.
    pub fn set_result<T>(&mut self, value: T) -> Result<(), Error>
    where
        T: Serialize,
    {
        self.payload.result = serde_json::to_value(value)?;

        Ok(())
    }

    /// Call the underlying routine of the job.
    pub async fn run(&mut self) -> Result<(), Error> {
        let result = (self.routine)().await?;

        self.set_result(result)
    }
}
