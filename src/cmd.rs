use std::{
	error::Error,
	io::{
		self,
		BufRead,
	},
	process,
	sync::mpsc,
	thread,
};

use atty::Stream;
use ignore::WalkState::Continue;
use rayon::ThreadPoolBuilder;

use crate::{
	app,
	job::Job,
	work,
};

pub struct Cmd {
	args: Vec<String>,
	no_ignore: bool,
	quiet: bool,
	follow_links: bool,
	n_jobs: usize,
	read_stdin: bool,
}

impl Cmd {
	pub fn from_args() -> Self {
		let m = app::new().get_matches();
		if !atty::is(Stream::Stdin) {
			return Self {
				args: Vec::new(),
				n_jobs: 0,
				read_stdin: true,
				follow_links: false,
				quiet: false,
				no_ignore: false,
			};
		}

		let quiet = m.is_present("quiet");
		let args = match m.values_of("pattern") {
			Some(xs) => xs.map(String::from).collect::<Vec<_>>(),
			None => {
				eprintln!("error: you must specify at least one file/pattern");
				process::exit(2);
			}
		};

		let follow_links = m.is_present("follow-links");
		let n_jobs = m
			.value_of("jobs")
			.map(|s| s.parse::<usize>().unwrap())
			.unwrap_or(0);

		let no_ignore = m.is_present("no-ignore");

		Self {
			quiet,
			follow_links,
			args,
			n_jobs,
			no_ignore,
			read_stdin: false,
		}
	}
}

impl Cmd {
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
