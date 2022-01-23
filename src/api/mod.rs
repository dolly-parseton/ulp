//
mod routes;
mod store;

// Uses
use crate::job::Job;
use std::path::PathBuf;
use std::sync::{mpsc, Arc, Mutex};
use warp::{reject::Reject, Filter, Rejection, Reply};

// Pub uses
pub use routes as handlers;
pub use store::Store;

//
pub type Sender<T> = Arc<Mutex<mpsc::Sender<T>>>;

// Functions
pub fn routes(
    current_job: &Store<Option<Job>>,
    sender_store: &Store<Sender<String>>,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    // Job
    let job_get = warp::path!("job")
        .and(current_job.clone().into_warp())
        .and(warp::get())
        .and_then(handlers::job::get);
    let job_post = warp::path!("job")
        .and(current_job.clone().into_warp())
        .and(sender_store.clone().into_warp())
        .and(warp::post())
        .and(job_post_body())
        .and_then(handlers::job::post);
    let job_delete = warp::path!("job")
        .and(current_job.clone().into_warp())
        .and(warp::delete())
        .and_then(handlers::job::delete);

    job_get.or(job_post).or(job_delete)
}

#[derive(Debug)]
pub struct CustomError(pub String);
impl Reject for CustomError {}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct JobPath(pub PathBuf);

pub fn job_post_body() -> impl Filter<Extract = (JobPath,), Error = warp::Rejection> + Clone {
    // When accepting a body, we want a JSON body
    // (and to reject huge payloads)...
    warp::body::content_length_limit(1024 * 16).and(warp::body::json())
}
