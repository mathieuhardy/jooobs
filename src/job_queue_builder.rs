use crate::prelude::*;

pub struct JobQueueBuilder<RoutineType, Context> {
    /// Job queue
    jq: JobQueue<RoutineType, Context>,
}

impl<RoutineType, Context> JobQueueBuilder<RoutineType, Context>
where
    RoutineType: Routine<Context> + Sync + 'static,
    Context: Send + 'static,
{
    /// Create a builder for the job queue.
    ///
    /// # Arguments:
    /// * `thread_pool_size` - Size of the thread pool.
    ///
    /// # Returns
    /// An instance of ̀`JobQueueBuilder`.
    ///
    /// # Errors
    /// One of `Error` enum.
    pub fn new(thread_pool_size: usize) -> Result<Self, ApiError> {
        Ok(Self {
            jq: JobQueue::<RoutineType, Context>::new(thread_pool_size)?,
        })
    }

    /// Set the backend to be used by the job queue.
    ///
    /// # Arguments:
    /// * `backend` - Instance to be set.
    ///
    /// # Returns
    /// An instance of ̀`JobQueueBuilder`.
    pub fn backend(self, backend: impl Backend<RoutineType, Context> + 'static) -> Self {
        let mut jq = self.jq;

        jq.set_backend(backend);

        Self { jq }
    }

    /// Set the notification handler to be used by the job queue.
    ///
    /// # Arguments:
    /// * `handler` - Instance to be set.
    ///
    /// # Returns
    /// An instance of ̀`JobQueueBuilder`.
    pub fn notification_handler(
        self,
        handler: impl Fn(Notification) + Send + Sync + 'static,
    ) -> Self {
        let mut jq = self.jq;

        jq.set_notification_handler(handler);

        Self { jq }
    }

    /// Set the context to be passed to every routine.
    ///
    /// # Arguments:
    /// * `context` - Instance to be set.
    ///
    /// # Returns
    /// An instance of ̀`JobQueueBuilder`.
    pub fn context(self, context: Context) -> Self {
        let mut jq = self.jq;

        jq.set_context(context);

        Self { jq }
    }

    /// Build the job queue consuming the current builder instance.
    ///
    /// # Returns
    /// An instance of ̀`JobQueue`.
    pub fn build(self) -> JobQueue<RoutineType, Context> {
        self.jq
    }
}
