pub use orchestrator::Orchestrator;
pub use pool::{message::Message, WorkerPool};
pub use queue::Queue;

mod orchestrator {
    use super::*;
    use crate::{
        api::{ApiMessageType, Store},
        job::Job,
        type_map::Mapping,
    };

    use std::{
        fs,
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
        pub completed_queue: Queue<crate::job::Job>,
        pub processing_queue: Queue<crate::job::Job>,
        pub worker_queue: Queue<crate::job::Job>,
        pub api_queue: Queue<ApiMessageType>,
        pub job_store: Store<Option<crate::job::Job>>,
    }

    impl Default for Orchestrator {
        fn default() -> Self {
            debug!("Creating default Orchestrator");
            Self {
                pool: Arc::new(Mutex::new(WorkerPool::new(*WORKERS_N))),
                completed_queue: Queue::new(),
                processing_queue: Queue::new(),
                worker_queue: Queue::new(),
                api_queue: Queue::new(),
                job_store: Store::new(),
            }
        }
    }

    impl Orchestrator {
        pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
            let job_store = self.job_store.clone();
            let api_queue = self.api_queue.clone();
            // Run Warp API
            debug!("Spawning async Orchestrator API thread");
            tokio::spawn(async move {
                warp::serve(crate::api::routes(&job_store, &api_queue).with(warp::log("ulp")))
                    .run(([0, 0, 0, 0], 3030))
                    .await;
            });
            // Run the thread for converting api messages to job messages
            let api_queue = self.api_queue.clone();
            let worker_queue = self.worker_queue.clone();
            let pool = self.pool.clone();
            debug!("Spawning Orchestrator API message reader thread");
            let _api_message_handle = spawn(move || loop {
                let message_wrapper = api_queue.take();
                match message_wrapper {
                    ApiMessageType::Job(message) => match Job::from_glob(message.as_str()) {
                        Some(job) => {
                            trace!("Converted message: {} to job: {:?}", &message, &job);
                            worker_queue.push(job);
                        }
                        None => error!("Failed to convert message to job: {}", &message),
                    },
                    ApiMessageType::Elastic(uuid) => {
                        info!("Elastic ingestion Job issued for uuid: {}", &uuid);
                        // Read mapping into memory
                        let target_dir = format!("{}/{}", crate::UPLOAD_DIR_ENV, uuid);
                        let mapping: Mapping =
                            match fs::read_to_string(format!("{}/{}", target_dir, "mappings.json"))
                            {
                                Ok(m) => serde_json::from_str(&m).unwrap(),
                                Err(e) => {
                                    error!(
                                        "Failed to read mapping file at {}. {} ",
                                        format!("{}/{}", target_dir, "mappings.json"),
                                        e
                                    );
                                    panic!(
                                        "Failed to read mapping file at {}. {}",
                                        format!("{}/{}", target_dir, "mappings.json"),
                                        e
                                    )
                                }
                            };
                        for entry in glob::glob(format!("{}/*.data", target_dir).as_str()).unwrap()
                        {
                            if let Ok(path) = entry {
                                pool.lock().unwrap().send_message(Message::Elastic {
                                    map: mapping.clone(),
                                    data: path,
                                });
                            }
                        }
                    }
                }
            });
            // Read in job messages from queue and push to workers as tasks (1 file = 1 task)
            let processing_queue = self.processing_queue.clone();
            let worker_queue = self.worker_queue.clone();
            let pool = self.pool.clone();
            debug!("Spawning Orchestrator Job reader / Task issuer thread");
            let _job_task_handle = spawn(move || loop {
                let worker_job = worker_queue.take();
                // Clone the job so we can send it to the worker
                for task_res in worker_job.clone() {
                    match task_res {
                        Ok(task) => {
                            debug!("Sending Task ({}) to WorkerPool", task.id);
                            pool.lock().unwrap().send_message(Message::Task(task));
                        }
                        Err(e) => error!("Failed to derive task from job: {}", e),
                    }
                }
                let sent_len = worker_job.sent.lock().unwrap().len();
                info!(
                    "Job {}: {} Tasks sent for processing.",
                    worker_job.id, sent_len
                );
                processing_queue.push(worker_job);
            });
            // Read the tasks coming back from the workers and match to jobs and track tasks returning
            let processing_queue = self.processing_queue.clone();
            let completed_queue = self.completed_queue.clone();
            let pool_receiver = self.pool.lock().unwrap().receiver.clone();
            let pool = self.pool.clone();
            debug!("Spawning Orchestrator Task receiver / Completed issuer thread");
            let _task_recv_handle = spawn(move || loop {
                let message = pool_receiver.lock().unwrap().recv().unwrap();
                if let super::Message::Task(task) = message {
                    debug!("Task Message received from WorkerPool: {}", &task.id);
                    // Match message to job in working queue
                    info!(
                        "{} jobs in processing in queue",
                        processing_queue.lock().len()
                    );
                    let len = processing_queue.lock().len();
                    'queue: for _ in 0..len {
                        let mut working_job = processing_queue.take();
                        // Is len of Sent == len of Processed?
                        // Is message id in working_job.sent
                        let does_contain = working_job.sent.lock().unwrap().contains(&task.id);
                        if does_contain {
                            // println!("{:#?}", &task);
                            info!("Confirmed task {} has finished processing", &task.id);
                            info!("{} tasks waiting", pool.lock().unwrap().queue.lock().len());
                            working_job.processed.push(task);

                            let sent_len = working_job.sent.lock().unwrap().len();
                            if sent_len != 0 && sent_len == working_job.processed.len() {
                                info!("Confirmed job {} has finished processing.", working_job.id);
                                completed_queue.push(working_job);
                                break 'queue;
                            }
                            processing_queue.push(working_job);
                            break 'queue; // Uuid ensures ther is only one match so break here is safe
                        } else {
                            processing_queue.push(working_job);
                        }
                    }
                }
            });
            // Loop on status of workers, eventually to be replaced with optional CLI GUI for non-docker runs
            info!("Entering main Orchestrator loop");
            loop {
                // Waiting on #![feature(thread_is_running)]
                // if !api_message_handle.is_running() {
                //     error!("Orchestrator API message thread has stopped");
                //     panic!("Orchestrator API message thread has stopped");
                // }
                // if !job_task_handle.is_running() {
                //     error!("Orchestrator Job to Task thread has stopped");
                //     panic!("Orchestrator Job to Task thread has stopped");
                // }
                // if !task_recv_handle.is_running() {
                //     error!("Orchestrator Task receiver thread has stopped");
                //     panic!("Orchestrator Task receiver thread has stopped");
                // }
                //

                // std::thread::sleep(std::time::Duration::from_secs(10));
                debug!(
                    "{:#?}",
                    self.pool.lock().expect("Could not lock pool").status_map()
                );
                let completed = self.completed_queue.take();
                // println!("{:#?}", completed);
                // std::process::exit(1);
                let mapping = completed.mapping.lock().unwrap();
                std::fs::create_dir_all(format!("{}/{}/", crate::UPLOAD_DIR_ENV, completed.id))
                    .unwrap();
                let mut mapping_file = fs::OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create(true)
                    .open(format!(
                        "{}/{}/{}",
                        crate::UPLOAD_DIR_ENV,
                        completed.id,
                        "mappings.json"
                    ))
                    .unwrap();
                use std::io::Write;
                // println!("{:#?}", mapping);
                write!(
                    &mut mapping_file,
                    "{}",
                    serde_json::to_string(&*mapping).unwrap().to_string()
                )
                .unwrap();
                info!(
                    "Completed Job {} in: {:?}\n\tFiles: {}",
                    completed.id,
                    completed.started.elapsed(),
                    completed.paths.len()
                );
            }
        }
    }
}

