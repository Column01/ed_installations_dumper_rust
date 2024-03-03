use std::io::{BufRead, BufReader};
use std::fs::File;
use bzip2::read::BzDecoder;
use mongodb::sync::Client;
use serde_json::Value;

enum Reader {
    DecompressorReader(BufReader<BzDecoder<File>>),
    NormalReader(BufReader<File>)
}

fn get_reader(file_name: &str) -> Reader {
    let file = File::open(file_name).expect(&format!("Error creating file object: {}", file_name));
    if file_name.ends_with("bz2") {
        return Reader::DecompressorReader(BufReader::new(BzDecoder::new(file)));
    }
    return Reader::NormalReader(BufReader::new(file));
}

pub fn import_file(client: &Client, file_name: &str) -> std::io::Result<()> {
    println!("Importing file: {file_name}");
    // Get the reader for the file and import it
    let db = client.database("FSSSignalDiscovered");
    let collection = db.collection("rust_test");
    // For testing purposes we just drop the collection to start fresh each time
    let _ = collection.drop(None);

    match get_reader(file_name) {
        Reader::DecompressorReader(r) => {
            // Compressed file reader
            let mut lines = vec![];
            for line in r.lines() {
                let line = line?;
                let json_blob: Value = serde_json::from_str(&line).expect(&format!("Error loading json data from: {}", file_name));
                let doc = bson::to_document(&json_blob).expect("Error converting json blob to bson!");
                lines.push(doc);
            }
            let _ = collection.insert_many(lines, None).unwrap();
        
        },
        Reader::NormalReader(r) => {
            // Normal file reader
            let mut lines = vec![];
            for line in r.lines() {
                let line = line?;
                let json_blob: Value = serde_json::from_str(&line).expect(&format!("Error loading json data from: {}", file_name));
                let doc = bson::to_document(&json_blob).expect("Error converting json blob to bson!");
                lines.push(doc);
            }
            let _ = collection.insert_many(lines, None).unwrap();
        },
    }
    
    return Ok(());
}