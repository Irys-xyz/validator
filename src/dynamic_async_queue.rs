use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    task::{Poll, Waker},
};

use futures::Stream;
use log::{debug, warn};

#[derive(Debug)]
struct State<Item> {
    items: VecDeque<Item>,
    done: bool,
    waker: Option<Waker>,
}

impl<Item> State<Item> {
    pub fn add_items(&mut self, items: Vec<Item>) {
        if self.done {
            // TODO: think this through, maybe we need to panic here?
            warn!("Added items after marked as done, mark as not done");
            self.done = false;
        }
        debug!("add {} items", items.len());
        self.items.append(&mut items.into());
        if let Some(waker) = self.waker.take() {
            waker.wake()
        }
    }

    pub fn add_item(&mut self, item: Item) {
        if self.done {
            // TODO: think this through, maybe we need to panic here?
            warn!("Added items after marked as done, mark as not done");
            self.done = false;
        }
        self.items.push_back(item);
        if let Some(waker) = self.waker.take() {
            waker.wake()
        }
    }

    pub fn take(&mut self) -> Option<Item> {
        self.items.pop_front()
    }

    pub fn done_pending(&mut self) {
        debug!("done pending, number of items: {}", self.items.len());
        if self.items.is_empty() {
            self.done = true;
            if let Some(waker) = self.waker.take() {
                waker.wake()
            }
        }
    }
}

#[derive(Debug)]
pub struct DynamicAsyncQueue<T>(Arc<Mutex<State<T>>>);

impl<T> Clone for DynamicAsyncQueue<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> DynamicAsyncQueue<T> {
    pub fn new(init: Vec<T>) -> Self {
        let items = VecDeque::from(init);
        Self(Arc::new(Mutex::new(State {
            items,
            done: false,
            waker: None,
        })))
    }

    pub fn add_items(&self, items: Vec<T>) {
        debug!("Add {} items", items.len());
        self.0
            .lock()
            .expect("Failed to acquire lock")
            .add_items(items)
    }

    pub fn add_item(&self, item: T) {
        debug!("Add item");
        self.0
            .lock()
            .expect("Failed to acquire lock")
            .add_item(item)
    }

    pub fn all_pending_work_done(&self) {
        debug!("all pending work done");
        self.0
            .lock()
            .expect("Failed to acquire lock")
            .done_pending();
    }
}

impl<T> Stream for DynamicAsyncQueue<T> {
    type Item = T;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        ctx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let mut state = self.0.lock().unwrap(); // There's no way to recover from poisoned lock

        if let Some(item) = state.take() {
            debug!("poll, found some");
            Poll::Ready(Some(item))
        } else if state.done {
            debug!("poll, done");
            Poll::Ready(None)
        } else {
            debug!("poll, pending");
            state.waker = Some(ctx.waker().clone());
            Poll::Pending
        }
    }
}
