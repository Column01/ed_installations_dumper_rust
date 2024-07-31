use bzip2::read::BzDecoder;
use mongodb::sync::Client;
use rayon::prelude::*;
use serde_json::Value;
use std::fs::File;
use std::io::{Error, Read};
use std::path::Path;

enum Reader {
    DecompressorReader(BzDecoder<File>),
    NormalReader(File),
}

fn get_reader(file_name: &str) -> Result<Reader, Error> {
    let file = File::open(file_name)?;
    if file_name.ends_with("bz2") {
        return Ok(Reader::DecompressorReader(BzDecoder::new(
            file,
        )));
    }
    return Ok(Reader::NormalReader(file));
}

pub fn import_files(
    client: &Client,
    file_paths: &Vec<String>,
    num_workers: usize,
) -> Result<(), Error> {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(num_workers)
        .build()
        .unwrap();
    pool.install(|| {
        file_paths.par_iter().for_each(|file_path| {
            println!("Importing file: {}", file_path);
            // Get the reader for the file and import it
            let db = client.database("FSSSignalDiscovered");
            let collection = db.collection("rust_test");

            match get_reader(file_path) {
                Ok(Reader::DecompressorReader(mut r)) => {
                    // Compressed file reader
                    // Collect all lines from the file and store them
                    let mut str = String::new();
                    r.read_to_string(&mut str).unwrap();
                    let mut docs = Vec::new();
                    for line in str.trim().split("\n") {
                        let json_blob: Value = serde_json::from_str(&line)
                            .expect(&format!("Error loading json data from: {}", file_path));
                        let doc = bson::to_document(&json_blob)
                            .expect("Error converting json blob to bson!");
                        docs.push(doc);
                    }
                    
                    // Remove the original string from memory
                    drop(str);
                    // Insert all lines into the collection
                    let _ = collection.insert_many(docs).run().unwrap();

                    // Move the file after processing
                    let processed_file_path = format!("downloads/processed/{}", file_path.replace("downloads/", ""));
                    // Ensure the processed directory exists
                    if !Path::new("downloads/processed").exists() {
                        std::fs::create_dir_all("downloads/processed").unwrap();
                    }
                    std::fs::rename(file_path, processed_file_path).expect("Error moving file after import!");
                }
                Ok(Reader::NormalReader(mut r)) => {
                    // Normal file reader
                    // Collect all lines from the file and store them
                    let mut str = String::new();
                    r.read_to_string(&mut str).unwrap();
                    let mut docs = Vec::new();
                    for line in str.trim().split("\n") {
                        let json_blob: Value = serde_json::from_str(&line)
                            .expect(&format!("Error loading json data from: {}", file_path));
                        let doc = bson::to_document(&json_blob)
                            .expect("Error converting json blob to bson!");
                        docs.push(doc);
                    }
                    // Remove the original string from memory
                    drop(str);
                    // Insert all lines into the collection
                    let _ = collection.insert_many(docs).run().unwrap();

                    // Move the file after processing
                    let processed_file_path = format!("downloads/processed/{}", file_path.replace("downloads/", ""));
                    // Ensure the processed directory exists
                    if !Path::new("downloads/processed").exists() {
                        std::fs::create_dir_all("downloads/processed").unwrap();
                    }
                    std::fs::rename(file_path, processed_file_path).expect("Error moving file after import!");
                }
                Err(e) => println!("Error when importing file: {}", e),
            }
        })
    });
    Ok(())
}
