use flate2::read::GzDecoder;
use std::fs::File;
use std::path::Path;
use tar::Archive;

pub fn untar(tar_path: &Path, output_dir: &Path) -> Result<(), std::io::Error> {
    let f = File::open(tar_path)?;
    let decoder = GzDecoder::new(f);
    let mut archive = Archive::new(decoder);
    archive.unpack(output_dir)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::untar;
    use std::env::temp_dir;
    use std::ffi::OsString;
    use std::fs::{DirEntry, create_dir, read_dir, read_to_string, remove_dir_all};
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn fails() {
        let mut dir = temp_dir();
        let now_millis = SystemTime::now().duration_since(UNIX_EPOCH).expect("You're not before 1970, surely?").as_millis();
        dir.push(format!("rules_rust-tar_wrapper-test-{}", now_millis));
        let dir = DeleteOnDrop(dir);

        create_dir(&dir.0).expect(&format!("Could not make dir {:?}", dir.0));
        untar(&Path::new("test/example.tar.gz"), &dir.0).expect("Error untaring");

        let dir_contents = get_dir_contents(&dir.0);
        assert_eq!(dir_contents.len(), 1);

        let dir_entry = &dir_contents[0];
        assert_eq!(dir_entry.file_name(), OsString::from("dir"));
        assert!(dir_entry.file_type().expect("Error getting file type").is_dir());

        let subdir_path = dir.0.join("dir");
        let subdir_contents = get_dir_contents(&subdir_path);
        assert_eq!(subdir_contents.len(), 1);
        let file_entry = &subdir_contents[0];
        assert_eq!(file_entry.file_name(), OsString::from("f"));
        assert!(file_entry.file_type().expect("Error getting file type").is_file());

        let file_path = subdir_path.join("f");
        let contents = read_to_string(file_path).expect("Error reading file");
        assert_eq!(&contents, "Yay pure rust\n");

    }

    fn get_dir_contents(dir: &Path) -> Vec<DirEntry> {
        let dir_contents: Result<Vec<DirEntry>, _> = read_dir(&dir).expect(&format!("Error calling read_dir on {:?}", dir)).collect();
        dir_contents.expect(&format!("Error reading dir {:?}", dir))
    }

    struct DeleteOnDrop(PathBuf);

    impl Drop for DeleteOnDrop {
        fn drop(&mut self) {
            let _ = remove_dir_all(&self.0);
        }
    }
}
