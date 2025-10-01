use clap::Parser;
use std::env;

fn set_working_dir_to_exe() {
    if let Ok(exe_path) = env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            env::set_current_dir(exe_dir).ok();
        }
    }
}

#[tokio::main]
async fn main() {
    set_working_dir_to_exe();
    let cli = ocrisp::cli::Cli::try_parse();

    match cli {
        Ok(cli) => {
            if cli.command.is_none() {
                ocrisp::gui::run_gui();
                return;
            }

            ocrisp::cli::run_cli(cli.command.expect("Could not run CLI")).await;
        }
        Err(error) => error.exit(),
    }
}
