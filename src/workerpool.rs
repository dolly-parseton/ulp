pub use orchestrator::Orchestrator;
pub use pool::{message::Message, WorkerPool};
pub use queue::Queue;

mod orchestrator {
    use super::*;
    use crate::{api::Store, job::Job};

    use std::{
        sync::{Arc, Mutex},
        thread::spawn,
    };
    use warp::Filter;
    // Default number of workers.
    const DEFAULT_WORKERS_N: usize = 8;
    lazy_static! {
        pub static ref WORKERS_N: usize = {
            if let Ok(v) = std::env::var("ULP_WORKERS_N") {
                if let Ok(u) = v.parse::<usize>() {
                    return u;
                }
            }
            DEFAULT_WORKERS_N
        };
    }

    #[derive(Clone)]
    pub struct Orchestrator {
        pub pool: Arc<Mutex<WorkerPool>>,
        pub worker_queue: Queue<crate::job::Job>,
        pub api_queue: Queue<String>,
        pub job_store: Store<Option<crate::job::Job>>,
    }

    impl Default for Orchestrator {
        fn default() -> Self {
            Self {
                pool: Arc::new(Mutex::new(WorkerPool::new(*WORKERS_N))),
                worker_queue: Queue::new(),
                api_queue: Queue::new(),
                job_store: Store::new(),
            }
        }
    }

    impl Orchestrator {
        pub async fn run_api(&self) {
            let job_store = self.job_store.clone();
            let message_queue = self.api_queue.clone();
            tokio::spawn(async move {
                println!("API starting...");
                warp::serve(crate::api::routes(&job_store, &message_queue).with(warp::log("ulp")))
                    .run(([0, 0, 0, 0], 3030))
                    .await;
            });
            //
            let api_queue = self.api_queue.clone();
            let worker_queue = self.worker_queue.clone();
            spawn(move || loop {
                let message = api_queue.take();
                println!("{}", &message);
                let job = Job::from(message.as_str());
                println!("{:?}", &job);
                worker_queue.push(job);
            });
            //
            let worker_queue = self.worker_queue.clone();
            let pool = self.pool.clone();
            spawn(move || loop {
                let worker_job = worker_queue.take();
                for task in worker_job {
                    pool.lock().unwrap().send_message(Message::Task(task));
                }
            });
            //
            loop {
                std::thread::sleep(std::time::Duration::from_secs(1));
                println!(
                    "{:#?}",
                    self.pool.lock().expect("Could not lock pool").status_map()
                )
            }
        }
    }
}

mod pool {
    use super::queue::Queue;
    use std::{
        collections::HashMap,
        path::PathBuf,
        sync::{mpsc, Arc, Mutex},
        thread::spawn,
    };
    //
    pub type Sender = mpsc::SyncSender<message::Message>;
    pub type Receiver = Arc<Mutex<mpsc::Receiver<message::Message>>>;
    pub fn worker_channel() -> (Sender, Receiver) {
        let (s, r) = mpsc::sync_channel(1000);
        (s, Arc::new(Mutex::new(r)))
    }
    //
    mod worker {
        use super::*;
        use crate::Parser;
        //
        #[derive(Clone)]
        pub struct Worker {
            pub id: uuid::Uuid,
            pub status: Arc<Mutex<Option<PathBuf>>>,
        }
        //
        impl Worker {
            pub fn new(queue: Queue<super::message::Message>, output: Sender) -> Self {
                let status = Arc::new(Mutex::new(None));
                let id = uuid::Uuid::new_v4();
                Self::run_worker(id, Arc::clone(&status), queue, output);
                Self { id, status }
            }
            pub fn get_status(&self) -> Option<PathBuf> {
                self.status
                    .lock()
                    .unwrap_or_else(|_| panic!("Worker {} failed to lock status Mutex", self.id))
                    .clone()
            }
            fn run_worker(
                id: uuid::Uuid,
                status: Arc<Mutex<Option<PathBuf>>>,
                queue: Queue<super::message::Message>,
                output: Sender,
            ) {
                spawn(move || loop {
                    // Worker thread
                    // Get the details of the task to be done. Includes:
                    // - The file to process
                    // - The size and meta data of the file
                    // - How it is to be parsed
                    if let Ok(mut s) = status.lock() {
                        *s = None
                    }
                    let task_wrapper = queue.take();
                    use super::message::Message::*;
                    match &task_wrapper {
                        Task(task) => {
                            if let Ok(mut s) = status.lock() {
                                *s = Some(task.path.clone())
                            }
                            // Log details and start
                            println!("Processing task ({:?}): {:?}", id, &task);
                            // Do the task / process the file

                            use std::convert::TryFrom;
                            if let Ok(parser) = Parser::try_from(&task.path) {
                                parser.run_parser(&task.path);
                            }

                            println!("Finished task ({:?}): {:?}", id, &task);
                            // Log finished details / send output
                            let _ = output.send(task_wrapper).unwrap_or_else(|_| {
                                panic!("Worker {} failed to send results to orchestrator", id)
                            });
                        }
                        Debug(_) => {
                            println!("Processing task ({:?}): {:?}", id, &task_wrapper);
                            std::thread::sleep(std::time::Duration::from_millis(1));
                            println!("Finished task ({:?}): {:?}", id, &task_wrapper);
                            let _ = output.send(task_wrapper).unwrap_or_else(|_| {
                                panic!("Worker {} failed to send results to orchestrator", id)
                            });
                        }
                    }
                });
            }
        }
    }

