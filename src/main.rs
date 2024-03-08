mod downloader;
mod helpers;
mod importer;

use std::fs;
use std::rc::Rc;

use bson::{doc, Document};
use chrono::prelude::*;
use html5ever::rcdom::Node;
use mongodb::{options::FindOptions, sync::Client, sync::Collection};
use num_cpus;
use reqwest;
use serde_json::{json, to_writer_pretty, Value};
use soup::prelude::*;

fn parse_url(td: &Rc<Node>, base_url: &str) -> Option<String> {
    if td.text() != "None" {
        let a = td.tag("a").find().expect("None");
        if a.text() != "None" {
            let url = a.get("href").expect("None");
            if url != "None" {
                return Some(format!("{}{}", base_url, url));
            }
        }
    }
    return None;
}

// Collects two lists: a list of files in the current directory and a list of directories
fn find_files(url: &str) -> (Vec<Value>, Vec<String>) {
    // Basically a const but I cannot figure out a better way to do this...
    let update_17 = NaiveDate::from_ymd_opt(2023, 10, 15).unwrap();

    let res = reqwest::blocking::get(url).unwrap();
    let html = res.text().unwrap();
    let soup = Soup::new(&html);

    // Find all table rows and initialize return values
    let trs = soup.tag("tr").find_all();
    let mut files = Vec::new();
    let mut directories = Vec::new();

    for tr in trs {
        // Get the info about the file
        let file_name = tr
            .class("n")
            .find()
            .expect("File name attribute was not found");
        let file_type = tr
            .class("t")
            .find()
            .expect("File type attribute was not found");
        let file_size = tr
            .class("s")
            .find()
            .expect("File size attribute was not found");
        let file_modified = tr
            .class("m")
            .find()
            .expect("File modified attribute was not found");

        // Skip table headers and "parent directory" items
        if file_name.text() == "Name" || file_name.text() == "../" {
            continue;
        }

        // Make a NaiveDate from the file modified info
        let file_date =
            NaiveDate::parse_from_str(file_modified.text().as_str(), "%Y-%b-%d %H:%M:%S").unwrap();

        // Compare the file's creation date to the update 17 release date, if its newer than that update we want it
        if helpers::date_is_after(file_date, update_17) {
            // Parse the url from the file_name Node
            let parsed_url = parse_url(&file_name, url);

            match parsed_url {
                Some(url) => {
                    if file_type.text() == "Directory" {
                        if file_name.text() != "../" {
                            // Add the next directory's URL to the list to be searched if its not the parent dir
                            directories.push(url);
                        }
                    } else {
                        // Create a json blob of the file info
                        let data = json!({
                            "name": file_name.text(),
                            "type": file_type.text(),
                            "size": helpers::string_to_bytes_value(file_size.text()),
                            "modified": file_modified.text(),
                            "url": url
                        });

                        // Push a file blob into the file vector
                        files.push(data);
                    }
                }
                None => {}
            }
        }
    }
    return (files, directories);
}

fn crawl_directory(base_url: &str) -> Vec<Value> {
    println!("Crawling URL: {}", base_url);
    // Collect all files in a directory
    let (mut files, directories) = find_files(base_url);
    // Run a recursive scan of sub directories and collect all files
    directories
        .iter()
        .for_each(|dir| files.extend(crawl_directory(&dir)));

    println!("Indexed {} files", files.len());
    return files;
}

