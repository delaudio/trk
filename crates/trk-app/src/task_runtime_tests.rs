use std::{sync::mpsc, time::Duration};

use super::*;

fn unused_job<T: Send + 'static>() -> TaskJob<T> {
    Box::new(|_| panic!("fake backend must not run jobs"))
}

#[test]
fn fake_backend_drives_progress_and_completion_deterministically() {
    let (backend, controller) = FakeTaskBackend::new();
    let mut runtime = TaskRuntime::<u32>::with_backend(backend);
    let id = runtime.submit("render", unused_job());
    controller.push(TaskUpdate::Started { id });
    controller.push(TaskUpdate::Progress {
        id,
        progress: TaskProgress::new(1, Some(2), "half"),
    });
    controller.push(TaskUpdate::Completed { id, result: 42 });

    let updates = runtime.drain_updates();

    assert_eq!(updates.len(), 3);
    let task = runtime.task(id).expect("task");
    assert_eq!(task.status, TaskStatus::Completed);
    assert_eq!(
        task.progress.as_ref().and_then(TaskProgress::percentage),
        Some(50)
    );
}

#[test]
fn cancellation_is_idempotent_and_rejects_stale_completion() {
    let (backend, controller) = FakeTaskBackend::new();
    let mut runtime = TaskRuntime::<u32>::with_backend(backend);
    let id = runtime.submit("render", unused_job());

    assert!(runtime.cancel(id));
    assert!(!runtime.cancel(id));
    assert!(controller.was_cancelled(id));
    controller.push(TaskUpdate::Completed { id, result: 42 });
    controller.push(TaskUpdate::Cancelled { id });

    let updates = runtime.drain_updates();
    assert_eq!(updates.len(), 1);
    assert!(matches!(updates[0], TaskUpdate::Cancelled { .. }));
    assert_eq!(
        runtime.task(id).expect("task").status,
        TaskStatus::Cancelled
    );
}

#[test]
fn failure_preserves_all_diagnostics() {
    let (backend, controller) = FakeTaskBackend::new();
    let mut runtime = TaskRuntime::<()>::with_backend(backend);
    let id = runtime.submit("index", unused_job());
    controller.push(TaskUpdate::Failed {
        id,
        diagnostics: vec![
            TaskDiagnostic::error("cannot read root"),
            TaskDiagnostic::error("index incomplete"),
        ],
    });

    runtime.drain_updates();

    let task = runtime.task(id).expect("task");
    assert_eq!(task.status, TaskStatus::Failed);
    assert_eq!(task.diagnostics.len(), 2);
}

#[test]
fn worker_panic_becomes_a_failed_task_diagnostic() {
    let mut runtime = TaskRuntime::<()>::default();
    let id = runtime.submit("panic", Box::new(|_| panic!("provider crashed")));

    for _ in 0..100 {
        runtime.drain_updates();
        if runtime.task(id).expect("task").status.is_terminal() {
            break;
        }
        thread::sleep(Duration::from_millis(1));
    }

    let task = runtime.task(id).expect("task");
    assert_eq!(task.status, TaskStatus::Failed);
    assert_eq!(
        task.diagnostics[0].message,
        "task panicked: provider crashed"
    );
}

#[test]
fn thread_backend_submit_does_not_wait_for_job_completion() {
    let mut runtime = TaskRuntime::<u32>::default();
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let id = runtime.submit(
        "blocked test job",
        Box::new(move |_| {
            entered_tx.send(()).expect("entered");
            release_rx.recv().expect("release");
            Ok(7)
        }),
    );

    entered_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("job runs on worker thread");
    assert!(!runtime.task(id).expect("task").status.is_terminal());
    release_tx.send(()).expect("release job");
    for _ in 0..100 {
        if runtime
            .drain_updates()
            .iter()
            .any(|update| matches!(update, TaskUpdate::Completed { .. }))
        {
            break;
        }
        thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(
        runtime.task(id).expect("task").status,
        TaskStatus::Completed
    );
}

#[test]
fn thread_backend_cancellation_does_not_wait_for_worker_acknowledgement() {
    let mut runtime = TaskRuntime::<()>::default();
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let id = runtime.submit(
        "uncooperative test job",
        Box::new(move |_| {
            entered_tx.send(()).expect("entered");
            release_rx.recv().expect("release");
            Ok(())
        }),
    );

    entered_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("job runs on worker thread");
    assert!(runtime.cancel(id));
    assert!(runtime
        .drain_updates()
        .iter()
        .any(|update| matches!(update, TaskUpdate::Cancelled { .. })));
    assert_eq!(
        runtime.task(id).expect("task").status,
        TaskStatus::Cancelled
    );

    release_tx.send(()).expect("release job");
}
