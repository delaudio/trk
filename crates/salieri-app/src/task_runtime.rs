use std::{
    any::Any,
    collections::{BTreeMap, HashMap},
    fmt,
    panic::{catch_unwind, AssertUnwindSafe},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
        Arc,
    },
    thread,
};

#[cfg(test)]
use std::{
    collections::{HashSet, VecDeque},
    sync::Mutex,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TaskId(u64);

impl TaskId {
    pub fn from_raw(value: u64) -> Self {
        Self(value)
    }
}

impl fmt::Display for TaskId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    Queued,
    Running,
    Cancelling,
    Completed,
    Failed,
    Cancelled,
}

impl TaskStatus {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

impl fmt::Display for TaskStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Cancelling => "cancelling",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        };
        formatter.write_str(label)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskProgress {
    pub completed: u64,
    pub total: Option<u64>,
    pub message: Option<String>,
    pub phase: Option<String>,
    pub tool: Option<String>,
}

impl TaskProgress {
    pub fn new(completed: u64, total: Option<u64>, message: impl Into<String>) -> Self {
        Self {
            completed,
            total,
            message: Some(message.into()),
            phase: None,
            tool: None,
        }
    }

    pub fn with_phase(mut self, phase: impl Into<String>) -> Self {
        self.phase = Some(phase.into());
        self
    }

    pub fn with_tool(mut self, tool: impl Into<String>) -> Self {
        self.tool = Some(tool.into());
        self
    }

