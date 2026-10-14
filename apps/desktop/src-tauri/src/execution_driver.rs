//! The sole host pacing exception. Delays never become domain time.
use desktop_session::{RunLease, RuntimeSessionService, TickOutcome};
use std::{future::Future, sync::Arc, time::Duration};
use tauri::Emitter;

async fn paced_loop<W, WF, T, TF, N>(period: Duration, mut wait: W, mut tick: T, mut notify: N)
where
    W: FnMut(Duration) -> WF,
    WF: Future<Output = ()>,
    T: FnMut() -> TF,
    TF: Future<Output = Result<TickOutcome, app_contract::IpcError>>,
    N: FnMut() -> bool,
{
    loop {
        // Wait before the first cycle and each subsequent cycle. No catch-up.
        wait(period).await;
        match tick().await {
            Ok(TickOutcome::Stale) => break,
            Ok(TickOutcome::Observed { running }) => {
                if !notify() || !running {
                    break;
                }
            }
            Err(error) => {
                eprintln!("{}: {}", error.code, error.message);
                notify();
                break;
            }
        }
    }
}

pub fn spawn(
    app: tauri::AppHandle,
    service: Arc<RuntimeSessionService>,
    lease: RunLease,
    period_micros: u64,
) {
    tauri::async_runtime::spawn(async move {
        let tick_service = Arc::clone(&service);
        paced_loop(
            Duration::from_micros(period_micros),
            tokio::time::sleep,
            move || {
                let service = Arc::clone(&tick_service);
                let failure_service = Arc::clone(&tick_service);
                async move {
                    match tauri::async_runtime::spawn_blocking(move || {
                        service.continuous_tick(lease)
                    })
                    .await
                    {
                        Ok(result) => result,
                        Err(_) => {
                            let error = app_contract::IpcError {
                                code: "APP-EXECUTION-WORKER".into(),
                                message: "The continuous execution worker could not finish.".into(),
                            };
                            tick_service_failure(&failure_service, lease, error.clone());
                            Err(error)
                        }
                    }
                }
            },
            || {
                if let Err(error) = app.emit(app_contract::EXECUTION_WAKE_EVENT, lease.wake()) {
                    eprintln!("APP-EXECUTION-NOTIFICATION: {error}");
                    tick_service_failure(
                        &service,
                        lease,
                        app_contract::IpcError {
                            code: "APP-EXECUTION-NOTIFICATION".into(),
                            message: "Execution wake delivery failed. Reload the program.".into(),
                        },
                    );
                    return false;
                }
                true
            },
        )
        .await;
    });
}

fn tick_service_failure(
    service: &RuntimeSessionService,
    lease: RunLease,
    error: app_contract::IpcError,
) {
    if let Err(failure) = service.driver_failed(lease, error) {
        eprintln!("{}: {}", failure.code, failure.message);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, future::ready};
    #[test]
    fn waits_before_each_tick_one_cycle_per_wake_and_exits_on_stale() {
        let events = RefCell::new(Vec::new());
        let attempts = RefCell::new(0);
        tauri::async_runtime::block_on(paced_loop(
            Duration::from_millis(100),
            |period| {
                assert_eq!(period, Duration::from_millis(100));
                events.borrow_mut().push("wait");
                ready(())
            },
            || {
                events.borrow_mut().push("tick");
                *attempts.borrow_mut() += 1;
                ready(Ok(if *attempts.borrow() == 3 {
                    TickOutcome::Stale
                } else {
                    TickOutcome::Observed { running: true }
                }))
            },
            || {
                events.borrow_mut().push("notify");
                true
            },
        ));
        assert_eq!(
            *events.borrow(),
            [
                "wait", "tick", "notify", "wait", "tick", "notify", "wait", "tick"
            ]
        );
    }
    #[test]
    fn fault_or_application_failure_notifies_once_and_never_retries() {
        for application_failure in [false, true] {
            let ticks = RefCell::new(0);
            let notices = RefCell::new(0);
            tauri::async_runtime::block_on(paced_loop(
                Duration::from_millis(100),
                |_| ready(()),
                || {
                    *ticks.borrow_mut() += 1;
                    ready(if application_failure {
                        Err(app_contract::IpcError {
                            code: "APP-TEST".into(),
                            message: "Injected host failure".into(),
                        })
                    } else {
                        Ok(TickOutcome::Observed { running: false })
                    })
                },
                || {
                    *notices.borrow_mut() += 1;
                    true
                },
            ));
            assert_eq!(*ticks.borrow(), 1);
            assert_eq!(*notices.borrow(), 1);
        }
    }
}
