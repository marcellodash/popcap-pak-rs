/// Pak Entry impl
pub mod entry;
/// Pak impl
pub mod pak;
pub(crate) mod reader;
pub(crate) mod writer;

pub use crate::{
    entry::Entry,
    pak::Pak,
};
use bstr::BString;

/// The maximum length of a file name, including path and slashes.
pub const MAX_NAME_LEN: usize = std::u8::MAX as usize;
/// The maximum data, in bytes, a single file in a pak file can hold.
pub const MAX_DATA_LEN: usize = std::u32::MAX as usize;
/// The magic number of a valid pak file. `[0xc0, 0x4a, 0xc0, 0xba]` XORed with `0xf7`, or "7½7M". This file type is often called "7x7M" as a result.
pub const MAGIC: &[u8] = &[0xc0, 0x4a, 0xc0, 0xba];
/// The version of pakfile that this library can read. `[0; 4]`.
pub const VERSION: &[u8] = &[0; 4];

const FILEFLAGS_END: u8 = 0x80;

const TICKS_PER_SECOND: i64 = 10_000_000;
const TICKS_PER_NANOSECOND: u32 = 100;
const MS_FILETIME_START_SECS: i64 = -11_644_473_600;
const MS_FILETIME_START_TICKS: i64 = MS_FILETIME_START_SECS * TICKS_PER_SECOND;

const PATH_SEPERATOR_BYTESET: &[u8] = b"\\/";

/// Result type of this library
pub type PakResult<T> = Result<T, PakError>;

/// Error type of this library
#[derive(Debug)]
pub enum PakError {
    /// IO errors that may occur during use
    Io(std::io::Error),

    /// Invalid Magic number. See [`MAGIC`].
    InvalidMagic([u8; 4]),
    /// Invalid Pak Version. See [`VERSION`].
    InvalidVersion([u8; 4]),

    /// The filename is too long. See [`MAX_NAME_LEN`].
    InvalidNameLength(usize),
    /// The data is too long. See [`MAX_DATA_LEN`].
    InvalidDataLength(usize),
}

impl From<std::io::Error> for PakError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

#[derive(Debug)]
struct Record {
    pub name: BString,
    pub file_size: u32,
    pub filetime: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use bstr::ByteSlice;
    use std::path::Path;

    const EXTRACT_PATH: &str = "test-extract";
    const PAK_PATH: &str = "test_data/Simple Building.pak";

    fn extract(pak: &mut Pak, extract_dir: &Path) {
        let _ = std::fs::remove_dir_all(extract_dir);

        for entry in pak.entries.iter_mut() {
            println!("Extracting '{}'...", entry.path());
            if let Some(dir) = entry.dir() {
                let entry_extract_dir = extract_dir.join(dir.to_path_lossy());
                std::fs::create_dir_all(&entry_extract_dir).unwrap();
            }

            let entry_extract_path = extract_dir.join(entry.path().to_path_lossy());
            let mut f = std::fs::File::create(&entry_extract_path).unwrap();
            std::io::copy(entry, &mut f).unwrap();
        }
    }

    #[test]
    fn extract_read() {
        let f = std::fs::File::open(PAK_PATH).unwrap();
        let mut p = Pak::from_read(f).unwrap();
        let extract_dir = Path::new(EXTRACT_PATH).join("read");

        extract(&mut p, &extract_dir);
    }

    #[test]
    fn extract_bytes() {
        let data = std::fs::read(PAK_PATH).unwrap();
        let mut p = Pak::from_bytes(&data).unwrap();
        let extract_dir = Path::new(EXTRACT_PATH).join("bytes");

        extract(&mut p, &extract_dir);
    }

    #[test]
    fn bytes_vs_read() {
        let data = std::fs::read(PAK_PATH).unwrap();
        let read = Pak::from_read(std::io::Cursor::new(&data)).unwrap();
        let bytes = Pak::from_bytes(&data).unwrap();
        assert_eq!(bytes, read);
    }

    #[test]
    fn round_trip() {
        let original = std::fs::read(PAK_PATH).unwrap();
        let mut pak = Pak::from_read(std::io::Cursor::new(&original)).unwrap();
        let mut round = Vec::new();
        pak.write_to(&mut round).unwrap();
        assert_eq!(&round, &original);
        let pak2 = Pak::from_read(std::io::Cursor::new(&round)).unwrap();
        assert_eq!(pak, pak2);
    }
}
