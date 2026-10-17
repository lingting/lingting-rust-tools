mod app;
mod apply;
mod cli;
mod common;
mod config;
mod front;
mod git;
mod junction;
mod mode;
mod plan;
mod rust;

fn main() {
    // 调试构建默认输出异常回溯
    #[cfg(debug_assertions)]
    unsafe {
        // main 入口此处仍是单线程，设置环境变量不会与其它线程竞争
        std::env::set_var("RUST_BACKTRACE", "1");
    }

    if let Err(error) = app::run() {
        eprintln!("执行异常! {error:#}");

        #[cfg(debug_assertions)]
        eprintln!("{}", std::backtrace::Backtrace::capture());

        std::process::exit(1);
    }
}
