use std::time::SystemTime;
use uuid::Uuid;

use crate::error::Error;

/// List of statuses of a job.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Status {
    /// Job is not running yet but ready to be started.
    Idle,

    /// Job is currently running.
    Running,

    /// Job is finished (on success or error).
    Finished,
}

/// Structure used to store timestamps and result of the job.
pub struct Payload {
    /// Timestamps for every step of the job.
    pub timestamps: Timestamps,
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

/// Description of a job.
pub struct Job {
    /// Unique identifier of the job.
    id: Uuid,

    /// The routine called when running.
    routine: Box<dyn Fn() + Sync + Send>,

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
    pub fn new(routine: Box<dyn Fn() + Sync + Send>) -> Self {
        Self {
            id: Uuid::now_v1(&[1, 2, 3, 4, 5, 6]),
            routine,
            status: Status::Idle,
            payload: Payload {
                timestamps: Timestamps {
                    enqueued: SystemTime::now(),
                    started: SystemTime::UNIX_EPOCH,
                    finished: SystemTime::UNIX_EPOCH,
                },
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
            Status::Idle => return Err(Error::InvalidJobStatusTransition((self.status, status))),

            Status::Running => {
                if self.status != Status::Idle {
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

    /// Call the underlying routine of the job.
    pub fn run(&self) {
        (self.routine)();
    }
}
