use std::fs;
use std::path::Path;
use std::io::{Error, ErrorKind, Write};
use reqwest;
use rayon::prelude::*;


fn download_file(url: &str, file_name: &str) -> Result<(), Error> {
    // Make the HTTP GET request using a fresh client (fixes issues where we cannot download in parallel)
    // Fuck async reqwest, all my homies hate managing async contexts
    let client = reqwest::blocking::Client::new();
    let resp = client.get(url).send();

    if !resp.is_err() {
        let response = resp.unwrap();
        // Try to create the downloads directory. This should always work the first time but if it doesn't it could break the code...

        if !Path::exists(Path::new("downloads/")) {
            let _ = fs::create_dir("downloads/");
        }

        let full_path = "downloads/".to_owned() + file_name;

        if Path::exists(Path::new(&full_path)) {
            println!("Skipping existing file: {}", file_name);
            return Ok(());
        }
        // Open a file to write the downloaded content
        let mut file = std::fs::File::create(full_path).expect(&format!("Error creating file: {}", file_name));

        println!("Downloading file: {}", url);
        let content = response.bytes();
        if !content.is_err() {
            file.write(&content.unwrap())?;
        }
        return Ok(());
        
        // let _ = response.copy_to(&mut file);
        // Copy the bytes from the response to the file
        // copy(&mut response, &mut file)?;
    }
    return Err(Error::new(
        ErrorKind::Other,
        format!("Request failed with status: {}", resp.unwrap().status()),
    ).into());
}


pub fn download_files_in_parallel(urls: &Vec<&str>, file_names: &Vec<&str>, num_workers: usize) -> Result<(), Error> {
    // Zip the URLs and file names together
    let pairs: Vec<_> = urls.iter().zip(file_names.iter()).collect();
    let pool = rayon::ThreadPoolBuilder::new().num_threads(num_workers).build().unwrap();

    // Download the files in parallel
    pool.install(|| {
        pairs.par_iter().for_each(|(url, file_name)| {
            if let Err(err) = download_file(url, file_name) {
                eprintln!("Error downloading {}: {}", url, err);
            }
        });
    });

    Ok(())
}