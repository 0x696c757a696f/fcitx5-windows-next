use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let mut args = env::args_os().skip(1);
    let mut self_check = false;
    let mut report: Option<PathBuf> = None;

    while let Some(arg) = args.next() {
        if arg == "--self-check" {
            self_check = true;
        } else if arg == "--report" {
            let Some(path) = args.next() else {
                eprintln!("--report requires a path");
                std::process::exit(2);
            };
            report = Some(PathBuf::from(path));
        } else {
            eprintln!("unknown argument: {}", arg.to_string_lossy());
            std::process::exit(2);
        }
    }

    if !self_check {
        eprintln!("usage: fcitx5-candidate-poc --self-check [--report PATH]");
        std::process::exit(2);
    }

    match fcitx5_candidate_core::run_candidate_poc_self_check() {
        Ok(output) => {
            if let Some(path) = report {
                if let Some(parent) = path.parent() {
                    if let Err(error) = fs::create_dir_all(parent) {
                        eprintln!("failed to create report directory: {error}");
                        std::process::exit(1);
                    }
                }
                if let Err(error) = fs::write(&path, output.as_bytes()) {
                    eprintln!("failed to write report: {error}");
                    std::process::exit(1);
                }
                println!("candidate-poc-report={} result=PASS", path.display());
                return;
            }
            println!("{output}");
        }
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}
