// use std::error::Error;
use regex::Regex;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use std::process;

// fn read_log_file(path: &str) -> Result<(), Box<dyn Error>> {
//     let contents = fs::read_to_string(path)?;
//     println!("With text:\n{contents}");
//     return Ok(());
// }

fn main() {
    // File hosts.txt must exist in the current path
    if let Ok(lines) = read_lines("./access.log") {
        // Consumes the iterator, returns an (Optional) String
        for line in lines {
            if let Ok(line) = line {
                let re = Regex::new(r"(?<ip>\d+\.\d+\.\d+\.\d+)\W-\W-\W\[(?<time>.+)\](?<rest>.+)")
                    .unwrap();
                // let re =
                //     Regex::new(r"(?<ip>[\d\.\:]+)\W-\W-\W\[(?<time>.+)\](?<method>.+)").unwrap();
                // let re = Regex::new(r"(?<ip>\d+\.\d+\.\d+\.\d+)\W-\W-\W\[(?<time>.+)\]").unwrap();
                let m = re.find(&line).unwrap();
                let caps = re.captures(&line).unwrap();
                println!("IP: {}", caps.name("ip").unwrap().as_str());
                println!("Time: {}", caps.name("time").unwrap().as_str());
                println!("Rest: {}", caps.name("rest").unwrap().as_str());
                println!("m: {}", m.as_str());
                println!("line: {}", line);
            }
        }
    } else {
        println!("Error WTF?");
        process::exit(1)
    }
}

// The output is wrapped in a Result to allow matching on errors
// Returns an Iterator to the Reader of the lines of the file.
fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}
// fn main() {
//     // println!("Hello, world!");
//     let path = "access.log";
//     if let Err(e) = read_log_file(path) {
//         println!("An error occurred: {e}");
//         process::exit(1);
//     };
// }
