use std::env;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

fn main() {
    for path in env::args_os().skip(1).map(PathBuf::from) {
        dump_file(&path);
    }
}

fn dump_file(path: &Path) {
    let file = File::open(path).unwrap();

    // To parse with continue-on-error mode:
    let exif = kamadak_exif::Reader::new()
        .read_from_container(&mut BufReader::new(&file))
        .unwrap();
    println!("{}", path.display());
    for f in exif.fields() {
        println!(
            "  {}/{}: {}",
            f.ifd_num.index(),
            f.tag,
            f.display_value().with_unit(&exif)
        );
        println!("      {:?}", f.value);
    }
}
