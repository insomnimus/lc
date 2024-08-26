mod cmd;
mod job;
mod work;

fn main() {
	if let Err(e) = cmd::Cmd::from_args().run() {
		eprintln!("error: {}", e);
		std::process::exit(2);
	}
}
