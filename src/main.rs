extern crate log;
extern crate serde_json;
extern crate ulp;
// mod api;
// mod mft;
//
use ulp::workerpool::Orchestrator;

#[tokio::main]
async fn main() {
    env_logger::init();
    // Create store
    // let job_store = api::Store::new();
    let orchestrator = Orchestrator::default();
    orchestrator.run_api().await;
    // loop {}
    // // Start parsing runtime
    // let runtime = Arc::new(Runtime::new().unwrap());
    // // Start orchestration thread
    // let job_store_orchestration: api::Store<Option<ulp::job::Job>> = job_store.clone();
    // let (sender, recveiver) = mpsc::channel::<String>();
    // let sender_store = api::Store::from_t(Arc::new(Mutex::new(sender)));

    // // Start API thread
    // runtime.spawn(async move {
    //     info!("API starting...");
    //     warp::serve(api::routes(&job_store, &sender_store).with(warp::log("ulp")))
    //         .run(([0, 0, 0, 0], 3030))
    //         .await;
    // });

    // let runtime_clone = runtime.clone();
    // runtime.block_on(async move {
    //     info!("Orchestration thread ready!");
    //     loop {
    //         let message = match recveiver.recv() {
    //             Ok(message) => message,
    //             Err(_) => {
    //                 error!("No message recieved");
    //                 continue;
    //             }
    //         };
    //         println!("{}", message);
    //         // Read Job
    //         let job: ulp::job::Job = match job_store_orchestration.inner.read() {
    //             Ok(opt) => match &*opt {
    //                 Some(job) => job.clone(),
    //                 None => continue,
    //             },
    //             Err(e) => {
    //                 error!("{}", e);
    //                 continue;
    //             }
    //         };
    //         println!("{:#?}", job);
    //         // Action Job
    //         // match &job.parser_type {
    //         //     UlpParser::Evtx => (),
    //         //     UlpParser::Mft => {
    //         //         let mut parser = mft::ParserWrapper::from_path(&job.file_path).unwrap();
    //         //         parser.run(&[]).unwrap();
    //         //     }
    //         //     UlpParser::None => {
    //         //         let mut parser = mft::ParserWrapper::from_path(&job.file_path).unwrap();
    //         //         parser.run(&[]).unwrap();
    //         //     }
    //         // }
    //         // Create manifest file
    //     }
    // });
    // error!("FAILED...");
    // // // Start API thread
    // // runtime.block_on(async {
    // //     info!("API starting...");
    // //     warp::serve(api::routes(&job_store, &sender_store).with(warp::log("ulp")))
    // //         .run(([0, 0, 0, 0], 3030))
    // //         .await;
    // // });
}

// // #[derive(Debug, Serialize, Deserialize)]
// pub struct ParsedData {
//     pub index_str: String,
//     pub data: serde_json::Value,
// }

// fn main() {
//     let now = std::time::Instant::now();
//     println!("Reading MFT!");
//     let mut value = Mapping::default();
//     // value.set_target(vec!["IsDeleted"]);
//     let mut first = true;
//     let parser = ParserWrapper::from_path("./.test_data/$MFT").unwrap();
//     let mut data = Vec::new();
//     for entry in parser {
//         let json = serde_json::to_value(entry).unwrap();
//         value.map_json(&json);
//         // if first {
//         //     println!("{:#?}", json);
//         //     println!("{:#?}", value);
//         // }
//         data.push(ParsedData {
//             index_str: "".to_string(),
//             data: json,
//         });
//         first = false;
//     }
//     println!("Parsed: {:#?} sec", now.elapsed().as_secs());
//     let now = std::time::Instant::now();
//     println!("Total records {:#?}", data.len());
//     while !data.is_empty() {
//         let mut batch = Vec::new();
//         for _ in 0..10000 {
//             if let Some(entry) = data.pop() {
//                 batch.push(entry);
//             }
//         }
//         // let res = client.insert(batch).unwrap();
//         println!("{:#?}", "Send batch");
//     }
//     println!("Uploaded: {:#?} secs", now.elapsed().as_secs());
//     println!("{:#?}", value);
// }
