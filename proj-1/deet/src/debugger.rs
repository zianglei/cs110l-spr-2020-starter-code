use crate::debugger_command::DebuggerCommand;
use crate::inferior::Inferior;
use crate::inferior::Status;
use crate::dwarf_data::{DwarfData, Error as DwarfError};

use rustyline::error::ReadlineError;
use rustyline::Editor;

pub struct Debugger {
    target: String,
    history_path: String,
    readline: Editor<()>,
    inferior: Option<Inferior>,
    debug_data: DwarfData,
    breakpoints: Vec<usize>,
}

impl Debugger {
    /// Initializes the debugger.
    pub fn new(target: &str) -> Debugger {
        // Initialize the DwarfData
        let debug_data = match DwarfData::from_file(target) {
            Ok(val) => val,
            Err(DwarfError::ErrorOpeningFile) => {
                println!("Could not open file {}", target);
                std::process::exit(1);
            }
            Err(DwarfError::DwarfFormatError(err)) => {
                println!("Could not debugging symbols from {}: {:?}", target, err);
                std::process::exit(1);
            }
        };

        debug_data.print();
        
        let history_path = format!("{}/.deet_history", std::env::var("HOME").unwrap());
        let mut readline = Editor::<()>::new();
        // Attempt to load history from ~/.deet_history if it exists
        let _ = readline.load_history(&history_path);

        Debugger {
            target: target.to_string(),
            history_path,
            readline,
            inferior: None,
            debug_data,
            breakpoints: vec![]
        }
    }

    fn cont(&mut self) {
        if self.inferior.is_none() || 
            !self.inferior.as_mut().unwrap().running().unwrap() {
                println!("No running subprocess");
                return;
        }
        match self.inferior.as_mut().unwrap().cont() {
            Ok(status) => {
                match status {
                    Status::Exited(code) => {
                        println!("Child exited (status {})", code);
                        self.inferior = None;
                    },
                    Status::Signaled(signal) => {
                        println!("Child signaled (signal {})", signal);
                        self.inferior = None;
                    },
                    Status::Stopped(signal, rip) => {
                        println!("Child stopped (signal {})", signal);
                        if let Some(line) = self.debug_data.get_line_from_addr(rip) {
                            println!("Stopped at {}", line);
                        }
                    }
                }
            },
            Err(_) => {
                println!("Error continuing subprocess");
            }
        }
    }

    fn parse_address(addr: &str) -> Option<usize> {
        let addr_without_0x = if addr.to_lowercase().starts_with("0x") {
            &addr[2..]
        } else {
            &addr
        };
        usize::from_str_radix(addr_without_0x, 16).ok()
    }

    pub fn run(&mut self) {
        loop {
            match self.get_next_command() {
                DebuggerCommand::Run(args) => {
                    if self.inferior.is_some() && 
                        self.inferior.as_mut().unwrap().running().unwrap() {
                        self.inferior.as_mut().unwrap()
                                     .kill().unwrap();
                    }
                    if let Some(inferior) = Inferior::new(&self.target, &args, &self.breakpoints) {
                        // Create the inferior
                        self.inferior = Some(inferior);
                        // Wake up the inferior
                        self.cont();
                    } else {
                        println!("Error starting subprocess");
                    }
                }
                DebuggerCommand::Quit => {
                    if self.inferior.is_some() && 
                        self.inferior.as_mut().unwrap().running().unwrap() {
                        self.inferior.as_mut().unwrap().kill().unwrap();
                    }
                    return;
                },
                DebuggerCommand::Continue => {
                    self.cont();
                },
                DebuggerCommand::Backtrace => {
                    if let Some(inferior) = &self.inferior {
                        match inferior.print_backtrace(&self.debug_data) {
                            Err(e) => {
                                println!("Error printing backtrace: {:?}", e);
                            },
                            _ => { }
                        }
                    }
                },
                DebuggerCommand::Breakpoint(addr) => {

                    if !addr.starts_with("*") {
                        println!("Invalid address");
                        return;
                    }

                    if let Some(b_addr) = Debugger::parse_address(&addr[1..]) {
                        // If the inferior is some, add this new breakpoint
                        if self.inferior.is_some() {
                            match self.inferior.as_mut().unwrap().write_byte(b_addr, 0xcc) {
                                Ok(_) => {},
                                Err(_) => {
                                    println!("Error setting breakpoint at {}", b_addr);
                                    return;
                                }
                            }
                        }
                        
                        println!("Set breakpoint {} at {:#x}", self.breakpoints.len(), b_addr);
                        self.breakpoints.push(b_addr);
                        
                    } else {
                        println!("Invalid address");
                    }
                }
            }
        }
    }

    /// This function prompts the user to enter a command, and continues re-prompting until the user
    /// enters a valid command. It uses DebuggerCommand::from_tokens to do the command parsing.
    ///
    /// You don't need to read, understand, or modify this function.
    fn get_next_command(&mut self) -> DebuggerCommand {
        loop {
            // Print prompt and get next line of user input
            match self.readline.readline("(deet) ") {
                Err(ReadlineError::Interrupted) => {
                    // User pressed ctrl+c. We're going to ignore it
                    println!("Type \"quit\" to exit");
                }
                Err(ReadlineError::Eof) => {
                    // User pressed ctrl+d, which is the equivalent of "quit" for our purposes
                    return DebuggerCommand::Quit;
                }
                Err(err) => {
                    panic!("Unexpected I/O error: {:?}", err);
                }
                Ok(line) => {
                    if line.trim().len() == 0 {
                        continue;
                    }
                    self.readline.add_history_entry(line.as_str());
                    if let Err(err) = self.readline.save_history(&self.history_path) {
                        println!(
                            "Warning: failed to save history file at {}: {}",
                            self.history_path, err
                        );
                    }
                    let tokens: Vec<&str> = line.split_whitespace().collect();
                    if let Some(cmd) = DebuggerCommand::from_tokens(&tokens) {
                        return cmd;
                    } else {
                        println!("Unrecognized command.");
                    }
                }
            }
        }
    }
}
