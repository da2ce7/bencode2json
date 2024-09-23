//! The stack used by the Bencoded to JSON converter to keep track of the 
//! current parsing state.

// code-review: should we use a fixed array to avoid heap fragmentation?

#[derive(Debug)]
pub struct Stack {
    items: Vec<State>,
}

// todo: rename states

#[derive(Debug, PartialEq, Clone)]
pub enum State {
    Initial,
    // For lists
    L, // LIST (swap L -> M).
    M, // Put the delimiter (',') between list items.
    // For dictionaries
    D,
    E,
    F,
}

impl Default for Stack {
    fn default() -> Self {
        let items = vec![State::Initial];
        Self { items }
    }
}

impl Stack {
    pub fn push(&mut self, item: State) {
        self.items.push(item);
    }

    pub fn pop(&mut self) {
        // Panic if the top element is the initial state.
        self.items.pop();
    }

    pub fn swap_top(&mut self, new_item: State) {
        // Panic if the top element is the initial state.
        self.items.pop();
        self.push(new_item);
    }

    /// It returns the top element on the stack without removing it.
    ///
    /// # Panics
    ///
    /// Will panic is the stack is empty. The stack is never empty because it's
    /// not allowed to pop the initial state.
    #[must_use]
    pub fn peek(&self) -> State {
        match self.items.last() {
            Some(top) => top.clone(),
            None => panic!("empty stack!"),
        }
    }
}

#[cfg(test)]
mod tests {

    mod it_should {
        use crate::stack::{Stack, State};

        #[test]
        fn have_an_initial_state() {
            assert_eq!(Stack::default().peek(), State::Initial);
        }

        // todo: the rest of operations on the stack.
    }
}
