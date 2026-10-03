mod app;
mod apply;
mod cli;
mod command;
mod config;
mod front;
mod git;
mod junction;
mod mode;
mod plan;
mod rust;
mod source_edit;
mod text_file;

fn main() {
    if let Err(error) = app::run() {
        eprintln!("执行异常! {error:#}");
        std::process::exit(1);
    }
}
