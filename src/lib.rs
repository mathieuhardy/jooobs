pub mod backend;
pub mod error;
pub mod job;
pub mod job_queue;
pub mod memory_backend;
pub mod prelude;

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
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
    impl AsyncRoutine for Routines {
        async fn call(&self) {
            match self {
                Self::SetFlag(args) => set_flag(args.clone()),
            }
        }
    }

    #[tokio::test]
    async fn nominal() {
        reset_flag();

        let mut jq = JobQueue::<Routines>::new(1, 8).unwrap();

        jq.start().unwrap();
        assert_eq!(jq.state(), State::Running);

        let routine = Routines::SetFlag(SetFlagArgs { value: true });

        jq.enqueue(Job::new(routine)).await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        check_flag();

        jq.stop().await.unwrap();
        jq.join().await.unwrap();
        assert_eq!(jq.state(), State::Stopped);
    }
}