mod pool {
    use super::queue::Queue;
    use std::{
        collections::BTreeMap,
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
                    match task_wrapper {
                        Task(task) => {
                            if let Ok(mut s) = status.lock() {
                                *s = Some(task.path.clone())
                            }
                            // Log details and start
                            trace!("Processing task ({:?}): {:?}", id, &task);
                            // Do the task / process the file
                            if let Ok(parser) = Parser::try_from(&task.path) {
                                parser.run_parser(&task);
                            }
                            trace!("Finished task ({:?}): {:?}", id, &task);
                            // Log finished details / send output
                            let _ = output.send(Task(task)).unwrap_or_else(|_| {
                                panic!("Worker {} failed to send results to orchestrator", id)
                            });
                        }
                        Elastic { map, data } => {
                            //
                            crate::elastic::normalise_then_send(map, data).unwrap();
                        }
                        Debug(_) => {
                            debug!("Processing task ({:?}): {:?}", id, &task_wrapper);
                            std::thread::sleep(std::time::Duration::from_millis(1));
                            debug!("Finished task ({:?}): {:?}", id, &task_wrapper);
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
        use crate::{job::Task, type_map::Mapping};
        use std::path::PathBuf;
        //
        #[derive(Clone, Debug)]
        pub enum Message {
            Debug(i64),
            Task(Task),
            Elastic { map: Mapping, data: PathBuf },
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
        pub fn status_map(&self) -> BTreeMap<uuid::Uuid, Option<PathBuf>> {
            let mut map = BTreeMap::new();
            for worker in &self.workers {
                map.insert(worker.id, worker.get_status());
            }
            map
        }
        //
        pub fn send_message(&mut self, message: message::Message) {
            self.queue.push(message);
        }
        //
        pub fn recv_message(&mut self) -> message::Message {
            self.receiver.lock().unwrap().recv().unwrap()
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
