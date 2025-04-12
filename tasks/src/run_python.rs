use std::process::Command;
use common::scan_config::AppConfig;

pub fn run_python(python_path: &str, script: &str, args: &[&str]) -> std::io::Result<std::process::Output> {
    let mut cmd = Command::new(python_path);
    cmd.arg(script);
    cmd.args(args);
    
    Ok(cmd.output()?)
}