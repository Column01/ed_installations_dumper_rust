use std::io;
use std::io::Write;

use chrono::{NaiveDate, TimeDelta};

const SIZE_STRINGS: [&str; 4] = ["KB", "MB", "GB", "TB"];

pub fn string_to_bytes_value(string: String) -> f64 {
    // Copy the string so we can mutate it.
    let mut str_copy = string.clone();
    // Remove the last char (the size character) and save it for later
    let size_char = str_copy.pop().unwrap();
    // Parse the str as a float
    let size_value: f64 = str_copy.parse().unwrap();
    // Find the correct multiplier
    let multiplier = match size_char {
        'K' => 1024,
        'M' => 1048576,
        'G' => 1073741824,
        _ => 1
    };
    // Calculate the bytes value
    return size_value * multiplier as f64;
}

pub fn bytes_value_to_size_string(bytes_value: f64) -> String {
    let mut ret = String::new();
    let mut val: f64 = bytes_value / 1024.0;
    let mut i = 0;

    while val >= 1024.0 && i < 4 {
        i += 1;
        val = val / 1024.0;
    }

    ret.push_str(&format!("{:.5}", val));
    ret.push_str(SIZE_STRINGS[i]);
    return ret;
}

pub fn date_is_after(to_check: NaiveDate, reference: NaiveDate) -> bool {
    return (to_check - reference) > TimeDelta::new(0, 0).unwrap();
}

pub fn get_input(message: &str) -> String {
    let mut input = String::new();
    print!("\n{}", message);
    io::stdout().flush().unwrap();
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");
    return input;
}
