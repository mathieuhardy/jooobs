use crate::prelude::*;

pub struct JobQueueBuilder<RoutineType> {
    /// Job queue
    jq: JobQueue<RoutineType>,
}

impl<RoutineType> JobQueueBuilder<RoutineType>
where
    RoutineType: Routine + Sync + 'static,
{
    pub fn new(thread_pool_size: usize) -> Result<Self, ApiError> {
        Ok(Self {
            jq: JobQueue::<RoutineType>::new(thread_pool_size)?,
        })
    }

    pub fn backend(self, backend: impl Backend<RoutineType> + 'static) -> Self {
        let mut jq = self.jq;

        jq.set_backend(backend);

        Self { jq }
    }

    pub fn notification_handler(
        self,
        handler: impl Fn(Notification) + Send + Sync + 'static,
    ) -> Self {
        let mut jq = self.jq;

        jq.set_notification_handler(handler);

        Self { jq }
    }

    pub fn build(self) -> JobQueue<RoutineType> {
        self.jq
    }
}
