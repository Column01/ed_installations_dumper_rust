mod downloader;
mod helpers;

use std::fs;
use std::io;
use std::rc::Rc;

use html5ever::rcdom::Node;
use reqwest;
use serde_json::{json, to_writer_pretty, Value};
use soup::prelude::*;
use chrono::prelude::*;
use chrono::TimeDelta;

fn parse_url(td: &Rc<Node>, base_url: &str) -> String {
    if td.text() != "None" {
        let a = td.tag("a").find().expect("None");
        if a.text() != "None" {
            let url = a.get("href").expect("None");
            if url != "None" {
                return format!("{}{}", base_url, url);
            }
            return "None".to_string();
        }
        return "None".to_string();
    }
    return "None".to_string();
}

// Collects two lists: a list of files in the current directory and a list of directories
fn find_files(url: &str) -> (Vec<Value>, Vec<String>) {
    // Basically a const but I cannot define a constant using a function call like this...
    let update_17 = NaiveDate::parse_from_str("2023-Oct-15 12:00:00", "%Y-%b-%d %H:%M:%S").unwrap();

    let res = reqwest::blocking::get(url).unwrap();
    let html = res.text().unwrap();
    let soup = Soup::new(&html);

    // Find all table rows and initialize return values
    let trs = soup.tag("tr").find_all();
    let mut files = Vec::new();
    let mut directories = Vec::new();

    for tr in trs {
        // Get the file name from the table row
        let file_name = tr.class("n").find().expect("None");

        // Skip the table header and "parent directory" items
        if file_name.text() == "Name" || file_name.text() == "../" {
            continue;
        }

        // Get other file info
        let file_modified = tr.class("m").find().expect("None");
        let file_type = tr.class("t").find().expect("None");
        let file_size = tr.class("s").find().expect("None");
        
        // Skip invalid entries
        if file_modified.text() != "None" {
            // Make a NaiveDate from the file modified info
            let file_date = NaiveDate::parse_from_str(file_modified.text().as_str(), "%Y-%b-%d %H:%M:%S").unwrap();

            // Compare the file's creation date to the update 17 release date, if its newer than that update we want it
            if (file_date - update_17) > TimeDelta::new(0, 0).expect("Error when making timedelta") {
                // Parse the url from the file_name Node
                let url = parse_url(&file_name, url);

                if url != "None" {
                    if file_type.text() == "Directory" {
                        if file_name.text() != "../" {
                            // Add the next directory's URL to the list to be searched if its not the parent dir
                            directories.push(url);
                        }
                    } else {
                        // Create a json blob of the file info
                        let data = json!({
                            "file_name": file_name.text(),
                            "file_type": file_type.text(),
                            "file_size": helpers::string_to_bytes_value(file_size.text()),
                            "file_modified": file_modified.text(),
                            "url": url
                        });

                        // Push a file blob into the file vector
                        files.push(data);
                    }
                }
            }
        }
    }
    return (files, directories);
}

// Crawls a directory recursively and collects all files in current and sub directories
fn crawl_directory(base_url: &str) -> Vec<Value> {
    println!("Crawling URL: {}", base_url);
    let (mut files, directories) = find_files(base_url);
    for directory in directories {
        files.extend(crawl_directory(&directory))
    }
    println!("Indexed {} files", files.len());
    return files;
}

fn main() -> std::io::Result<()> {
    // The base url to start our recursive crawl
    let base_url = "https://edgalaxydata.space/EDDN/";
    // The timestamp for update 17 which added the SignalType attribute to FSS Signal Logs
    let update_17 = NaiveDate::parse_from_str("2023-Oct-15 12:00:00", "%Y-%b-%d %H:%M:%S").unwrap();

    // Start crawling the directories
    let files: Vec<Value> = crawl_directory(base_url);

    println!("Files have been indexed. Would you like to save their details to a json file? (Y/N): ");
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");
    let input = input.trim();
    match input {
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

    let signal_files: Vec<&Value>= files.iter()
                                        .filter(|x| x["file_name"].to_string().contains("FSSSignalDiscovered"))
                                        .filter(|y| {
                                            (NaiveDate::parse_from_str(y["file_modified"].as_str().unwrap(), "%Y-%b-%d %H:%M:%S").unwrap() - update_17) > TimeDelta::new(0, 0).expect("Error when making timedelta")
                                        })
                                        .collect();

    let mut total_size: f64 = 0.0;
    signal_files.iter().for_each(|x| total_size += x["file_size"].as_f64().unwrap());


    println!("Filtered {} files totalling {} in size. Would you like to download them? (Y/N): ", signal_files.len(), helpers::bytes_value_to_size_string(total_size));
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");
    let input = input.trim();
    match input {
        "Y" | "y" => {
            // The number of threads to use in the download. Defaults to 4
            let num_workers = 10;
            println!("Downloading files to disk with {} threads...", num_workers);

            // Initialize the two file info vectors
            let mut file_urls: Vec<&str> = Vec::new();
            let mut file_names: Vec<&str> = Vec::new();
            // Populate the file info vectors with the info we need
            signal_files.iter().for_each(|x| {
                file_urls.push(x["url"].as_str().unwrap());
                file_names.push(x["file_name"].as_str().unwrap());
            });
            // Download the files
            let result = downloader::download_files_in_parallel(&file_urls, &file_names, num_workers);

            match result {
                Ok(_) => println!("Successfully downloaded {} files!", file_urls.len()),
                Err(e) => println!("Problem downloading files! {:?}", e)
            }

        }
        "N" | "n" => println!("Not saving files to Disk..."),
        _ => println!("Invalid input. Please enter Y or N."),
    }
    
    return Ok(());
}
