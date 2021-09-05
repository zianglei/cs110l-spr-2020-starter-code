use std::env;
use std::process;
use std::io::{ self, BufRead };
use std::fs::File;

fn read_file_lines(filename: &String) -> Result<Vec<String>, io::Error> {
    
    let file = File::open(filename)?;
    let mut contents = Vec::new();

    for line in io::BufReader::new(file).lines() {
        let line_str = line?;
        contents.push(line_str);
    }

    Ok(contents)
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Too few arguments.");
        process::exit(1);
    }
    let filename = &args[1];                                                                                               
    // Your code here :)
    let contents: Vec<String> = read_file_lines(filename).unwrap_or_else(|_| panic!("Invalid filename: {}", filename));
    
    let mut words: usize = 0;
    let mut lines: usize = 0;
    let mut characters: usize = 0;
    
    for line in contents.iter() {
        characters += line.len() + 1;
        let word_vec = line.split(" ").collect::<Vec<&str>>();
        words += word_vec.len();
        lines += 1;
    }

    println!("{} {} {}", words, lines, characters);
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_read_file_lines() {
        let lines_result = read_file_lines(&String::from("handout-a.txt"));
        assert!(lines_result.is_ok());
        let lines = lines_result.unwrap();
        assert_eq!(lines.len(), 8);
        assert_eq!(
            lines[0],
            "This week's exercises will continue easing you into Rust and will feature some"
        );
    }
}