fn main() -> std::io::Result<()> {
    // The base url to start our recursive crawl
    let base_url = "https://edgalaxydata.space/EDDN/";

    // Start crawling the directories
    let files: Vec<Value> = crawl_directory(base_url);

    let input = helpers::get_input(
        "Files have been indexed. Would you like to save their details to a json file? (Y/N): ",
    );
    match input.trim() {
        "Y" | "y" => {
            println!("Saving files to JSON...");
            // Take the collection of files and turn it into a json blob
            let json_data = json!({"files": files});
            // Create a files.json file and dump the json data to it
            let file = fs::File::create("files.json")?;
            let result = to_writer_pretty(file, &json_data);
            if result.is_ok() {
                println!("Dumped {} file blobs to disk.", files.len());
            } else {
                println!("Error writing file blobs to disk:\n {:?}", result);
            }
        }
        "N" | "n" => {
            println!("Not saving files to JSON...");
        }
        _ => println!("Invalid input. Please enter Y or N."),
    }

    println!("Filtering files to only gather relevant ones");
    let signal_files: Vec<&Value> = files
        .iter()
        .filter(|x| x["name"].to_string().contains("FSSSignalDiscovered"))
        .filter(|x| !x["name"].to_string().contains("Test"))
        .collect();

    // Calculate the total size of all files we indexed
    let mut total_size: f64 = 0.0;
    signal_files
        .iter()
        .for_each(|x| total_size += x["size"].as_f64().unwrap());

    let input = helpers::get_input(&format!(
        "Filtered {} files totalling {} in size. Would you like to download them? (Y/N): ",
        signal_files.len(),
        helpers::bytes_value_to_size_string(total_size)
    ));
    match input.trim() {
        "Y" | "y" => {
            // The number of threads to use in the download. Defaults to: num_cpus - 1
            // (though if you have any more than a few cores and slow internet, you may want to lower this)
            let num_workers = num_cpus::get() - 1;
            println!("Downloading files to disk with {} threads...", num_workers);

            // Initialize the two file info vectors
            let mut urls: Vec<&str> = Vec::new();
            let mut names: Vec<&str> = Vec::new();
            // Populate the file info vectors with the info we need
            signal_files.iter().for_each(|x| {
                urls.push(x["url"].as_str().unwrap());
                names.push(x["name"].as_str().unwrap());
            });
            // Download the files
            let result = downloader::download_files_in_parallel(&urls, &names, num_workers);

            match result {
                Ok(_) => println!("Successfully downloaded {} files!", urls.len()),
                Err(e) => println!("Problem downloading files! {:?}", e),
            }
        }
        "N" | "n" => println!("Not saving files to Disk..."),
        _ => println!("Invalid input. Please enter Y or N."),
    }
    let input = helpers::get_input("Do you want to import any downloaded files? THIS IS A CONSIDERABLE TIME INVESTMENT! (Y/N): ");
    match input.trim() {
        "Y" | "y" => {
            let num_workers = num_cpus::get() / 3;
            // Create the mongo DB client we will use
            let client = Client::with_uri_str("mongodb://localhost:27017")
                .expect("Error when creating database client!");
            let db = client.database("FSSSignalDiscovered");
            let collection: Collection<Document> = db.collection("rust_test");

            // Create a new list to fill with file names
            let mut files = Vec::new();
            // Populate the list with files in the downloads directory
            let dir = fs::read_dir("downloads/").unwrap();
            for file in dir {
                let file = file.unwrap();
                let path = file.path();
                if path.is_file() {
                    files.push(path.into_os_string().into_string().unwrap());
                }
            }
            println!("Importing {} files...", files.len());
            // Try to import the files
            let _ = importer::import_files(&client, &files, num_workers)
                .expect("Error when inserting files into DB!");
        }
        "N" | "n" => println!("Not importing files to DB..."),
        _ => println!("Invalid input. Please enter Y or N."),
    }

    let input = helpers::get_input("Would you like to generate an installations dump? (Y/N): ");

    match input.trim() {
        "Y" | "y" => {
            println!("Connecting to database...");
            // Create the mongo DB client we will use
            let client = Client::with_uri_str("mongodb://localhost:27017")
                .expect("Error when creating database client!");
            let db = client.database("FSSSignalDiscovered");
            let collection: Collection<Document> = db.collection("rust_test");

            // Define the query
            let query = doc! {"message.signals.SignalType": "Installation"};

            // Define the projection
            let projection = doc! {
                "message.StarSystem": 1,
                "message.SystemAddress": 1,
                "message.StarPos": 1,
                "message.signals": {
                    "$filter": {
                        "input": "$message.signals",
                        "as": "signal",
                        "cond": {
                            "$eq": ["$$signal.SignalType", "Installation"]
                        }
                    }
                }
            };

            println!("Generating query...");
            let find_options = FindOptions::builder().projection(projection).build();
            let signals = collection.find(query, find_options).unwrap();
            // Create a new JSON Value to store unique signals
            let mut unique_signals = serde_json::Map::new();

            println!("Filtering results...");

            let mut i = 0;
            let mut has_printed = false;
            // Iterate over the results
            for result in signals {
                match result {
                    Ok(mut signal) => {
                        // Remove the _id key from the signal
                        signal.remove("_id");

                        // Convert the signal to a serde_json::Value
                        let data: serde_json::Value =
                            serde_json::from_str(&signal.to_string()).unwrap();

                        // Get the StarSystem from the message key in the data
                        let star_system =
                            &data["message"]["StarSystem"].as_str().unwrap().to_string();

                        // Check if the star system is in the unique_signals JSON value
                        if !unique_signals.contains_key(star_system) {
                            // If it isn't, add it with the data as the value
                            unique_signals.insert(star_system.to_string(), data);
                            // Print a message every 100 results
                            i += 1;
                            has_printed = false;
                        } else {
                            // If it is, check the timestamp of the current signal and the one in unique_signals
                            let current_timestamp = &data["message"]["signals"][0]["timestamp"];
                            let stored_timestamp =
                                &unique_signals[star_system]["message"]["signals"][0]["timestamp"];

                            // Parse the timestamps as NaiveDate
                            let current_date = NaiveDate::parse_from_str(
                                current_timestamp.as_str().unwrap(),
                                "%Y-%m-%dT%H:%M:%SZ",
                            )
                            .unwrap();
                            let stored_date = NaiveDate::parse_from_str(
                                stored_timestamp.as_str().unwrap(),
                                "%Y-%m-%dT%H:%M:%SZ",
                            )
                            .unwrap();

                            // If the current signal is newer, replace the one in unique_signals
                            if helpers::date_is_after(current_date, stored_date) {
                                unique_signals.insert(star_system.to_string(), data);
                                i += 1;
                                has_printed = false;
                            }
                        }
                        // Print a message every 1000 results we process
                        if i % 1000 == 0 && !has_printed {
                            println!("Processed {} total items that are new or should replace existing ones...", i);
                            has_printed = true;
                        }
                    }
                    Err(e) => println!("Error processing document: {:?}", e),
                }
            }

            // Print a message with the number of signals remaining in unique_signals
            println!("Number of unique signals: {}", unique_signals.len());
            println!("Dumping to disk as requested...");
            // Create a files.json file and dump the json data to it
            let file = fs::File::create("installations.json")?;
            let result = to_writer_pretty(file, &unique_signals);
            if result.is_ok() {
                println!("Dumped {} signals blobs to disk.", unique_signals.len());
            } else {
                println!("Error writing signal blobs to disk:\n {:?}", result);
            }
        }
        "N" | "n" => println!("Not generating an installations dump..."),
        _ => println!("Invalid input. Please enter Y or N."),
    }
    return Ok(());
}
