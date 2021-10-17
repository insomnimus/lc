use std::{
	fs::File,
	io::{
		BufRead,
		BufReader,
		Result,
	},
	path::PathBuf,
	sync::mpsc::Receiver,
};

use rayon::iter::{
	ParallelBridge,
	ParallelIterator,
};

pub fn work(jobs: Receiver<PathBuf>, quiet: bool) {
	let total: Vec<_> = jobs
		.into_iter()
		.par_bridge()
		.filter_map(move |p| match count_lines(p) {
			Ok(n) => Some(n),
			Err(_) if quiet => None,
			Err(e) => {
				eprintln!("error: {}", e);
				None
			}
		})
		.collect();

	let n_files = total.len();
	let n_lines: usize = total.iter().copied().sum();

	println!(
		"{} {} in {} {}",
		n_lines,
		if n_lines == 1 { "line" } else { "lines" },
		n_files,
		if n_files == 1 { "file" } else { "files" },
	);
}

fn count_lines(p: PathBuf) -> Result<usize> {
	let file = File::open(&p)?;
	let mut reader = BufReader::with_capacity(1024, file);
	let mut buf = Vec::with_capacity(1024);
	let mut total = 0_usize;
	while reader.read_until(b'\n', &mut buf)? > 0 {
		total += 1;
	}

	Ok(total)
}
