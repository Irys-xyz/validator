use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    task::{Poll, Waker},
};

use futures::Stream;
use log::debug;

#[derive(Debug)]
struct State<Item> {
    resources: VecDeque<Item>,
    waker: Option<Waker>,
}

impl<Item> State<Item> {
    pub fn release(&mut self, item: Item) {
        self.resources.push_back(item);
        if let Some(waker) = self.waker.take() {
            waker.wake()
        }
    }

    pub fn acquire(&mut self) -> Option<Item> {
        self.resources.pop_front()
    }
}

#[derive(Debug)]
pub struct Pool<T>(Arc<Mutex<State<T>>>);

impl<T> Clone for Pool<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> Pool<T> {
    pub fn new(resources: Vec<T>) -> Self {
        Self(Arc::new(Mutex::new(State {
            resources: VecDeque::from(resources),
            waker: None,
        })))
    }

    pub fn release(&self, item: T) {
        self.0.lock().expect("Failed to acquire lock").release(item)
    }
}

impl<T> Stream for Pool<T> {
    type Item = T;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        ctx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let mut state = self.0.lock().unwrap(); // There's no way to recover from poisoned lock

        if let Some(item) = state.acquire() {
            Poll::Ready(Some(item))
        } else {
            state.waker = Some(ctx.waker().clone());
            Poll::Pending
        }
    }
}
