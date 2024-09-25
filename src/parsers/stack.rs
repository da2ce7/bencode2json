//! The stack used by the Bencoded to JSON converter to keep track of the
//! current parsing state.

// code-review: should we use a fixed array to avoid heap fragmentation?

use std::fmt::Display;

#[derive(Debug)]
pub struct Stack {
    items: Vec<State>,
}

/// States while parsing list or dictionaries.
///
/// There are no states for integers and strings because it's a straightforward
/// operation. We know when they finish and there is no recursion.
///
/// States are displayed with a short name using only one letter:
///
/// I, L, M, D, E, F
///
/// This comes from the original implementation.
#[derive(Debug, PartialEq, Clone)]
pub enum State {
    Initial, // I

    // States while parsing lists
    ExpectingFirstListItemOrEnd, // L
    ExpectingNextListItem,       // M

    // States while parsing dictionaries
    ExpectingFirstDictFieldOrEnd, // D
    ExpectingDictFieldValue,      // E
    ExpectingDictFieldKey,        // F
}

impl Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let output = match self {
            State::Initial => "I",
            State::ExpectingFirstListItemOrEnd => "L",
            State::ExpectingNextListItem => "M",
            State::ExpectingFirstDictFieldOrEnd => "D",
            State::ExpectingDictFieldValue => "E",
            State::ExpectingDictFieldKey => "F",
        };
        write!(f, "{output}")
    }
}

impl Default for Stack {
    fn default() -> Self {
        let items = vec![State::Initial];
        Self { items }
    }
}

impl Display for Stack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;
        for (idx, item) in <std::vec::Vec<State> as Clone>::clone(&self.items)
            .into_iter()
            .enumerate()
        {
            if idx > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{item}")?;
        }
        write!(f, "]")?;
        Ok(())
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
        use crate::parsers::stack::{Stack, State};

        #[test]
        fn have_an_initial_state() {
            assert_eq!(Stack::default().peek(), State::Initial);
        }

        // todo: the rest of operations on the stack.
    }
}
