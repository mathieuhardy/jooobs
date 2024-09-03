pub mod backend;
pub mod error;
pub mod job;
pub mod job_queue;
pub mod memory_backend;
pub mod prelude;

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use serde_json::Value;
    use std::sync::Mutex;

    use crate::prelude::*;

    static FLAG: Mutex<bool> = Mutex::new(false);

    fn reset_flag() {
        set_flag(SetFlagArgs { value: false });
    }

    fn check_flag() {
        assert!(*FLAG.lock().unwrap());
    }

    fn set_flag(args: SetFlagArgs) {
        *FLAG.lock().unwrap() = args.value;
    }

    #[derive(Clone, Serialize, Deserialize)]
    struct SetFlagArgs {
        value: bool,
    }

    #[derive(Serialize, Deserialize)]
    enum Routines {
        SetFlag(SetFlagArgs),
    }

    #[async_trait]
    impl Routine for Routines {
        async fn call(&self) -> Result<Value, Error> {
            match self {
                Self::SetFlag(args) => {
                    set_flag(args.clone());
                    Ok(serde_json::json!({
                        "result": "SET_FLAG_OK",
                    }))
                }
            }
        }
    }

    #[tokio::test]
    async fn nominal() {
        reset_flag();

        // Create and start the job queue
        let mut jq = JobQueue::<Routines>::new(1, 8).unwrap();

        jq.start().unwrap();
        assert_eq!(jq.state(), State::Running);

        // Create the job and push it
        let routine = Routines::SetFlag(SetFlagArgs { value: true });
        let job = Job::new(routine).unwrap();
        let job_id = job.id();

        jq.enqueue(job).await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Verify that job has been processed
        check_flag();
        let result = jq.job_result(job_id).await.unwrap();
        assert_eq!(result["result"], "SET_FLAG_OK");
        // TODO: fetch result and check

        // Stop the job queue
        jq.stop().await.unwrap();
        jq.join().await.unwrap();
        assert_eq!(jq.state(), State::Stopped);
    }
}
