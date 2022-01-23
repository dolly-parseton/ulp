// Uses
use super::{store::Store, Sender};
// use std::sync::{mpsc, Mutex};
use crate::job::{Job, Status};
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
        store: Store<Option<Job>>,
        sender: Store<Sender<String>>,
        path: crate::api::JobPath,
    ) -> Result<Box<dyn Reply>, Rejection> {
        let s = match sender.inner.read() {
            Err(e) => return Err(warp::reject::custom(crate::api::CustomError(e.to_string()))),
            Ok(send) => match send.lock() {
                Err(e) => return Err(warp::reject::custom(crate::api::CustomError(e.to_string()))),
                Ok(s) => s.clone(),
            },
        };
        match store.inner.write() {
            Err(e) => Err(warp::reject::custom(crate::api::CustomError(e.to_string()))),
            Ok(mut opt) => {
                println!("{:?}", opt);
                match &*opt {
                    Some(job) => match job.status {
                        Status::Pending => Ok(Box::new(StatusCode::FORBIDDEN)),
                        Status::Done => {
                            s.send(format!("{}", &path.0.display())).unwrap();
                            *opt = Some(Job::from(path.0));
                            Ok(Box::new(StatusCode::OK))
                        }
                    },
                    None => {
                        s.send(format!("{}", &path.0.display())).unwrap();
                        *opt = Some(Job::from(path.0));
                        Ok(Box::new(StatusCode::OK))
                    }
                }
            }
        }
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
