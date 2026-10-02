// ZIP files, without a ZIP library: reading what people drop on the page (a
// ZIP of pictures) and writing what they download (a ZIP of icons or PNGs).
// Reading handles stored and deflated entries, which is what every ZIP tool
// makes; encrypted files, ZIP64 and other compression methods are refused with
// a clear message. Everything read is size-limited, so a small ZIP cannot claim
// gigabytes (a "zip bomb"). Writing stores the files uncompressed: pictures and
// icons are compressed already.

use crate::validate::crc32;

const END_OF_CENTRAL_DIRECTORY: u32 = 0x0605_4b50;
const CENTRAL_ENTRY: u32 = 0x0201_4b50;
const LOCAL_ENTRY: u32 = 0x0403_4b50;

/// Most entries a ZIP may list before it is refused.
pub const MAX_ENTRIES: usize = 2000;

/// Most bytes one entry may unpack to.
pub const MAX_ENTRY_BYTES: usize = 120_000_000;

#[derive(Debug, Clone)]
struct RawEntry {
    name: String,
    method: u16,
    flags: u16,
    crc: u32,
    compressed: u32,
    size: u32,
    offset: u32,
}

/// A ZIP file that has been read: its entries can be listed and unpacked one
/// by one.
#[derive(Debug, Clone)]
pub struct ZipArchive {
    bytes: Vec<u8>,
    entries: Vec<RawEntry>,
}

/// One file in a ZIP, as listed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZipFile {
    /// The position to ask `ZipArchive::read` for.
    pub index: usize,
    /// The path inside the ZIP, with `/` between folders.
    pub name: String,
    /// The size when unpacked.
    pub size: u64,
}

