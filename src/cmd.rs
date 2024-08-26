use std::{
	error::Error,
	io::{
		self,
		BufRead,
		IsTerminal,
	},
	sync::mpsc,
	thread,
};

use clap::Parser;
use ignore::WalkState::Continue;
use rayon::ThreadPoolBuilder;

use crate::{
	job::Job,
	work,
};

/// Count lines of files
#[derive(Parser)]
#[command(version)]
pub struct Cmd {
	/// Do not use .gitignore and .ignore files
	#[arg(short, long)]
	no_ignore: bool,
	/// Do not show non-fatal error messages
	#[arg(short, long)]
	quiet: bool,
	/// Follow symbolic links
	#[arg(short = 'l', long)]
	follow_links: bool,
	/// Number of parallel jobs
	#[arg(short = 'j', long = "jobs", default_value_t = 0)]
	n_jobs: usize,

	/// The files or glob patterns
	#[arg()]
	args: Vec<String>,
	/// Read lines from standard input instead (you don't need to use this if you're already piping to lc)
	#[arg(long)]
	read_stdin: bool,
}

impl Cmd {
	pub fn from_args() -> Self {
		let mut a = Self::parse();
		a.read_stdin = a.read_stdin || !io::stdin().is_terminal();
		a
	}

	pub fn run(&self) -> Result<(), Box<dyn Error>> {
		if self.read_stdin {
			return read_stdin();
		}

		ThreadPoolBuilder::new()
			.num_threads(self.n_jobs)
			.build_global()?;

		let (jobs, recv) = mpsc::channel();

		for s in &self.args {
			match Job::new(s) {
				Job::File(p) => jobs.send(p)?,
				Job::Walk(mut builder) => {
					let walker_jobs = jobs.clone();
					let walker = builder
						.standard_filters(!self.no_ignore)
						.threads(self.n_jobs)
						.follow_links(self.follow_links)
						.build_parallel();
					let quiet = self.quiet;

					thread::spawn(move || {
						walker.run(move || {
							let walker_jobs = walker_jobs.clone();
							Box::new(move |p| {
								let entry = match p {
									Ok(x) => x,
									Err(_) if quiet => return Continue,
									Err(e) => {
										eprintln!("error: {}", e);
										return Continue;
									}
								};
								if entry.file_type().map_or(false, |f| f.is_file()) {
									walker_jobs.send(entry.into_path()).unwrap();
								}
								Continue
							})
						});
					});
				}
			};
		}

		std::mem::drop(jobs);
		work::work(recv, self.quiet);
		Ok(())
	}
}

fn read_stdin() -> Result<(), Box<dyn Error>> {
	let stdin = io::stdin();
	let mut stdin = stdin.lock();
	let mut buf = Vec::with_capacity(1024);
	let mut total = 0_usize;
	while stdin.read_until(b'\n', &mut buf)? > 0 {
		total += 1;
	}
	if total == 1 {
		println!("1 line");
	} else {
		println!("{} lines", total);
	}

	Ok(())
}
