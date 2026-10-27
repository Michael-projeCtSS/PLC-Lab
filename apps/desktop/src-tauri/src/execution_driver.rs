//! The sole host pacing exception. Wall-clock delays only pace cycles; they never become
//! simulation time. One cycle per wake, no catch-up.
use desktop_session::{DesktopApp, RunStart, TickOutcome};
use std::{future::Future, sync::Arc, time::Duration};
use tauri::Emitter;

/// What one paced step produced, as seen by the loop.
pub(crate) enum Step {
    /// Continue after waiting `next_period`.
    Continue(Duration),
    /// Stop pacing (paused, stale lease, failure or notification failure).
    Exit,
}

pub(crate) async fn paced_loop<W, WF, T, TF>(first_period: Duration, mut wait: W, mut step: T)
where
    W: FnMut(Duration) -> WF,
    WF: Future<Output = ()>,
    T: FnMut() -> TF,
    TF: Future<Output = Step>,
{
    let mut period = first_period;
    loop {
        // Wait before every cycle, including the first.
        wait(period).await;
        match step().await {
            Step::Continue(next) => period = next,
            Step::Exit => break,
        }
    }
}

pub fn spawn(app: tauri::AppHandle, service: Arc<DesktopApp>, start: RunStart) {
    let lease = start.lease;
    tauri::async_runtime::spawn(async move {
        paced_loop(
            Duration::from_micros(start.period_us),
            tokio::time::sleep,
            move || {
                let service = Arc::clone(&service);
                let app = app.clone();
                async move {
                    let worker = Arc::clone(&service);
                    let outcome =
                        tauri::async_runtime::spawn_blocking(move || worker.tick(lease)).await;
                    match outcome {
                        Ok(Ok(TickOutcome::Stale)) => Step::Exit,
                        Ok(Ok(TickOutcome::Advanced {
                            running,
                            next_period_us,
                            wake,
                        })) => {
                            if let Err(error) = app.emit(app_contract::EXECUTION_WAKE_EVENT, wake) {
                                service.driver_failed(
                                    lease,
                                    format!("Execution wake delivery failed: {error}"),
                                );
                                return Step::Exit;
                            }
                            if running {
                                Step::Continue(Duration::from_micros(next_period_us))
                            } else {
                                Step::Exit
                            }
                        }
                        Ok(Err(error)) => {
                            // The workspace already failed closed; tell the view to refresh once.
                            let _ = app.emit(app_contract::EXECUTION_WAKE_EVENT, lease.wake(0));
                            eprintln!("{}: {}", error.code, error.message);
                            Step::Exit
                        }
                        Err(_) => {
                            service.driver_failed(
                                lease,
                                "The simulation worker could not finish.".into(),
                            );
                            let _ = app.emit(app_contract::EXECUTION_WAKE_EVENT, lease.wake(0));
                            Step::Exit
                        }
                    }
                }
            },
        )
        .await;
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, future::ready};

    #[test]
    fn waits_before_each_step_follows_new_periods_and_exits() {
        let waits = RefCell::new(Vec::new());
        let steps = RefCell::new(0);
        tauri::async_runtime::block_on(paced_loop(
            Duration::from_millis(100),
            |period| {
                waits.borrow_mut().push(period.as_millis());
                ready(())
            },
            || {
                *steps.borrow_mut() += 1;
                ready(match *steps.borrow() {
                    1 => Step::Continue(Duration::from_millis(50)),
                    2 => Step::Continue(Duration::from_millis(200)),
                    _ => Step::Exit,
                })
            },
        ));
        assert_eq!(*waits.borrow(), [100, 50, 200]);
        assert_eq!(*steps.borrow(), 3);
    }
}
