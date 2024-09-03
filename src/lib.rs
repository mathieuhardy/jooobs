pub mod backend;
pub mod error;
pub mod job;
pub mod job_queue;
pub mod memory_backend;
pub mod prelude;

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use tokio::sync::Mutex;

    use crate::prelude::*;

    macro_rules! toggle {
        () => {{
            let value = Arc::new(Mutex::new(false));
            let cloned = value.clone();
            (value, cloned)
        }};
    }

    macro_rules! set_toggle {
        ($ident: expr) => {{
            *$ident.lock().await = true;
        }};
    }

    macro_rules! check_toggle {
        ($ident: expr) => {{
            assert!(*$ident.lock().await);
        }};
    }

    #[tokio::test]
    async fn nominal() {
        let mut jq = JobQueue::new(1, 8).unwrap();
        let (t, t2) = toggle!();

        jq.start().unwrap();
        assert_eq!(jq.state(), State::Running);

        let routine = Box::new(|| {
            let routine_output = Box::pin(async move {
                set_toggle!(t2);

                Ok(serde_json::Value::default())
            });

            routine_output
                as std::pin::Pin<Box<dyn std::future::Future<Output = RoutineResult> + Send>>
        });

        jq.enqueue(Job::new(routine)).await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        check_toggle!(t);

        jq.stop().await.unwrap();
        jq.join().await.unwrap();
        assert_eq!(jq.state(), State::Stopped);
    }
}