fn u16_at(bytes: &[u8], at: usize) -> Option<u16> {
    bytes
        .get(at..at + 2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
}

fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    bytes
        .get(at..at + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

const BROKEN: &str = "This is not a readable ZIP file (it is damaged or cut off).";

impl ZipArchive {
    /// Reads the table of contents of the ZIP in `bytes`.
    pub fn open(bytes: Vec<u8>) -> Result<ZipArchive, String> {
        // The end-of-central-directory record is the last thing in the file,
        // possibly followed by a comment of up to 65535 bytes.
        let search_from = bytes.len().saturating_sub(22 + 65_535);
        let eocd = (search_from..=bytes.len().saturating_sub(22))
            .rev()
            .find(|&at| u32_at(&bytes, at) == Some(END_OF_CENTRAL_DIRECTORY))
            .ok_or_else(|| BROKEN.to_string())?;

        let count = u16_at(&bytes, eocd + 10).ok_or(BROKEN)? as usize;
        let directory_size = u32_at(&bytes, eocd + 12).ok_or(BROKEN)?;
        let directory_offset = u32_at(&bytes, eocd + 16).ok_or(BROKEN)?;
        if count == 0xFFFF || directory_size == 0xFFFF_FFFF || directory_offset == 0xFFFF_FFFF {
            return Err("ZIP64 files (over 4 GB or 65535 files) are not supported.".to_string());
        }
        if count > MAX_ENTRIES {
            return Err(format!(
                "The ZIP lists {count} files; at most {MAX_ENTRIES} are accepted."
            ));
        }

        let mut entries = Vec::with_capacity(count);
        let mut at = directory_offset as usize;
        for _ in 0..count {
            if u32_at(&bytes, at) != Some(CENTRAL_ENTRY) {
                return Err(BROKEN.to_string());
            }
            let name_length = u16_at(&bytes, at + 28).ok_or(BROKEN)? as usize;
            let extra_length = u16_at(&bytes, at + 30).ok_or(BROKEN)? as usize;
            let comment_length = u16_at(&bytes, at + 32).ok_or(BROKEN)? as usize;
            let name_bytes = bytes.get(at + 46..at + 46 + name_length).ok_or(BROKEN)?;
            entries.push(RawEntry {
                // Names are UTF-8 in practice (flag bit 11); anything else is
                // shown as well as it can be.
                name: String::from_utf8_lossy(name_bytes).replace('\\', "/"),
                flags: u16_at(&bytes, at + 8).ok_or(BROKEN)?,
                method: u16_at(&bytes, at + 10).ok_or(BROKEN)?,
                crc: u32_at(&bytes, at + 16).ok_or(BROKEN)?,
                compressed: u32_at(&bytes, at + 20).ok_or(BROKEN)?,
                size: u32_at(&bytes, at + 24).ok_or(BROKEN)?,
                offset: u32_at(&bytes, at + 42).ok_or(BROKEN)?,
            });
            at += 46 + name_length + extra_length + comment_length;
        }
        Ok(ZipArchive { bytes, entries })
    }

    /// The files in the ZIP: folders, the `__MACOSX` resource forks that
    /// macOS adds, and hidden files (names starting with a dot) are left out.
    pub fn files(&self) -> Vec<ZipFile> {
        self.entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| {
                let base = entry.name.rsplit('/').next().unwrap_or("");
                !entry.name.ends_with('/')
                    && !base.is_empty()
                    && !base.starts_with('.')
                    && !entry.name.split('/').any(|part| part == "__MACOSX")
            })
            .map(|(index, entry)| ZipFile {
                index,
                name: entry.name.clone(),
                size: u64::from(entry.size),
            })
            .collect()
    }

    /// Unpacks entry `index` (as listed by `files`).
    pub fn read(&self, index: usize) -> Result<Vec<u8>, String> {
        let entry = self
            .entries
            .get(index)
            .ok_or_else(|| format!("The ZIP has no file number {index}."))?;
        let name = &entry.name;
        if entry.flags & 1 != 0 {
            return Err(format!(
                "'{name}' is encrypted; encrypted ZIPs are not supported."
            ));
        }
        if entry.size as usize > MAX_ENTRY_BYTES {
            return Err(format!(
                "'{name}' is {:.0} MB unpacked; at most {} MB per file are accepted.",
                f64::from(entry.size) / 1e6,
                MAX_ENTRY_BYTES / 1_000_000
            ));
        }

        let local = entry.offset as usize;
        if u32_at(&self.bytes, local) != Some(LOCAL_ENTRY) {
            return Err(format!("'{name}' is damaged in the ZIP."));
        }
        let name_length = u16_at(&self.bytes, local + 26).ok_or(BROKEN)? as usize;
        let extra_length = u16_at(&self.bytes, local + 28).ok_or(BROKEN)? as usize;
        let start = local + 30 + name_length + extra_length;
        let packed = self
            .bytes
            .get(start..start + entry.compressed as usize)
            .ok_or_else(|| format!("'{name}' is cut off in the ZIP."))?;

        let data = match entry.method {
            0 => packed.to_vec(),
            8 => miniz_oxide::inflate::decompress_to_vec_with_limit(packed, MAX_ENTRY_BYTES)
                .map_err(|e| format!("'{name}' could not be unpacked: {:?}.", e.status))?,
            other => {
                return Err(format!(
                    "'{name}' uses compression method {other}, which is not supported (only stored and deflated)."
                ));
            }
        };
        if data.len() != entry.size as usize || crc32(&data) != entry.crc {
            return Err(format!("'{name}' is damaged: its checksum does not match."));
        }
        Ok(data)
    }
}

/// Writes a ZIP holding `files` (name, contents), uncompressed.
pub fn write_zip(files: &[(String, Vec<u8>)]) -> Result<Vec<u8>, String> {
    if files.len() >= 0xFFFF {
        return Err("Too many files for one ZIP.".to_string());
    }
    let mut out: Vec<u8> = Vec::new();
    let mut directory: Vec<u8> = Vec::new();
    for (name, data) in files {
        let offset = u32::try_from(out.len()).map_err(|_| "The ZIP would be over 4 GB.")?;
        let size = u32::try_from(data.len()).map_err(|_| "A file is over 4 GB.")?;
        let crc = crc32(data);
        let name_bytes = name.as_bytes();
        let name_length = u16::try_from(name_bytes.len())
            .map_err(|_| format!("The name '{name}' is too long."))?;
        // Bit 11 of the flags: the name is UTF-8.
        let flags: u16 = 1 << 11;

        out.extend_from_slice(&LOCAL_ENTRY.to_le_bytes());
        out.extend_from_slice(&20u16.to_le_bytes()); // version needed
        out.extend_from_slice(&flags.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // stored
        out.extend_from_slice(&0u16.to_le_bytes()); // time
        out.extend_from_slice(&0x0021u16.to_le_bytes()); // date: 1980-01-01
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(&size.to_le_bytes());
        out.extend_from_slice(&size.to_le_bytes());
        out.extend_from_slice(&name_length.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // extra
        out.extend_from_slice(name_bytes);
        out.extend_from_slice(data);

        directory.extend_from_slice(&CENTRAL_ENTRY.to_le_bytes());
        directory.extend_from_slice(&20u16.to_le_bytes()); // made by
        directory.extend_from_slice(&20u16.to_le_bytes()); // version needed
        directory.extend_from_slice(&flags.to_le_bytes());
        directory.extend_from_slice(&0u16.to_le_bytes());
        directory.extend_from_slice(&0u16.to_le_bytes());
        directory.extend_from_slice(&0x0021u16.to_le_bytes());
        directory.extend_from_slice(&crc.to_le_bytes());
        directory.extend_from_slice(&size.to_le_bytes());
        directory.extend_from_slice(&size.to_le_bytes());
        directory.extend_from_slice(&name_length.to_le_bytes());
        directory.extend_from_slice(&0u16.to_le_bytes()); // extra
        directory.extend_from_slice(&0u16.to_le_bytes()); // comment
        directory.extend_from_slice(&0u16.to_le_bytes()); // disk
        directory.extend_from_slice(&0u16.to_le_bytes()); // internal attributes
        directory.extend_from_slice(&0u32.to_le_bytes()); // external attributes
        directory.extend_from_slice(&offset.to_le_bytes());
        directory.extend_from_slice(name_bytes);
    }

    let directory_offset = u32::try_from(out.len()).map_err(|_| "The ZIP would be over 4 GB.")?;
    let directory_size =
        u32::try_from(directory.len()).map_err(|_| "The ZIP would be over 4 GB.")?;
    out.extend_from_slice(&directory);
    out.extend_from_slice(&END_OF_CENTRAL_DIRECTORY.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&(files.len() as u16).to_le_bytes());
    out.extend_from_slice(&(files.len() as u16).to_le_bytes());
    out.extend_from_slice(&directory_size.to_le_bytes());
    out.extend_from_slice(&directory_offset.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes()); // comment length
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A ZIP made by Python's zipfile (deflated, with a folder, a macOS
    /// resource fork, a hidden file and one stored file).
    const PYTHON_ZIP: &str = "504b0304140000000800079c425d00000000020000000000000007000000696d616765732f0300504b0304140000000800079c425d5fb6101f0e000000180100000f000000696d616765732f6c6f676f2e706e670bf07377710c710c18a502502800504b0304140000000800079c425d074e6a2b1200000025000000090000006e6f7465732e747874cb48cdc9c95728cf2fca49d151c8c0c10100504b0304140000000800079c425d399cfb060600000004000000130000005f5f4d41434f53582f2e5f6c6f676f2e706e67cb2acdcb0600504b0304140000000800079c425d8316dc8c0300000001000000070000002e68696464656eab0000504b0304140000000000000021007877a60f0c0000000c0000000a00000073746f7265642e62696e73746f726564206279746573504b01021400140000000800079c425d000000000200000000000000070000000000000000001000fd4100000000696d616765732f504b01021400140000000800079c425d5fb6101f0e000000180100000f0000000000000000000000800127000000696d616765732f6c6f676f2e706e67504b01021400140000000800079c425d074e6a2b12000000250000000900000000000000000000008001620000006e6f7465732e747874504b01021400140000000800079c425d399cfb06060000000400000013000000000000000000000080019b0000005f5f4d41434f53582f2e5f6c6f676f2e706e67504b01021400140000000800079c425d8316dc8c03000000010000000700000000000000000000008001d20000002e68696464656e504b01021400140000000000000021007877a60f0c0000000c0000000a00000000000000000000008001fa00000073746f7265642e62696e504b05060000000006000600570100002e0100000000";

    fn from_hex(text: &str) -> Vec<u8> {
        (0..text.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
            .collect()
    }

    #[test]
    fn a_zip_from_another_tool_is_read_and_its_clutter_left_out() {
        let zip = ZipArchive::open(from_hex(PYTHON_ZIP)).unwrap();
        let names: Vec<String> = zip.files().into_iter().map(|f| f.name).collect();
        assert_eq!(names, vec!["images/logo.png", "notes.txt", "stored.bin"]);
        let files = zip.files();
        assert_eq!(zip.read(files[0].index).unwrap(), b"PNGDATA".repeat(40));
        assert_eq!(
            zip.read(files[1].index).unwrap(),
            b"hello world, hello world, hello world"
        );
        assert_eq!(zip.read(files[2].index).unwrap(), b"stored bytes");
        assert_eq!(files[0].size, 280);
    }

    #[test]
    fn what_is_written_is_read_back() {
        let files = vec![
            ("a.png".to_string(), vec![1, 2, 3]),
            (
                "folder/b ü.ico".to_string(),
                (0..=255u8).collect::<Vec<u8>>(),
            ),
            ("empty.bin".to_string(), Vec::new()),
        ];
        let zip = ZipArchive::open(write_zip(&files).unwrap()).unwrap();
        let listed = zip.files();
        assert_eq!(listed.len(), 3);
        for (file, (name, data)) in listed.iter().zip(&files) {
            assert_eq!(&file.name, name);
            assert_eq!(&zip.read(file.index).unwrap(), data);
        }
        assert!(
            ZipArchive::open(write_zip(&[]).unwrap())
                .unwrap()
                .files()
                .is_empty()
        );
    }

    #[test]
    fn a_damaged_zip_is_refused_not_trusted() {
        assert!(ZipArchive::open(Vec::new()).is_err());
        assert!(ZipArchive::open(b"PK but not a zip".to_vec()).is_err());
        let bytes = from_hex(PYTHON_ZIP);
        // Cut off: the table of contents is gone.
        assert!(ZipArchive::open(bytes[..bytes.len() - 30].to_vec()).is_err());
        // A flipped byte inside a file: the checksum notices.
        let mut flipped = bytes.clone();
        let at = flipped
            .windows(12)
            .position(|w| w == b"stored bytes")
            .unwrap();
        flipped[at] ^= 0xFF;
        let zip = ZipArchive::open(flipped).unwrap();
        let index = zip.files().last().unwrap().index;
        assert!(zip.read(index).unwrap_err().contains("checksum"));
    }

    #[test]
    fn encrypted_zip64_and_unknown_methods_say_so() {
        let mut encrypted = write_zip(&[("a.bin".to_string(), vec![1])]).unwrap();
        // Set bit 0 of the general purpose flags in the central entry.
        let central = encrypted
            .windows(4)
            .rposition(|w| w == CENTRAL_ENTRY.to_le_bytes())
            .unwrap();
        encrypted[central + 8] |= 1;
        let zip = ZipArchive::open(encrypted).unwrap();
        assert!(zip.read(0).unwrap_err().contains("encrypted"));

        let mut method = write_zip(&[("a.bin".to_string(), vec![1])]).unwrap();
        let central = method
            .windows(4)
            .rposition(|w| w == CENTRAL_ENTRY.to_le_bytes())
            .unwrap();
        method[central + 10] = 12; // bzip2
        let zip = ZipArchive::open(method).unwrap();
        assert!(zip.read(0).unwrap_err().contains("method 12"));

        let mut zip64 = write_zip(&[("a.bin".to_string(), vec![1])]).unwrap();
        let eocd = zip64
            .windows(4)
            .rposition(|w| w == END_OF_CENTRAL_DIRECTORY.to_le_bytes())
            .unwrap();
        zip64[eocd + 16..eocd + 20].copy_from_slice(&[0xFF; 4]);
        assert!(ZipArchive::open(zip64).unwrap_err().contains("ZIP64"));
    }

    /// A ZIP with one deflated entry whose directory claims `claimed` bytes.
    fn deflated_zip(name: &str, packed: &[u8], claimed: u32) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&LOCAL_ENTRY.to_le_bytes());
        out.extend_from_slice(&[20, 0, 0, 0, 8, 0, 0, 0, 0x21, 0]);
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&(packed.len() as u32).to_le_bytes());
        out.extend_from_slice(&claimed.to_le_bytes());
        out.extend_from_slice(&(name.len() as u16).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(packed);
        let directory_offset = out.len() as u32;
        out.extend_from_slice(&CENTRAL_ENTRY.to_le_bytes());
        out.extend_from_slice(&[20, 0, 20, 0, 0, 0, 8, 0, 0, 0, 0x21, 0]);
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&(packed.len() as u32).to_le_bytes());
        out.extend_from_slice(&claimed.to_le_bytes());
        out.extend_from_slice(&(name.len() as u16).to_le_bytes());
        out.extend_from_slice(&[0; 12]);
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(name.as_bytes());
        let directory_size = out.len() as u32 - directory_offset;
        out.extend_from_slice(&END_OF_CENTRAL_DIRECTORY.to_le_bytes());
        out.extend_from_slice(&[0, 0, 0, 0, 1, 0, 1, 0]);
        out.extend_from_slice(&directory_size.to_le_bytes());
        out.extend_from_slice(&directory_offset.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out
    }

    #[test]
    fn a_zip_bomb_is_stopped_by_the_size_limit() {
        // 150 MB of zeros deflates to a small file; the directory claims 10 bytes.
        let packed = miniz_oxide::deflate::compress_to_vec(&vec![0u8; 150_000_000], 1);
        let archive = ZipArchive::open(deflated_zip("big.bin", &packed, 10)).unwrap();
        assert!(
            archive
                .read(0)
                .unwrap_err()
                .contains("could not be unpacked")
        );
    }

    #[test]
    fn an_entry_that_claims_too_much_is_refused_before_unpacking() {
        let archive = ZipArchive::open(deflated_zip("big.bin", &[3, 0], 4_000_000_000)).unwrap();
        assert!(archive.read(0).unwrap_err().contains("per file"));
    }
}