    pub fn percentage(&self) -> Option<u64> {
        self.total
            .filter(|total| *total > 0)
            .map(|total| self.completed.min(total).saturating_mul(100) / total)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskDiagnostic {
    pub code: Option<String>,
    pub message: String,
}

impl TaskDiagnostic {
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            code: None,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskFailure {
    pub diagnostics: Vec<TaskDiagnostic>,
}

impl TaskFailure {
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            diagnostics: vec![TaskDiagnostic::error(message)],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskSnapshot {
    pub id: TaskId,
    pub name: String,
    pub status: TaskStatus,
    pub progress: Option<TaskProgress>,
    pub diagnostics: Vec<TaskDiagnostic>,
}

#[derive(Debug)]
pub enum TaskUpdate<T> {
    Started {
        id: TaskId,
    },
    Progress {
        id: TaskId,
        progress: TaskProgress,
    },
    Completed {
        id: TaskId,
        result: T,
    },
    Failed {
        id: TaskId,
        diagnostics: Vec<TaskDiagnostic>,
    },
    Cancelled {
        id: TaskId,
    },
}

impl<T> TaskUpdate<T> {
    pub fn id(&self) -> TaskId {
        match self {
            Self::Started { id }
            | Self::Progress { id, .. }
            | Self::Completed { id, .. }
            | Self::Failed { id, .. }
            | Self::Cancelled { id } => *id,
        }
    }
}

pub struct TaskContext<T> {
    id: TaskId,
    cancelled: Arc<AtomicBool>,
    update_tx: Sender<TaskUpdate<T>>,
}

impl<T> TaskContext<T> {
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }

    pub fn report_progress(&self, progress: TaskProgress) -> bool {
        !self.is_cancelled()
            && self
                .update_tx
                .send(TaskUpdate::Progress {
                    id: self.id,
                    progress,
                })
                .is_ok()
    }

    pub fn check_cancelled(&self) -> Result<(), TaskFailure> {
        if self.is_cancelled() {
            Err(TaskFailure::error("task cancelled"))
        } else {
            Ok(())
        }
    }
}

pub type TaskJob<T> = Box<dyn FnOnce(TaskContext<T>) -> Result<T, TaskFailure> + Send + 'static>;

pub trait TaskBackend<T> {
    fn spawn(&mut self, id: TaskId, job: TaskJob<T>);
    fn cancel(&mut self, id: TaskId);
    fn try_recv(&mut self) -> Option<TaskUpdate<T>>;
    fn forget(&mut self, id: TaskId);
}

pub struct ThreadTaskBackend<T> {
    update_tx: Sender<TaskUpdate<T>>,
    update_rx: Receiver<TaskUpdate<T>>,
    cancellations: HashMap<TaskId, Arc<AtomicBool>>,
}

impl<T> Default for ThreadTaskBackend<T> {
    fn default() -> Self {
        let (update_tx, update_rx) = mpsc::channel();
        Self {
            update_tx,
            update_rx,
            cancellations: HashMap::new(),
        }
    }
}

impl<T: Send + 'static> TaskBackend<T> for ThreadTaskBackend<T> {
    fn spawn(&mut self, id: TaskId, job: TaskJob<T>) {
        let cancelled = Arc::new(AtomicBool::new(false));
        self.cancellations.insert(id, Arc::clone(&cancelled));
        let update_tx = self.update_tx.clone();
        thread::spawn(move || {
            if cancelled.load(Ordering::Acquire) {
                let _ = update_tx.send(TaskUpdate::Cancelled { id });
                return;
            }
            let _ = update_tx.send(TaskUpdate::Started { id });
            let context = TaskContext {
                id,
                cancelled: Arc::clone(&cancelled),
                update_tx: update_tx.clone(),
            };
            let result = catch_unwind(AssertUnwindSafe(|| job(context)));
            let update = if cancelled.load(Ordering::Acquire) {
                TaskUpdate::Cancelled { id }
            } else {
                match result {
                    Ok(Ok(result)) => TaskUpdate::Completed { id, result },
                    Ok(Err(failure)) => TaskUpdate::Failed {
                        id,
                        diagnostics: failure.diagnostics,
                    },
                    Err(payload) => TaskUpdate::Failed {
                        id,
                        diagnostics: vec![panic_diagnostic(payload)],
                    },
                }
            };
            let _ = update_tx.send(update);
        });
    }

    fn cancel(&mut self, id: TaskId) {
        if let Some(cancelled) = self.cancellations.get(&id) {
            cancelled.store(true, Ordering::Release);
            let _ = self.update_tx.send(TaskUpdate::Cancelled { id });
        }
    }

    fn try_recv(&mut self) -> Option<TaskUpdate<T>> {
        self.update_rx.try_recv().ok()
    }

    fn forget(&mut self, id: TaskId) {
        self.cancellations.remove(&id);
    }
}

fn panic_diagnostic(payload: Box<dyn Any + Send>) -> TaskDiagnostic {
    let detail = payload
        .downcast_ref::<&str>()
        .map(|message| (*message).to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "unknown panic payload".to_string());
    TaskDiagnostic::error(format!("task panicked: {detail}"))
}

pub struct TaskRuntime<T> {
    next_id: u64,
    backend: Box<dyn TaskBackend<T>>,
    tasks: BTreeMap<TaskId, TaskSnapshot>,
}

impl<T> fmt::Debug for TaskRuntime<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TaskRuntime")
            .field("next_id", &self.next_id)
            .field("tasks", &self.tasks)
            .finish_non_exhaustive()
    }
}

impl<T: Send + 'static> Default for TaskRuntime<T> {
    fn default() -> Self {
        Self::with_backend(ThreadTaskBackend::default())
    }
}

