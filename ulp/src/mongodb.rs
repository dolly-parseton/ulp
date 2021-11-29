use mongod::Client;

pub fn create_client(database: &str) -> Result<Client, mongod::Error> {
    mongod::ClientBuilder::new()
        .database(database) // Database should be job
        .uri(crate::MONGODB_ADDRESS.to_string())
        .build()
}

#[derive(Bson, Mongo)]
#[mongo(collection = "parsed", field, filter, update)]
pub struct ParsedData {
    pub index_str: String,
    pub data: Vec<u8>,
}

pub async fn send_data(
    client: Client,
    index_str: &str,
    data: Vec<u8>,
) -> Result<(), mongod::Error> {
    //
    let res = client
        .insert(vec![ParsedData {
            index_str: index_str.to_string(),
            data,
        }])
        .await;
    println!("{:#?}", res);
    Ok(())
}

pub fn send_mapping(client: &Client, mapping: Vec<u8>, update: bool) -> Result<(), mongod::Error> {
    //
    Ok(())
}
