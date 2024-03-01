use std::io::{BufRead, BufReader};
use std::fs::File;
use bzip2::read::BzDecoder;

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

pub fn import_file(file_name: &str) -> std::io::Result<()> {

    // Get the reader for the file and import it
    // TODO: Find a way to not care about the type and
    match get_reader(file_name) {
        Reader::DecompressorReader(mut r) => {
            // Compressed file reader
            let mut line = String::new();
            while r.read_line(&mut line)? > 0 {
                println!("{}", line.trim());
                line.clear();
                break;
            }
        },
        Reader::NormalReader(mut r) => {
            // Normal file reader
            let mut line = String::new();
            while r.read_line(&mut line)? > 0 {
                println!("{}", line.trim());
                line.clear();
                break;
            }
        },
    }
    
    Ok(())
}