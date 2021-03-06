use crate::{
    entry::Entry,
    reader::PakReader,
    writer::PakWriter,
    PakError,
    PakResult,
    FILEFLAGS_END,
    MAGIC,
    VERSION,
};
use byteorder::{
    WriteBytesExt,
    LE,
};
use std::{
    convert::TryInto,
    io::{
        Cursor,
        Read,
        Write,
    },
};

/// An In-memory pakfile. It may reference borrowed data to avoid decrypting the entire file in memory all at once.
#[derive(Debug, PartialEq)]
pub struct Pak<'a> {
    /// All Entries in this pakfile. This will likely become private in the future and replaced by safer ways to interact with entries.
    pub entries: Vec<Entry<'a>>,
}

impl<'a> Pak<'a> {
    /// Read a pakfile from a read source.
    /// Returns a pakfile with ownership over is data, decrypting it all upfront.
    /// [`Pak::from_bytes`] should probably be preferred, as it is faster to load upfront, though reading takes slightly longer
    /// and manipulations will incur memory allocations and expensisve copying/decryption.
    pub fn from_read<R: Read>(reader: R) -> Result<Pak<'static>, PakError> {
        let mut reader = PakReader::new(reader);
        reader.read_magic()?;
        reader.read_version()?;

        let records = reader.read_records()?;

        let mut entries = Vec::with_capacity(records.len());
        for record in records {
            let mut data = vec![0; record.file_size.try_into().unwrap()];
            reader.read_exact(&mut data)?;

            entries.push(Entry {
                path: record.name,
                filetime: record.filetime,
                data: Cursor::new(data.into()),
            });
        }

        Ok(Pak { entries })
    }

    /// Read a pakfile from a byte slice. Returns a pakfile that borrows sections of data from the slice.
    /// Compared to [`Pak::from_read`], this takes less time to load, but reading takes slightly longer and manipulations require copying and decrypting the data.
    pub fn from_bytes(bytes: &'a [u8]) -> Result<Pak<'a>, PakError> {
        let mut reader = PakReader::new(&*bytes);
        reader.read_magic()?;
        reader.read_version()?;

        let records = reader.read_records()?;

        let mut bytes = reader.into_reader();

        let mut entries = Vec::with_capacity(records.len());
        for record in records {
            let len = record.file_size as usize;
            let data = &bytes[..len];
            bytes = &bytes[len..];

            entries.push(Entry {
                path: record.name,
                filetime: record.filetime,
                data: Cursor::new(data.into()),
            });
        }

        Ok(Self { entries })
    }

    /// Takes ownership of all data, decrypting it and returing an [`Pak`] that is guaranteed to not refernce external data.
    pub fn into_owned(self) -> Pak<'static> {
        let entries = self
            .entries
            .into_iter()
            .map(|entry| entry.into_owned())
            .collect();

        Pak { entries }
    }

    /// Writes data to a writeable destination. This takes `&mut self` because at the end of this function, all files' cursors will be at the end of the stream.
    pub fn write_to<W: Write>(&mut self, writer: W) -> PakResult<()> {
        let mut writer = PakWriter::new(writer);
        writer.write_all(MAGIC)?;
        writer.write_all(VERSION)?;

        for entry in self.entries.iter() {
            writer.write_u8(0x00)?;
            writer.write_filename(entry.path.as_slice().into())?;
            writer.write_u32::<LE>(
                entry
                    .size()
                    .try_into()
                    .map_err(|_| PakError::InvalidDataLength(entry.size()))?,
            )?;
            writer.write_u64::<LE>(entry.filetime)?;
        }
        writer.write_u8(FILEFLAGS_END)?;

        for entry in self.entries.iter_mut() {
            std::io::copy(entry, &mut writer)?;
        }

        Ok(())
    }
}