    pub mod message {
        //
        #[derive(Clone, Debug)]
        pub enum Message {
            Debug(i64),
            Task(crate::job::Task),
        }

        impl From<i64> for Message {
            fn from(i: i64) -> Self {
                Message::Debug(i)
            }
        }
    }

    #[derive(Clone)]
    pub struct WorkerPool {
        pub queue: Queue<message::Message>,
        pub workers: Vec<worker::Worker>,
        //
        pub sender: Sender,
        pub receiver: Receiver,
    }

    impl WorkerPool {
        pub fn new(size: usize) -> Self {
            let (sender, receiver) = worker_channel();
            let queue = Queue::new();
            let mut workers = Vec::with_capacity(size);
            for _i in 0..size {
                workers.push(worker::Worker::new(queue.clone(), sender.clone()));
            }
            Self {
                workers,
                queue,
                sender,
                receiver,
            }
        }
        //
        pub fn status_map(&self) -> HashMap<uuid::Uuid, Option<PathBuf>> {
            let mut map = HashMap::new();
            for worker in &self.workers {
                map.insert(worker.id, worker.get_status());
            }
            map
        }
        //
        pub fn send_message(&mut self, message: message::Message) {
            self.queue.push(message);
        }
    }
    //
    #[cfg(test)]
    mod tests {
        #[test]
        fn pool_test_01() {
            let mut pool = super::WorkerPool::new(8);
            for i in 0..1000 {
                pool.send_message(i.into());
            }
            while pool.queue.len() != 0 {
                std::thread::sleep(std::time::Duration::from_secs(1));
                println!("{} jobs left in queue", pool.queue.len());
            }
            println!("FINISHED! {} jobs left in queue", pool.queue.len());
        }
    }
}

mod queue {
    use std::{
        convert::Infallible,
        sync::{Arc, Condvar, Mutex, MutexGuard},
    };
    use warp::Filter;

    #[derive(Clone)]
    pub struct Queue<T: Clone + Send + Sync> {
        block: Arc<Mutex<()>>,
        guard: Arc<Condvar>,
        queue: Arc<Mutex<Vec<T>>>,
    }

    impl<T: Clone + Send + Sync> Default for Queue<T> {
        fn default() -> Self {
            Queue {
                block: Arc::new(Mutex::new(())),
                guard: Arc::new(Condvar::new()),
                queue: Arc::new(Mutex::new(vec![])),
            }
        }
    }

    impl<T: Clone + Send + Sync> Queue<T> {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn clear(&self) {
            let mut block = self.block.lock().expect("could not get empty");
            let mut queue = self.queue.lock().expect("could not get queue");
            queue.clear();
            *block = ();
        }

        pub fn is_empty(&self) -> bool {
            let empty = self.lock().is_empty();
            empty
        }

        pub fn len(&self) -> usize {
            let len = self.queue.lock().expect("could not get empty").len();
            len
        }

        pub fn lock(&self) -> LockedQueue<'_, T> {
            LockedQueue {
                queue: self.queue.lock().expect("could not get queue"),
            }
        }

        pub fn push(&self, element: T) {
            let mut block = self.block.lock().expect("could not get empty");
            let mut queue = self.queue.lock().expect("could not get queue");
            queue.push(element);
            *block = ();
            self.guard.notify_one();
        }

        pub fn remove<V>(&self, element: &V) -> Option<T>
        where
            T: PartialEq<V>,
        {
            let mut block = self.block.lock().expect("could not get empty");
            let mut queue = self.queue.lock().expect("could not get queue");
            let mut index = None;
            for (i, x) in queue.iter().enumerate() {
                if x == element {
                    index = Some(i);
                }
            }
            let val;
            if let Some(i) = index {
                val = Some(queue.remove(i));
            } else {
                val = None;
            }
            *block = ();
            val
        }

        pub fn take(&self) -> T {
            let mut block = self.block.lock().expect("could not get empty");
            loop {
                {
                    let mut queue = self.queue.lock().expect("could not get queue");
                    if !queue.is_empty() {
                        self.guard.notify_one();
                        return queue.remove(0);
                    }
                }
                block = self.guard.wait(block).unwrap();
            }
        }

        pub fn try_take(&self) -> Option<T> {
            let _block = self.block.lock().expect("could not get empty");
            let mut queue = self.queue.lock().expect("could not get queue");
            if !queue.is_empty() {
                self.guard.notify_one();
                return Some(queue.remove(0));
            }
            None
        }

        pub fn into_warp(self) -> impl Filter<Extract = (Self,), Error = Infallible> + Clone {
            warp::any().map(move || self.clone())
        }
    }

    pub struct LockedQueue<'a, T> {
        queue: MutexGuard<'a, Vec<T>>,
    }

    impl<'a, T> LockedQueue<'a, T> {
        pub fn iter(&self) -> impl Iterator<Item = &T> {
            self.queue.iter()
        }

        pub fn len(&self) -> usize {
            self.queue.len()
        }

        pub fn is_empty(&self) -> bool {
            self.queue.len() == 0
        }
    }
}
