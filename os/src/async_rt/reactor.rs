use alloc::boxed::Box;
use alloc::collections::btree_map::BTreeMap;
use alloc::sync::Arc;
use core::mem;

use lazy_static::*;
use spin::Mutex;

use super::TaskId;

lazy_static! {
    pub static ref REACTOR: Arc<Mutex<Box<Reactor>>> = Reactor::new();
}

pub enum TaskState {
    Ready,
    NotReady,
    Finish,
}

pub struct Reactor {
    tasks: BTreeMap<TaskId, TaskState>,
}

impl Reactor {
    fn new() -> Arc<Mutex<Box<Self>>> {
        let reactor = Arc::new(Mutex::new(Box::new(Reactor {
            tasks: BTreeMap::new(),
        })));
        reactor
    }

    fn wake(&mut self, id: TaskId) {
        let state = self.tasks.get_mut(&id).unwrap();
        match mem::replace(state, TaskState::Ready) {
            TaskState::NotReady => (),
            TaskState::Finish => panic!("Called 'wake' twice on task: {:?}", id),
            _ => unreachable!()
        }
    }

    pub fn register(&mut self, id: TaskId) {
        if self.tasks.insert(id, TaskState::NotReady).is_some() {
            panic!("Tried to insert a task with id: '{:?}', twice!", id);
        }
    }

    pub(crate) fn is_ready(&self, id: TaskId) -> bool {
        self.tasks.get(&id).map(|state| match state {
            TaskState::Ready => true,
            _ => false,
        }).unwrap_or(false)
    }

    pub(crate) fn get_task(&self, task_id: TaskId) -> Option<&TaskState> {
        self.tasks.get(&task_id)
    }

    pub(crate) fn get_task_mut(&mut self, task_id: TaskId) -> Option<&mut TaskState> {
        self.tasks.get_mut(&task_id)
    }

    pub(crate) fn add_task(&mut self, task_id: TaskId) -> Option<TaskState> {
        self.tasks.insert(task_id, TaskState::NotReady)
    }

    pub(crate) fn contains_task(&self, task_id: TaskId) -> bool {
        self.tasks.contains_key(&task_id)
    }
}

