// Simple Hangman Program
// User gets five incorrect guesses
// Word chosen randomly from words.txt
// Inspiration from: https://doc.rust-lang.org/book/ch02-00-guessing-game-tutorial.html
// This assignment will introduce you to some fundamental syntax in Rust:
// - variable declaration
// - string manipulation
// - conditional statements
// - loops
// - vectors
// - files
// - user input
// We've tried to limit/hide Rust's quirks since we'll discuss those details
// more in depth in the coming lectures.
extern crate rand;
use rand::Rng;
use std::fs;
use std::io;
use std::io::Write;

const NUM_INCORRECT_GUESSES: u32 = 5;
const WORDS_PATH: &str = "words.txt";

fn pick_a_random_word() -> String {
    let file_string = fs::read_to_string(WORDS_PATH).expect("Unable to read file.");
    let words: Vec<&str> = file_string.split('\n').collect();
    String::from(words[rand::thread_rng().gen_range(0, words.len())].trim())
}

fn run(chars: &Vec<char>) {
    let mut guesses = NUM_INCORRECT_GUESSES;
    let mut guess_chars = vec!['-' ; chars.len()];
    let mut guessed_chars = Vec::new();

    let mut correct_char_num = 0;
    let correct_string: String = chars.iter().collect();
    let char_len = chars.len();

    println!("Welcom to CS110L Hangman!");

    let result = loop {
        if correct_char_num == char_len { 
            break format!("Congratulations you guessed the secret word: {}!", correct_string); 
        }

        if guesses <= 0 { break String::from("Sorry, you ran out of guesses!"); }

        let mut guess = String::new();
        
        print!("The word so far is ");
        for c in &guess_chars {
            print!("{}", c);
        }
        println!();

        print!("You have guessed the following letters: ");
        
        for c in &guessed_chars {
            print!("{}", c);
        }
        println!();

        println!("You have {} guesses left", guesses);
        print!("Please guess a letter: ");
        
        io::stdout().flush()
                    .expect("Error flushing stdout.");
        io::stdin()
            .read_line(&mut guess)
            .expect("Error reading line.");

        let guess_vec: Vec<char> = guess.chars().collect();
        guessed_chars.push(guess_vec[0]);

        let res: Result<(), &str> = {
            let mut found = false;
            for (i, c) in chars.iter().enumerate() {
                if guess_vec[0] == *c && guess_chars[i] == '-' {
                    guess_chars[i] = *c;
                    found = true;
                    break;
                }
            }
            match found {
                true => Ok(()),
                false => Err("Sorry, that letter is not in the word")
            }
        };
        match res {
            Ok(_) => {
                correct_char_num += 1;
            },
            Err(s) => {
                println!("{}", s);
                guesses -= 1;
            }
        }
        println!();
    };

    println!("{}", result);
}

fn main() {
    let secret_word = pick_a_random_word();
    // Note: given what you know about Rust so far, it's easier to pull characters out of a
    // vector than it is to pull them out of a string. You can get the ith character of
    // secret_word by doing secret_word_chars[i].
    let secret_word_chars: Vec<char> = secret_word.chars().collect();
    // Uncomment for debugging:
    println!("random word: {}", secret_word);

    // Your code here! :)
    run(&secret_word_chars);

}
