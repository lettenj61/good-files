use std::convert::From;
use std::fs::{self, OpenOptions};
use std::io::{self, BufReader, BufWriter};
use std::io::prelude::*;
use std::ops;
use std::path::{Path, PathBuf};

/// `Open` has a responsible for handling file system entry
/// from path and other information, such as open options,
/// for further operation such as `io::Read` or `io::Write`.
pub trait Open {
    fn open<P: AsRef<Path>>(&self, path: P) -> io::Result<fs::File>;
}

/// `CreateMode` defines if a file must be created when it opens.
pub enum CreateMode {
    CreateNew,
    IfNotExists,
    Never,
}

/// `WriteOption` represents how an input would be handled
/// on further `Write` operation on a file.
pub enum WriteOption {
    Append,
    Overwrite,
    Truncate,
}

/// `FileOpener` is typical implementation of `Open` trait, handles that
/// how a file must be created, or what operations will be allowed on open file.
pub struct FileOpener(CreateMode, bool, Option<WriteOption>);

/// The `File` object wraps `PathBuf` and provides convenient functions
/// to perform I/O operation.
///
/// Not like its name, `File` is closer to `Path` struct than `std::fs::File`,
/// as it implements `Deref<Target=Path>`.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct File(PathBuf);

impl FileOpener {
    /// Open file for appending, fails if file does not exist.
    pub fn appending() -> Self {
        FileOpener(CreateMode::Never, false, Some(WriteOption::Append))
    }

    /// Open file for writing, create new file if the file does not exist.
    /// The content of file will be truncated.
    pub fn truncate() -> Self {
        FileOpener(CreateMode::IfNotExists, false, Some(WriteOption::Truncate))
    }

    /// Open file for writing, create new file if the file does not exist.
    /// The content of the file will be overwritten.
    pub fn overwrite() -> Self {
        FileOpener(CreateMode::IfNotExists, false, Some(WriteOption::Overwrite))
    }

    /// Open file for appending, create new file if the file does not exist.
    /// The content of the file will be preserved.
    pub fn append_or_create() -> Self {
        FileOpener(CreateMode::IfNotExists, false, Some(WriteOption::Append))
    }

    /// Open file for reading, fails if the file does not exist.
    pub fn readonly() -> Self {
        FileOpener(CreateMode::Never, true, None)
    }

    /// Construct owned `OpenOptions` from this opener.
    fn into_open_options(&self) -> OpenOptions {
        let mut opts = OpenOptions::new();
        // set creation mode
        match self.0 {
            CreateMode::CreateNew => {
                opts.create_new(true);
            }
            CreateMode::IfNotExists => {
                opts.create(true);
            }
            _ => {}
        }
        // set read option
        opts.read(self.1);
        // set write option
        match self.2 {
            Some(WriteOption::Append) => {
                opts.append(true);
            }
            Some(WriteOption::Overwrite) => {
                opts.write(true);
            }
            Some(WriteOption::Truncate) => {
                opts.truncate(true);
            }
            None => {}
        }
        opts
    }
}

impl Open for FileOpener {
    fn open<P: AsRef<Path>>(&self, path: P) -> io::Result<fs::File> {
        self.into_open_options().open(&path)
    }
}

impl File {
    /// Create a new owned `File` with given path.
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        File(path.as_ref().to_path_buf())
    }

    /// Open file with owned `Path` with given open options
    pub fn open_with<O: Open>(&self, opt: O) -> io::Result<fs::File> {
        opt.open(&self.0)
    }

    pub fn create_if_absent(&self) -> io::Result<fs::File> {
        self.open_with(FileOpener::append_or_create())
    }

    pub fn buf_reader(&self) -> io::Result<BufReader<fs::File>> {
        let f = FileOpener::readonly().open(&self.0)?;
        Ok(BufReader::new(f))
    }

    pub fn buf_writer<O: Open>(&self, opt: O) -> io::Result<BufWriter<fs::File>> {
        let f = opt.open(&self.0)?;
        Ok(BufWriter::new(f))
    }

    pub fn read_all(&self) -> io::Result<Vec<u8>> {
        let mut v = Vec::new();
        let mut r = self.buf_reader()?;
        r.read_to_end(&mut v)?;
        Ok(v)
    }

    pub fn read_string(&self) -> io::Result<String> {
        let mut s = String::new();
        let mut r = self.buf_reader()?;
        r.read_to_string(&mut s)?;
        Ok(s)
    }

    pub fn append(&self, buf: &[u8]) -> io::Result<()> {
        self.write_all_with(buf, FileOpener::appending())
    }

    pub fn overwrite(&self, buf: &[u8]) -> io::Result<()> {
        self.write_all_with(buf, FileOpener::overwrite())
    }

    pub fn truncate(&self, buf: &[u8]) -> io::Result<()> {
        self.write_all_with(buf, FileOpener::truncate())
    }

    pub fn write_all_with<O: Open>(&self, buf: &[u8], opt: O) -> io::Result<()> {
        let mut w = self.buf_writer(opt)?;
        w.write_all(buf)?;
        w.get_ref().sync_all()?;
        Ok(())
    }
}

impl AsRef<Path> for File {
    fn as_ref(&self) -> &Path {
        self.0.as_ref()
    }
}

impl Default for File {
    fn default() -> Self {
        File(PathBuf::new())
    }
}

impl ops::Deref for File {
    type Target = Path;

    fn deref(&self) -> &Path {
        self.0.as_ref()
    }
}

impl From<PathBuf> for File {
    fn from(path: PathBuf) -> File {
        File(path)
    }
}

#[cfg(test)]
mod tests {

    extern crate tempdir;

    use std::io::prelude::*;
    use std::path::Path;
    use self::tempdir::TempDir;
    use super::*;

    fn test_dir() -> io::Result<TempDir> {
        let dir = TempDir::new("good-files-test")?;
        Ok(dir)
    }

    #[test]
    fn file_object() {
        let f = File::new("/path/to/some/file");
        assert_eq!(Path::new("/path/to/some/file"), f.as_ref());
    }

    #[test]
    #[should_panic]
    fn open_readonly() {
        let tmp_dir = test_dir().unwrap();
        let path = tmp_dir.path().join("panics.txt");
        let mut f = FileOpener::readonly()
            .open(&path)
            .unwrap();
        let _ = f.write(b"this should never be written").unwrap();
    }

    #[test]
    fn read_write_ops() {
        // TODO: revisit when good-files' utilities are ready
        let tmp_dir = test_dir().unwrap();
        let path = tmp_dir.path().join("foo.txt");
        let f = File::new(&path);

        // writing
        f.overwrite(b"some text\n2nd line").unwrap();

        // reading
        let s = f.read_string().unwrap();
        assert_eq!("some text\n2nd line", &s);
    }
}
