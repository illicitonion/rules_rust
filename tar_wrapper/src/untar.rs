use std::path::Path;
use tar_wrapper::untar;

fn main() {
    let mut args: Vec<_> = std::env::args_os().collect();
    if args.len() != 3 {
        eprintln!("Usage: {:?} path/to/tar.gz path/to/untar/to", args[0]);
        std::process::exit(1);
    }
    let output_dir = args.pop().unwrap();
    let tar_path = args.pop().unwrap();
    untar(&Path::new(&tar_path), &Path::new(&output_dir)).expect(&format!(
        "Failed to untar {:?} to {:?}",
        tar_path, output_dir
    ));
}
