// Uses
use crate::{job::Job, workerpool::Queue};
use std::sync::{mpsc, Arc, Mutex};
use warp::{reject::Reject, Filter, Rejection, Reply};

// Pub uses
pub use routes as handlers;
pub use store::Store;

//
pub type Sender<T> = Arc<Mutex<mpsc::Sender<T>>>;

//
#[derive(Debug, Clone)]
pub enum ApiMessageType {
    Job(String),
    Elastic(uuid::Uuid),
}

// Functions
pub fn routes(
    current_job: &Store<Option<Job>>,
    message_queue: &Queue<ApiMessageType>,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    // Job
    let job_get = warp::path!("job")
        .and(current_job.clone().into_warp())
        .and(warp::get())
        .and_then(handlers::job::get);
    let job_post = warp::path!("job")
        .and(message_queue.clone().into_warp())
        .and(job_post_body())
        .and(warp::post())
        .and_then(handlers::job::post);
    let job_delete = warp::path!("job")
        .and(current_job.clone().into_warp())
        .and(warp::delete())
        .and_then(handlers::job::delete);
    // Elastic
    let elastic_post = warp::path!("elastic")
        .and(message_queue.clone().into_warp())
        .and(job_post_body())
        .and(warp::post())
        .and_then(handlers::job::post); // Reuse same route
    job_get.or(job_post).or(job_delete).or(elastic_post)
}

#[derive(Debug)]
pub struct CustomError(pub String);
impl Reject for CustomError {}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct PostString(pub String);

pub fn job_post_body() -> impl Filter<Extract = (PostString,), Error = warp::Rejection> + Clone {
    // When accepting a body, we want a JSON body
    // (and to reject huge payloads)...
    warp::body::content_length_limit(1024 * 16).and(warp::body::json())
}

mod routes {
    // Uses
    use super::*;
    // use std::sync::{mpsc, Mutex};
    use crate::job::Job;
    use warp::{http::StatusCode, Rejection, Reply};

    // Job handlers
    pub mod job {
        use super::*;
        pub async fn get(store: Store<Option<Job>>) -> Result<Box<dyn Reply>, Rejection> {
            match store.inner.read() {
                Err(e) => Err(warp::reject::custom(crate::api::CustomError(e.to_string()))),
                Ok(opt) => match &*opt {
                    Some(job) => Ok(Box::new(warp::reply::json(job))),
                    None => Ok(Box::new(StatusCode::NO_CONTENT)),
                },
            }
        }
        pub async fn post(
            queue: Queue<ApiMessageType>,
            path: PostString,
        ) -> Result<Box<dyn Reply>, Rejection> {
            match uuid::Uuid::parse_str(&path.0) {
                Ok(uuid) => queue.push(ApiMessageType::Elastic(uuid)),
                Err(_) => queue.push(ApiMessageType::Job(path.0)),
            }
            Ok(Box::new(StatusCode::OK))
        }
        pub async fn delete(store: Store<Option<Job>>) -> Result<Box<dyn Reply>, Rejection> {
            match store.inner.write() {
                Err(e) => Err(warp::reject::custom(crate::api::CustomError(e.to_string()))),
                Ok(mut opt) => {
                    *opt = None;
                    Ok(Box::new(StatusCode::OK))
                }
            }
        }
    }
}

mod store {
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
}
