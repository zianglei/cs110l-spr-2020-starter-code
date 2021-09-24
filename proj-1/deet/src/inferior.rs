use nix::sys::ptrace;
use nix::sys::signal;
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use nix::unistd::Pid;
use std::os::unix::process::CommandExt;
use std::process::{Child, Command};
use std::mem::size_of;
use std::collections::HashMap;
use crate::debugger::Breakpoint;
use crate::dwarf_data::{DwarfData};

pub enum Status {
    /// Indicates inferior stopped. Contains the signal that stopped the process, as well as the
    /// current instruction pointer that it is stopped at.
    Stopped(signal::Signal, usize),

    /// Indicates inferior exited normally. Contains the exit status code.
    Exited(i32),

    /// Indicates the inferior exited due to a signal. Contains the signal that killed the
    /// process.
    Signaled(signal::Signal),
}

/// This function calls ptrace with PTRACE_TRACEME to enable debugging on a process. You should use
/// pre_exec with Command to call this in the child process.
fn child_traceme() -> Result<(), std::io::Error> {
    ptrace::traceme().or(Err(std::io::Error::new(
        std::io::ErrorKind::Other,
        "ptrace TRACEME failed",
    )))
}

fn align_addr_to_word(addr: usize) -> usize {
    addr & (-(size_of::<usize>() as isize) as usize)
}

pub struct Inferior {
    child: Child,
}

impl Inferior {
    /// Attempts to start a new inferior process. Returns Some(Inferior) if successful, or None if
    /// an error is encountered.
    pub fn new(target: &str, args: &Vec<String>, breakpoints: & mut HashMap<usize, Breakpoint>) -> Option<Inferior> {
        let mut cmd = Command::new(target);
        cmd.args(args);
        
        unsafe {
            cmd.pre_exec(child_traceme);
        }
        
        let child = cmd.spawn().ok()?;
        let mut inferior = Inferior { child };

        match waitpid(nix::unistd::Pid::from_raw(inferior.child.id() as i32), None).ok()? {
            WaitStatus::Stopped(_pid, _sig) => {
                // The target is actually loaded, add breakpoints
                for (baddr, breakpoint) in breakpoints {
                    match inferior.write_byte(*baddr, 0xcc) {
                        Err(_) => {
                            println!("Unable to set breakpoint at {}", baddr);
                            return None;
                        },
                        Ok(orig_byte) => { breakpoint.orig_byte = orig_byte; }
                    }
                }
                Some(inferior)
            }
            _ => {
                return None
            }
        }
    }

    /// Returns the pid of this inferior.
    pub fn pid(&self) -> Pid {
        nix::unistd::Pid::from_raw(self.child.id() as i32)
    }

    /// Calls waitpid on this inferior and returns a Status to indicate the state of the process
    /// after the waitpid call.
    pub fn wait(&self, options: Option<WaitPidFlag>) -> Result<Status, nix::Error> {
        Ok(match waitpid(self.pid(), options)? {
            WaitStatus::Exited(_pid, exit_code) => Status::Exited(exit_code),
            WaitStatus::Signaled(_pid, signal, _core_dumped) => Status::Signaled(signal),
            WaitStatus::Stopped(_pid, signal) => {
                let regs = ptrace::getregs(self.pid())?;
                Status::Stopped(signal, regs.rip as usize)
            }
            other => panic!("waitpid returned unexpected status: {:?}", other),
        })
    }

    /// Wakes up this inferior and waits until the inferior stops or terminates.
    pub fn cont(&self) -> Result<Status, nix::Error> {
        ptrace::cont(self.pid(), None)?;
        self.wait(None)
    }

    /// Kills this inferior and waits it to exit.
    pub fn kill(&mut self) -> Result<Status, nix::Error> {
        self.child.kill().unwrap();
        println!("Killing running inferior (pid {})", self.pid());
        self.wait(None)
    }

    /// Check if this inferior is running
    pub fn running(&mut self) -> Result<bool, nix::Error> {
        Ok(match self.child.try_wait() {
            Ok(Some(_)) => false,
            Ok(None) => true,
            Err(e) => panic!("try_wait returned unexpected err: {:?}", e)
        })
    }

    /// Print this inferior's backtrace using debugging symbols
    pub fn print_backtrace(&self, debug_data: &DwarfData) -> Result<(), nix::Error> {
        let pid = self.pid();
        let mut rip = ptrace::getregs(pid)?.rip as usize;
        let mut rbp = ptrace::getregs(pid)?.rbp as usize;
        loop {
            let func_name = debug_data.get_function_from_addr(rip).ok_or(nix::Error::Sys(nix::errno::Errno::EINVAL))?;
            let func_line = debug_data.get_line_from_addr(rip).ok_or(nix::Error::Sys(nix::errno::Errno::EINVAL))?;
            println!("{} ({})", func_name, func_line);
            if func_name == "main" { break; }
            rip = ptrace::read(pid, (rbp + 8) as ptrace::AddressType)? as usize;
            rbp = ptrace::read(pid, rbp as ptrace::AddressType)? as usize;
        }
        Ok(())
    }

    pub fn write_byte(&mut self, addr: usize, val: u8) -> Result<u8, nix::Error> {
        let aligned_addr = align_addr_to_word(addr);
        let byte_offset = addr - aligned_addr;
        let word = ptrace::read(self.pid(), aligned_addr as ptrace::AddressType)? as u64;
        let orig_byte = (word >> 8 * byte_offset) & 0xff;
        let masked_word = word & !(0xff << 8 * byte_offset);
        let updated_word = masked_word | ((val as u64) << 8 * byte_offset);
        ptrace::write(
            self.pid(),
            aligned_addr as ptrace::AddressType,
            updated_word as *mut std::ffi::c_void,
        )?;
        Ok(orig_byte as u8)
    }

    pub fn step_back_rip(&mut self) -> Result<(), nix::Error> {
        let mut regs = ptrace::getregs(self.pid())?;
        regs.rip = regs.rip - 1;
        ptrace::setregs(self.pid(), regs)
    }

    pub fn step(&mut self) -> Result<Status, nix::Error> {
        ptrace::step(self.pid(), None)?;
        self.wait(None)
    }
}