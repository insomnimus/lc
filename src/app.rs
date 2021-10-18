use clap::{
	crate_version,
	App,
	Arg,
};

pub fn new() -> App<'static> {
	let app = App::new("lc")
		.version(crate_version!())
		.about("Count lines in files.");

	let args = Arg::new("pattern")
		.about("Files or glob patterns to count lines of.")
		.multiple_values(true);

	let no_ignore = Arg::new("no-ignore")
		.about("Do not use .gitignore and .ignore files.")
		.short('n')
		.long("no-ignore");

	let depth = Arg::new("depth")
		.about("Maximum recursion depth.")
		.short('d')
		.long("depth")
		.takes_value(true)
		.validator(is_positive);

	let n_jobs = Arg::new("jobs")
		.about("Number of parallel jobs. Defaults to the number of cpus.")
		.short('j')
		.long("jobs")
		.takes_value(true)
		.validator(is_positive);

	let follow_links = Arg::new("follow-links")
		.about("Follow symbolic links.")
		.short('f')
		.long("follow-links");

	let quiet = Arg::new("quiet")
		.about("Do not display non-fatal errors.")
		.short('q')
		.long("quiet");

	app.arg(follow_links)
		.arg(quiet)
		.arg(no_ignore)
		.arg(n_jobs)
		.arg(depth)
		.arg(args)
}

fn is_positive(s: &str) -> Result<(), String> {
	match s.parse::<usize>() {
		Err(_) | Ok(0) => Err(String::from("the value must be a positive number")),
		Ok(_) => Ok(()),
	}
}
