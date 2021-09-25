use crate::debugger_command::DebuggerCommand;
use crate::inferior::Inferior;
use crate::inferior::Status;
use crate::dwarf_data::{DwarfData, Error as DwarfError};
use rustyline::error::ReadlineError;
use rustyline::Editor;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Breakpoint {
    pub addr: usize,
    pub orig_byte: u8
}

pub struct Debugger {
    target: String,
    history_path: String,
    readline: Editor<()>,
    inferior: Option<Inferior>,
    debug_data: DwarfData,
    breakpoints: HashMap<usize, Breakpoint>,
    inferior_stopped_by_bp: bool,
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
            breakpoints: HashMap::new(),
            inferior_stopped_by_bp: false
        }
    }

    fn cont(&mut self) {
        if self.inferior.is_none() || 
            !self.inferior.as_mut().unwrap().running().unwrap() {
                println!("No running subprocess");
                return;
        }

        // At first, self.inferior_stopped_by_bp flag is false. When the inferior is stopped at a breakpoint,
        // this flag shoule be set to true. Then after the next `continue` command, self.inferior_stopped_by_bp
        // is true, and the byte in the memory at breakpoint address should be set to the original value, and %rip -= 1.
        // So the inferior can re-execute the next instruction, as if the breakpoint doesn't exist.\
        // 
        // Finally restore this breakpoint, which set the byte at breakpoint address to 0xcc, and clear flag.
        if self.inferior_stopped_by_bp {
            match self.inferior.as_mut().unwrap().step() {
                Ok(status) => {
                    match status {
                        Status::Exited(code) => {
                            println!("Child exited (status {})", code);
                            self.inferior = None;
                            return;
                        },
                        Status::Signaled(signal) => {
                            println!("Child signaled (signal {})", signal);
                            self.inferior = None;
                            return;
                        },
                        Status::Stopped(signal, rip) => {
                            if signal == nix::sys::signal::Signal::SIGTRAP {
                                self.reset_bp(rip);
                                self.inferior_stopped_by_bp = false;
                            }
                        }    
                    }
                },
                Err(e) => {
                    println!("Error stepping inferior ({:?})", e);
                    self.inferior = None;
                    self.inferior_stopped_by_bp = false;
                    return;
                }
            }
            
        }

        // Continue 
        match self.inferior.as_mut().unwrap().cont() {
            Ok(status) => {
                match status {
                    Status::Exited(code) => {
                        println!("Child exited (status {})", code);
                        self.inferior_stopped_by_bp = false;
                        self.inferior = None;
                    },
                    Status::Signaled(signal) => {
                        println!("Child signaled (signal {})", signal);
                        self.inferior_stopped_by_bp = false;
                        self.inferior = None;
                    },
                    Status::Stopped(signal, rip) => {
                        println!("Child stopped (signal {})", signal);

                        if let Some(line) = self.debug_data.get_line_from_addr(rip) {
                            println!("Stopped at {}", line);
                        }

                        // Check breakpoint
                        if signal == nix::sys::signal::Signal::SIGTRAP {
                            self.restore_bp(rip);
                            self.inferior_stopped_by_bp = true;
                        }
                    }
                }
            },
            Err(_) => {
                println!("Error continuing subprocess");
            }
        }
    }

    fn reset_bp(&mut self, rip: usize) {
        // Set the breakpoint
        if let Some(breakpoint) = self.breakpoints.get_mut(&(rip - 1)) {
            breakpoint.orig_byte = self.inferior.as_mut().unwrap()
                .write_byte(breakpoint.addr, 0xcc)
                .expect(&format!("Reset breakpoint at {} failed", breakpoint.addr));
        }
    }

    fn restore_bp(&mut self, rip: usize) -> Option<()> {
        // Now rip == breakpoint_addr + 1;
        if let Some(breakpoint) = self.breakpoints.get(&(rip - 1)) {
            // Restore the breakpoint
            let inferior = self.inferior.as_mut().unwrap();
            inferior.write_byte(breakpoint.addr, breakpoint.orig_byte)
                    .expect(&format!("Restore breakpoint at {} failed", breakpoint.addr));
            inferior.step_back_rip().unwrap();
            return Some(())
        }
        None
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
                    // If the inferior exists and is running, kill it.
                    if self.inferior.is_some() && 
                        self.inferior.as_mut().unwrap().running().unwrap() {
                        self.inferior.as_mut().unwrap()
                                     .kill().unwrap();
                    }
                    if let Some(inferior) = Inferior::new(&self.target, &args, &mut self.breakpoints) {
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
                DebuggerCommand::Breakpoint(token) => {
                    self.set_bp(token);
                },
                DebuggerCommand::Next => {
                    
                    if let Some(inferior) = &self.inferior {
                        let line_number = self.debug_data.get_line_from_addr(
                            inferior.get_rip().unwrap()
                        );
                        if line_number.is_none() {
                            println!("Error get current line number");
                            break;
                        }
                        let old_line_number = line_number.unwrap();
                        
                        loop {
                            match self.inferior.as_mut().unwrap().step() {
                                Err(e) => {
                                    println!("Error next command: {:?}", e);
                                },
                                Ok(status) => {
                                    match status {
                                        Status::Exited(code) => {
                                            println!("Child exited (status {})", code);
                                            self.inferior = None;
                                            break;
                                        },
                                        Status::Signaled(signal) => {
                                            println!("Child signaled (signal {})", signal);
                                            self.inferior = None;
                                            break;
                                        },
                                        Status::Stopped(signal, rip) => {
                                            // println!("{}", self.inferior_stopped_by_bp);
                                            if self.inferior_stopped_by_bp {
                                                self.reset_bp(rip);
                                                self.inferior_stopped_by_bp = false;
                                            }
                                            if signal == nix::sys::signal::Signal::SIGTRAP {
                                                // println!("rip: {:#x}", rip);
                                                if self.restore_bp(rip).is_some() {
                                                    // Stopped at a breakpoint
                                                    println!("stopped at a breakpoint");
                                                    self.inferior_stopped_by_bp = true;
                                                    break;
                                                } else {
                                                    // Just a step, get the line number
                                                    if let Some(line_number) = self.debug_data.get_line_from_addr(rip) {
                                                        // println!("line_number: {}, old: {}", line_number.number, old_line_number.number);
                                                        if line_number.number == old_line_number.number + 1 {
                                                            // Reach the next line, stop.
                                                            break;
                                                        }
                                                    }
                                                    // Continue to execute the next instruction.
                                                }
                                            } else {
                                                // Other signals, stop execution;
                                                println!("Child stopped (signal {})", signal);
                                                break;
                                            }
                                        }   
                                    }
                                }
                            }
                        }
                    } else {
                        println!("No running subprocess!");
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

    fn set_bp(&mut self, token: String) {

        let bp_addr: Option<usize>;
        if token.starts_with("*") {
            // address
            bp_addr = Debugger::parse_address(&token[1..]);
        } else if let Some(line_number) = token.parse::<usize>().ok() {
            // line number
            bp_addr = self.debug_data.get_addr_for_line(None, line_number);
        } else {
            // function name
            bp_addr = self.debug_data.get_addr_for_function(None, &token);
        }

        if bp_addr.is_none() {
            println!("Invalid breakpoint!");
            return;
        }
        
        let addr = bp_addr.unwrap();
        let mut breakpoint = Breakpoint { addr: addr, orig_byte: 0};
                
        if self.inferior.is_some() {
            match self.inferior.as_mut().unwrap().write_byte(addr, 0xcc) {
                Ok(orig_byte) => { breakpoint.orig_byte = orig_byte },
                Err(_) => {
                    println!("Error setting breakpoint at {}", addr);
                    return;
                }
            }
        }
        
        println!("Set breakpoint {} at {:#x}", self.breakpoints.len(), addr);
        
        breakpoint.addr = addr;
        self.breakpoints.insert(addr, breakpoint);
        return;
    }
}
