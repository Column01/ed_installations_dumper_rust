use std::io::{BufRead, BufReader, Error};
use std::fs::File;
use bzip2::read::BzDecoder;
use mongodb::sync::Client;
use serde_json::Value;
use rayon::prelude::*;

enum Reader {
    DecompressorReader(BufReader<BzDecoder<File>>),
    NormalReader(BufReader<File>)
}

fn get_reader(file_name: &str) -> Result<Reader, Error> {
    let file = File::open(file_name)?;
    if file_name.ends_with("bz2") {
        return Ok(Reader::DecompressorReader(BufReader::new(BzDecoder::new(file))));
    } else {
        return Ok(Reader::NormalReader(BufReader::new(file)));
    }
    
}

pub fn import_files(client: &Client, file_names: &Vec<String>, num_workers: usize) -> std::io::Result<()> {

    let pool = rayon::ThreadPoolBuilder::new().num_threads(num_workers).build().unwrap();
    pool.install(|| {
        file_names.par_iter().for_each(|file_name| {
            println!("Importing file: {}", file_name);
            // Get the reader for the file and import it
            let db = client.database("FSSSignalDiscovered");
            let collection = db.collection("rust_test");
    
            match get_reader(file_name) {
                Ok(Reader::DecompressorReader(r)) => {
                    // Compressed file reader
                    let mut lines = Vec::new();
                    for line in r.lines() {
                        let line = line.unwrap();
                        let json_blob: Value = serde_json::from_str(&line).expect(&format!("Error loading json data from: {}", file_name));
                        let doc = bson::to_document(&json_blob).expect("Error converting json blob to bson!");
                        lines.push(doc);
                    }
                    let _ = collection.insert_many(lines, None).unwrap();
                }
                Ok(Reader::NormalReader(r)) => {
                    // Normal file reader
                    let mut lines = Vec::new();
                    for line in r.lines() {
                        let line = line.unwrap();
                        let json_blob: Value = serde_json::from_str(&line).expect(&format!("Error loading json data from: {}", file_name));
                        let doc = bson::to_document(&json_blob).expect("Error converting json blob to bson!");
                        lines.push(doc);
                    }
                    let _ = collection.insert_many(lines, None).unwrap();
                }
                Err(_) => todo!(),
            }
        })
    });
    Ok(())
}