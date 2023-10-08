use super::context::Context;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum State {
    Uninit,
    Ready,
    Running,
    Exited,
}

impl Default for State {
    fn default() -> Self {
        Self::Uninit
    }
}

#[derive(Default, Clone, Copy)]
pub struct TaskControlBlock {
    pub context: Context,
    pub state: State,
}
