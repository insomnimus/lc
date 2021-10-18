use std::path::{
	Path,
	PathBuf,
};

use ignore::{
	overrides::OverrideBuilder,
	WalkBuilder,
};

pub enum Job {
	File(PathBuf),
	Walk(WalkBuilder),
}

impl Job {
	pub fn new(s: &str) -> Self {
		if !is_glob(s) {
			return Self::File(PathBuf::from(s));
		}

		let p = PathBuf::from(s);
		let comps = p.components();
		let comps = comps.collect::<Vec<_>>();

		// Find the index of the first glob.
		let idx = comps
			.iter()
			.position(|c| c.as_os_str().to_str().map_or(false, is_glob))
			.expect("internal logic error: expected a glob but there was none");

		let glob = comps[idx..].iter().collect::<PathBuf>();
		let glob = glob.as_os_str().to_str().unwrap();

		Self::Walk(if glob.contains("**") {
			// No depth limit.
			if idx == 0 {
				walker("./", glob, 0)
			} else {
				let root = comps[..idx].iter().collect::<PathBuf>();
				walker(&root, glob, 0)
			}
		} else {
			let depth = comps.len() - idx;
			let mut root = comps[..idx].iter().collect::<PathBuf>();
			if idx == 0 {
				root.push("./");
			}
			walker(&root, glob, depth)
		})
	}
}

fn walker<P: AsRef<Path>>(root: P, glob: &str, depth: usize) -> WalkBuilder {
	let mut ov = OverrideBuilder::new(root.as_ref());
	let _ = ov.case_insensitive(true);

	if let Err(e) = ov.add(glob) {
		eprintln!("pattern error: {}", e);
		std::process::exit(2);
	};

	let ov = ov.build().unwrap();

	let mut walk = WalkBuilder::new(root.as_ref());

	walk.max_depth(if depth == 0 { None } else { Some(depth) })
		.overrides(ov);

	walk
}

fn is_glob(s: &str) -> bool {
	s.chars().any(|c| c == '*' || c == '?' || c == '[')
}