impl<T: 'static> TaskRuntime<T> {
    pub fn with_backend(backend: impl TaskBackend<T> + 'static) -> Self {
        Self {
            next_id: 1,
            backend: Box::new(backend),
            tasks: BTreeMap::new(),
        }
    }

    pub fn submit(&mut self, name: impl Into<String>, job: TaskJob<T>) -> TaskId {
        let id = TaskId(self.next_id);
        self.next_id = self
            .next_id
            .checked_add(1)
            .expect("task ID space exhausted");
        self.tasks.insert(
            id,
            TaskSnapshot {
                id,
                name: name.into(),
                status: TaskStatus::Queued,
                progress: None,
                diagnostics: Vec::new(),
            },
        );
        self.backend.spawn(id, job);
        id
    }

    pub fn cancel(&mut self, id: TaskId) -> bool {
        let Some(task) = self.tasks.get_mut(&id) else {
            return false;
        };
        if task.status.is_terminal() || task.status == TaskStatus::Cancelling {
            return false;
        }
        task.status = TaskStatus::Cancelling;
        self.backend.cancel(id);
        true
    }

    pub fn drain_updates(&mut self) -> Vec<TaskUpdate<T>> {
        let mut accepted = Vec::new();
        while let Some(update) = self.backend.try_recv() {
            if self.apply_update(&update) {
                if matches!(
                    update,
                    TaskUpdate::Completed { .. }
                        | TaskUpdate::Failed { .. }
                        | TaskUpdate::Cancelled { .. }
                ) {
                    self.backend.forget(update.id());
                }
                accepted.push(update);
            }
        }
        accepted
    }

    pub fn task(&self, id: TaskId) -> Option<&TaskSnapshot> {
        self.tasks.get(&id)
    }

    pub fn tasks(&self) -> impl DoubleEndedIterator<Item = &TaskSnapshot> {
        self.tasks.values()
    }

    pub fn is_idle(&self) -> bool {
        self.tasks.values().all(|task| task.status.is_terminal())
    }

    fn apply_update(&mut self, update: &TaskUpdate<T>) -> bool {
        let Some(task) = self.tasks.get_mut(&update.id()) else {
            return false;
        };
        if task.status.is_terminal() {
            return false;
        }
        match update {
            TaskUpdate::Started { .. } if task.status == TaskStatus::Queued => {
                task.status = TaskStatus::Running;
            }
            TaskUpdate::Progress { progress, .. } if task.status == TaskStatus::Running => {
                task.progress = Some(progress.clone());
            }
            TaskUpdate::Completed { .. }
                if matches!(task.status, TaskStatus::Queued | TaskStatus::Running) =>
            {
                task.status = TaskStatus::Completed;
            }
            TaskUpdate::Failed { diagnostics, .. }
                if matches!(task.status, TaskStatus::Queued | TaskStatus::Running) =>
            {
                task.status = TaskStatus::Failed;
                task.diagnostics = diagnostics.clone();
            }
            TaskUpdate::Cancelled { .. } => {
                task.status = TaskStatus::Cancelled;
            }
            _ => return false,
        }
        true
    }
}

#[cfg(test)]
struct FakeTaskState<T> {
    updates: VecDeque<TaskUpdate<T>>,
    spawned: Vec<TaskId>,
    cancelled: HashSet<TaskId>,
}

#[cfg(test)]
impl<T> Default for FakeTaskState<T> {
    fn default() -> Self {
        Self {
            updates: VecDeque::new(),
            spawned: Vec::new(),
            cancelled: HashSet::new(),
        }
    }
}

#[cfg(test)]
pub struct FakeTaskBackend<T> {
    state: Arc<Mutex<FakeTaskState<T>>>,
}

#[cfg(test)]
#[derive(Clone)]
pub struct FakeTaskController<T> {
    state: Arc<Mutex<FakeTaskState<T>>>,
}

#[cfg(test)]
impl<T> FakeTaskBackend<T> {
    pub fn new() -> (Self, FakeTaskController<T>) {
        let state = Arc::new(Mutex::new(FakeTaskState::default()));
        (
            Self {
                state: Arc::clone(&state),
            },
            FakeTaskController { state },
        )
    }
}

#[cfg(test)]
impl<T> FakeTaskController<T> {
    pub fn push(&self, update: TaskUpdate<T>) {
        self.state
            .lock()
            .expect("fake task lock")
            .updates
            .push_back(update);
    }

    pub fn was_cancelled(&self, id: TaskId) -> bool {
        self.state
            .lock()
            .expect("fake task lock")
            .cancelled
            .contains(&id)
    }
}

#[cfg(test)]
impl<T> TaskBackend<T> for FakeTaskBackend<T> {
    fn spawn(&mut self, id: TaskId, _job: TaskJob<T>) {
        self.state.lock().expect("fake task lock").spawned.push(id);
    }

    fn cancel(&mut self, id: TaskId) {
        self.state
            .lock()
            .expect("fake task lock")
            .cancelled
            .insert(id);
    }

    fn try_recv(&mut self) -> Option<TaskUpdate<T>> {
        self.state
            .lock()
            .expect("fake task lock")
            .updates
            .pop_front()
    }

    fn forget(&mut self, _id: TaskId) {}
}

#[cfg(test)]
#[path = "task_runtime_tests.rs"]
mod tests;
