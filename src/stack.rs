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
        self.items.pop();
    }

    pub fn swap_top(&mut self, new_item: State) {
        self.items.pop();
        self.push(new_item);
    }

    /// It return the top element on the stack.
    ///
    /// The stack user should never pop if it's not sure there is an element.
    ///
    /// # Panics
    ///
    /// Will panic is the stack is empty.
    #[must_use]
    pub fn top(&self) -> State {
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
            assert_eq!(Stack::default().top(), State::Initial);
        }

        // todo: the rest of operations on the stack.
    }
}
