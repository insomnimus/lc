mod app;
mod cmd;
mod work;

fn main() {
	if let Err(e) = cmd::Cmd::from_args().run() {
		eprintln!("error: {}", e);
		std::process::exit(2);
	}
}
