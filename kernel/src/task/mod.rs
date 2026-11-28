// SPDX-License-Identifier: Apache-2.0
//! Bounded single-core task lifecycle. Callers serialize access to the table.

/// A slot and its lifetime. Exhausted generations permanently retire a slot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TaskId {
    slot: usize,
    generation: u32,
}

/// Logical execution state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum State {
    Ready,
    Running,
    Terminated,
}

/// Failures leave the table unchanged.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Full,
    UnknownTask,
    InvalidState,
}

/// Task metadata and an owned, caller-supplied context.
pub struct Task<T> {
    pub name: &'static str,
    state: State,
    pub context: T,
}

impl<T> Task<T> {
    pub fn state(&self) -> State {
        self.state
    }
}

struct Slot<T> {
    generation: u32,
    task: Option<Task<T>>,
}

/// The default capacity is eight slots. The idle context lives outside the table.
pub struct Table<T, const N: usize = 8> {
    slots: [Slot<T>; N],
    current: Option<TaskId>,
}

impl<T, const N: usize> Default for Table<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const N: usize> Table<T, N> {
    pub const fn new() -> Self {
        Self {
            slots: [const {
                Slot {
                    generation: 0,
                    task: None,
                }
            }; N],
            current: None,
        }
    }

    /// Return the context to the caller if no slot can accept it.
    pub fn create(&mut self, name: &'static str, context: T) -> Result<TaskId, (Error, T)> {
        for (slot, entry) in self.slots.iter_mut().enumerate() {
            if entry.task.is_none()
                && let Some(generation) = entry.generation.checked_add(1)
            {
                entry.generation = generation;
                entry.task = Some(Task {
                    name,
                    state: State::Ready,
                    context,
                });
                return Ok(TaskId { slot, generation });
            }
        }
        Err((Error::Full, context))
    }

    pub fn current(&self) -> Option<TaskId> {
        self.current
    }

    pub fn get(&self, id: TaskId) -> Result<&Task<T>, Error> {
        self.slots
            .get(id.slot)
            .filter(|s| s.generation == id.generation)
            .and_then(|s| s.task.as_ref())
            .ok_or(Error::UnknownTask)
    }

    pub fn context_mut(&mut self, id: TaskId) -> Result<&mut T, Error> {
        self.get(id)?;
        Ok(&mut self.slots[id.slot].task.as_mut().unwrap().context)
    }

    /// Pick a ready task in slot order after the current task, wrapping once.
    pub fn next_ready(&self) -> Option<TaskId> {
        let start = self.current.map_or(0, |id| id.slot + 1);
        (start..N).chain(0..start).find_map(|slot| {
            let entry = &self.slots[slot];
            (entry.task.as_ref()?.state == State::Ready).then_some(TaskId {
                slot,
                generation: entry.generation,
            })
        })
    }

    /// Commit a handoff on a separate stack. Exiting without a target leaves
    /// the table without a running task. A yield with no target keeps running.
    pub fn handoff(&mut self, next: Option<TaskId>, exiting: bool) -> Result<(), Error> {
        if exiting && self.current.is_none() {
            return Err(Error::InvalidState);
        }
        if let Some(id) = next {
            if self.get(id)?.state != State::Ready {
                return Err(Error::InvalidState);
            }
        } else if !exiting {
            return Ok(());
        }
        if let Some(id) = self.current {
            self.slots[id.slot].task.as_mut().unwrap().state = if exiting {
                State::Terminated
            } else {
                State::Ready
            };
        }
        self.current = next;
        if let Some(id) = next {
            self.slots[id.slot].task.as_mut().unwrap().state = State::Running;
        }
        Ok(())
    }

    /// The caller must ensure the terminated task's stack is no longer live.
    pub fn reap(&mut self, id: TaskId) -> Result<T, Error> {
        if self.get(id)?.state != State::Terminated {
            return Err(Error::InvalidState);
        }
        Ok(self.slots[id.slot].task.take().unwrap().context)
    }
}
