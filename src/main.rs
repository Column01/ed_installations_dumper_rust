mod downloader;
mod helpers;
mod importer;

use std::fs;
use std::io;
use std::rc::Rc;

use html5ever::rcdom::Node;
use reqwest;
use num_cpus;
use serde_json::{json, to_writer_pretty, Value};
use soup::prelude::*;
use chrono::prelude::*;

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
        let file_name = tr.class("n").find().expect("File name attribute was not found");
        let file_type = tr.class("t").find().expect("File type attribute was not found");
        let file_size = tr.class("s").find().expect("File size attribute was not found");
        let file_modified = tr.class("m").find().expect("File modified attribute was not found");

        // Skip table headers and "parent directory" items
        if file_name.text() == "Name" || file_name.text() == "../" {
            continue;
        }

        // Make a NaiveDate from the file modified info
        let file_date = NaiveDate::parse_from_str(file_modified.text().as_str(), "%Y-%b-%d %H:%M:%S").unwrap();

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
                },
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
    directories.iter().for_each(|dir| files.extend(crawl_directory(&dir)));

    println!("Indexed {} files", files.len());
    return files;
}

fn main() -> std::io::Result<()> {
    // The base url to start our recursive crawl
    let base_url = "https://edgalaxydata.space/EDDN/";

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
                                        .filter(|x| x["name"].to_string().contains("FSSSignalDiscovered"))
                                        .collect();

    let mut total_size: f64 = 0.0;
    signal_files.iter().for_each(|x| total_size += x["size"].as_f64().unwrap());

    println!("Filtered {} files totalling {} in size. Would you like to download them? (Y/N): ", signal_files.len(), helpers::bytes_value_to_size_string(total_size));
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");
    let input = input.trim();
    match input {
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
                Err(e) => println!("Problem downloading files! {:?}", e)
            }

        }
        "N" | "n" => println!("Not saving files to Disk..."),
        _ => println!("Invalid input. Please enter Y or N."),
    }
    
    return Ok(());
}
