use std::{
	error::Error,
	io::{
		self,
		BufRead,
	},
	path::{
		Component,
		Path,
	},
	process,
	sync::mpsc,
	thread,
};

use atty::Stream;
use glob::MatchOptions;
use ignore::{
	overrides::OverrideBuilder,
	WalkBuilder,
};
use rayon::ThreadPoolBuilder;

use crate::{
	app,
	work,
};

pub struct Cmd {
	args: Vec<String>,
	no_ignore: bool,
	depth: Option<usize>,
	quiet: bool,
	follow_links: bool,
	n_jobs: usize,
	read_stdin: bool,
}

impl Cmd {
	pub fn from_args() -> Self {
		let m = app::new().get_matches();
		if atty::is(Stream::Stdin) {
			return Self {
				args: Vec::new(),
				n_jobs: 0,
				read_stdin: true,
				depth: None,
				follow_links: false,
				quiet: false,
				no_ignore: false,
			};
		}

		let quiet = m.is_present("quiet");
		let args = match m.values_of("pattern") {
			Some(xs) => xs.map(String::from).collect::<Vec<_>>(),
			None => {
				println!("{}", app::new().get_about().unwrap());
				process::exit(0);
			}
		};

		let follow_links = m.is_present("follow-links");
		let n_jobs = m
			.value_of("jobs")
			.map(|s| s.parse::<usize>().unwrap())
			.unwrap_or(0);

		let depth = m.value_of("depth").map(|s| s.parse::<usize>().unwrap());

		let no_ignore = m.is_present("no-ignore");

		Self {
			depth,
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
	pub fn run(self) -> Result<(), Box<dyn Error>> {
		if self.read_stdin {
			return read_stdin();
		}

		let Self {
			mut args,
			n_jobs,
			depth,
			follow_links,
			quiet,
			no_ignore,
			..
		} = self;

		ThreadPoolBuilder::new()
			.num_threads(n_jobs)
			.build_global()
			.unwrap();

		let (jobs, recv) = mpsc::channel();

		let mut overrides = OverrideBuilder::new("./");
		overrides.case_insensitive(true).unwrap();

		args.retain(|s| {
			if is_include(s) {
				overrides.add(s).unwrap();
				false
			} else {
				true
			}
		});

		let overrides = overrides.build()?;

		if !overrides.is_empty() {
			let walker_jobs = jobs.clone();

			thread::spawn(move || {
				WalkBuilder::new("./")
					.overrides(overrides)
					.threads(n_jobs)
					.max_depth(depth)
					.follow_links(follow_links)
					.standard_filters(!no_ignore)
					.build_parallel()
					.run(move || {
						let walker_jobs = walker_jobs.clone();
						Box::new(move |p| {
							match p {
								Ok(p) => walker_jobs.send(p.into_path()).unwrap(),
								Err(_) if quiet => (),
								Err(e) => eprintln!("error: {}", e),
							};
							ignore::WalkState::Continue
						})
					});
			});
		}

		if !args.is_empty() {
			thread::spawn(move || {
				for p in args
					.iter()
					.map(|s| {
						const OPTS: MatchOptions = MatchOptions {
							require_literal_separator: true,
							require_literal_leading_dot: true,
							case_sensitive: false,
						};

						glob::glob_with(s, OPTS)
							.unwrap_or_else(|e| {
								eprintln!("error parsing glob pattern: {}", e);
								process::exit(2);
							})
							.filter_map(|r| match r {
								Ok(x) => Some(x),
								Err(_) if quiet => None,
								Err(e) => {
									eprintln!("error: {}", e);
									None
								}
							})
					})
					.flatten()
				{
					jobs.send(p).unwrap();
				}
			});
		}

		work::work(recv, quiet);
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

fn is_include(p: impl AsRef<Path>) -> bool {
	let p = p.as_ref();

	match p.to_str() {
		Some(s) if !s.contains('*') => return false,
		_ => (),
	};

	let mut comps = p.components();
	match comps.next() {
		None => true,
		Some(c) => matches!(c, Component::Normal(_)),
	}
}
