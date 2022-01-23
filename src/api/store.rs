use std::{
    convert::Infallible,
    marker::Sync,
    sync::{Arc, RwLock},
};
use warp::Filter;

#[derive(Clone)]
pub struct Store<T: Clone + Sync + Send> {
    pub inner: Arc<RwLock<T>>,
}

impl<T: Clone + Default + Sync + Send> Default for Store<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone + Default + Sync + Send> Store<T> {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(T::default())),
        }
    }
}

impl<T: Clone + Sync + Send> Store<T> {
    pub fn from_t(t: T) -> Self {
        Self {
            inner: Arc::new(RwLock::new(t)),
        }
    }

    pub fn into_warp(self) -> impl Filter<Extract = (Self,), Error = Infallible> + Clone {
        warp::any().map(move || self.clone())
    }
}
