use std::process::Command;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::time::Instant;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandResult {
    pub command: String,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub summary: String,
    pub finished_at: Instant,
}

pub struct CommandRunner {
    pub running_cmd: Option<String>,
    pub last_result: Option<CommandResult>,
    rx: Option<Receiver<CommandResult>>,
    tx: Sender<CommandResult>,
}

impl CommandRunner {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        Self {
            running_cmd: None,
            last_result: None,
            rx: Some(rx),
            tx,
        }
    }

    pub fn is_running(&self) -> bool {
        self.running_cmd.is_some()
    }

    pub fn run_task(&mut self, name: &str, program: &str, args: &[&str], working_dir: &str) -> bool {
        if self.is_running() {
            eprintln!("A task is already running: {:?}", self.running_cmd);
            return false;
        }

        self.running_cmd = Some(name.to_string());
        let tx = self.tx.clone();
        let cmd_name = name.to_string();
        let prog = program.to_string();
        let arg_vec: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        let cwd = working_dir.to_string();

        thread::spawn(move || {
            let mut cmd = Command::new(&prog);
            cmd.args(&arg_vec);
            if !cwd.is_empty() && cwd != "." {
                cmd.current_dir(&cwd);
            }

            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
            }

            let start = Instant::now();
            let output = cmd.output();
            let duration = start.elapsed();

            let result = match output {
                Ok(out) => {
                    let success = out.status.success();
                    let exit_code = out.status.code();
                    let summary = if success {
                        format!("Passed in {:.1}s", duration.as_secs_f32())
                    } else {
                        format!("Failed (code {:?}) in {:.1}s", exit_code, duration.as_secs_f32())
                    };

                    CommandResult {
                        command: cmd_name,
                        success,
                        exit_code,
                        summary,
                        finished_at: Instant::now(),
                    }
                }
                Err(err) => CommandResult {
                    command: cmd_name,
                    success: false,
                    exit_code: None,
                    summary: format!("Launch error: {}", err),
                    finished_at: Instant::now(),
                },
            };

            let _ = tx.send(result);
        });

        true
    }

    pub fn poll_result(&mut self) -> Option<CommandResult> {
        if let Some(ref rx) = self.rx {
            if let Ok(res) = rx.try_recv() {
                self.running_cmd = None;
                self.last_result = Some(res.clone());
                return Some(res);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_runner_initial_state() {
        let runner = CommandRunner::new();
        assert!(!runner.is_running());
        assert!(runner.last_result.is_none());
    }
}
