// use std::error::Error;
use regex::Regex;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use std::process;

fn main() {
    // File hosts.txt must exist in the current path
    if let Ok(lines) = read_lines("./access.log") {
        // Consumes the iterator, returns an (Optional) String
        for line in lines {
            if let Ok(line) = line {
                // let re = Regex::new(
                //     r#"(?<ip>\d+\.\d+\.\d+\.\d+)\W-\W-\W\[(?<time>.+)\] (?<method>".+")(?<code>\W\d+\W)(?<bytes>\d+) "(?<misc>.+)" "(?<ua>.+)""#,
                // )
                let re = Regex::new(
                    r#"(?<ip>\S+) - - \[(?<time>.+)\] (?<method>".+")(?<code>\W\d+\W)(?<bytes>\d+) "(?<misc>.+)" "(?<ua>.+)""#,
                )
                .unwrap();
                let m = re.find(&line).unwrap();
                let caps = re.captures(&line).unwrap();
                println!("\n");
                println!("IP: {}", caps.name("ip").unwrap().as_str());
                println!("Time: {}", caps.name("time").unwrap().as_str());
                println!("Method: {}", caps.name("method").unwrap().as_str());
                println!("Code: {}", caps.name("code").unwrap().as_str());
                println!("Bytes: {}", caps.name("bytes").unwrap().as_str());
                println!("Misc: {}", caps.name("misc").unwrap().as_str());
                println!("UA: {}", caps.name("ua").unwrap().as_str());
                println!("m: {}", m.as_str());
                println!("line: {}", line);
                println!("\n");
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
