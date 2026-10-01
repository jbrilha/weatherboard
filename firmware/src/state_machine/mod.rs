use core::hash::Hash;
use heapless::{Vec, index_map::FnvIndexMap};

pub trait StateId: Copy + Eq + Hash {}
impl<T> StateId for T where T: Copy + Eq + Hash {}

#[derive(Debug)]
pub enum StateError {
    StatesFull,
    UnknownState,
    NoTransition,
}

pub struct Transition<T, E> {
    pub condition: fn() -> bool,
    pub pre: Option<fn() -> Result<(), E>>,
    pub next: Option<T>,
    pub post: Option<fn() -> Result<(), E>>,
}

impl<T, E> Transition<T, E> {
    pub fn new(
        condition: fn() -> bool,
        pre: Option<fn() -> Result<(), E>>,
        next: Option<T>,
        post: Option<fn() -> Result<(), E>>,
    ) -> Self {
        Self {
            condition,
            pre,
            next,
            post,
        }
    }
}

pub struct Node<T, E> {
    pub state: T,
    pub transitions: Vec<Transition<T, E>, 5>,
}

impl<T: StateId, E> Node<T, E> {
    pub fn new(state: T, transitions: Vec<Transition<T, E>, 5>) -> Self {
        Self { state, transitions }
    }

    pub fn set_transitions(&mut self, transitions: Vec<Transition<T, E>, 5>) {
        self.transitions = transitions;
    }

    pub fn add_transition(&mut self, transition: Transition<T, E>) -> Result<(), Transition<T, E>> {
        self.transitions.push(transition)
    }
}

pub struct StateMachine<T: StateId, E> {
    nodes: FnvIndexMap<T, Node<T, E>, 16>,
    current: T,
}

impl<T: StateId, E> StateMachine<T, E> {
    pub fn new(initial: Node<T, E>) -> Result<Self, StateError> {
        let current = initial.state;
        let mut nodes = FnvIndexMap::new();
        nodes
            .insert(current, initial)
            .map_err(|_| StateError::StatesFull)?;
        Ok(Self { nodes, current })
    }

    pub fn add_node(&mut self, node: Node<T, E>) -> Result<(), StateError> {
        self.nodes
            .insert(node.state, node)
            .map(|_| ())
            .map_err(|_| StateError::StatesFull)
    }

    pub fn current(&self) -> T {
        self.current
    }

    pub fn advance(&mut self) -> Result<T, StateError> {
        Ok(self.current)
    }
}
