use std::{
    fs::File,
    io::{BufReader, Read, Seek, SeekFrom},
    path::Path,
};

use crate::Error;

use super::{ReleaseId, Version};

const KEY_OFFSET: usize = 6;

struct StructHeaders {
    length: usize,
    value_length: usize,
}

enum ReadResult {
    Version(String),
    NewOffset(usize),
}

pub(super) fn read_pe_version<F>(
    file_path: &Path,
    read_from_version_info: F,
) -> Result<Option<Version>, Error>
where
    F: Fn(&[u8]) -> Result<Option<Version>, String>,
{
    let file = File::open(file_path).map_err(|e| Error::IoError(file_path.to_path_buf(), e))?;
    let mut reader = BufReader::new(file);

    let data = read_version_resource_data(&mut reader)
        .map_err(|e| Error::PeParsingError(file_path.to_path_buf(), e.into()))?;

    if let Some(data) = data {
        read_from_version_info(&data)
            .map_err(|e| Error::PeParsingError(file_path.to_path_buf(), e.into()))
    } else {
        Ok(None)
    }
}

// <https://coffi.readthedocs.io/en/latest/pecoff_v11.pdf>
// <https://0xrick.github.io/win-internals/pe3/>
fn read_version_resource_data<T: Read + Seek>(reader: &mut T) -> std::io::Result<Option<Vec<u8>>> {
    const DOS_MAGIC: &[u8; 2] = b"MZ";
    const PE_MAGIC: &[u8; 4] = b"PE\0\0";
    const PE_HEADER_OFFSET_OFFSET: u64 = 0x3C;

    let mut dos_magic = [0u8; 2];
    reader.read_exact(&mut dos_magic)?;

    if &dos_magic != DOS_MAGIC {
        return Err(std::io::Error::other("Unknown file magic"));
    }

    reader.seek(std::io::SeekFrom::Start(PE_HEADER_OFFSET_OFFSET))?;

    let mut pe_header_offset = [0u8; 2];
    reader.read_exact(&mut pe_header_offset)?;

    let pe_header_offset = u64::from(u16::from_le_bytes(pe_header_offset));

    reader.seek(std::io::SeekFrom::Start(pe_header_offset))?;

    let mut pe_magic = [0u8; 4];
    reader.read_exact(&mut pe_magic)?;

    if &pe_magic != PE_MAGIC {
        return Err(std::io::Error::other("Unexpected PE header magic bytes"));
    }

    let coff_header = CoffFileHeader::read(reader)?;

    let optional_header =
        OptionalHeader::read(reader, u64::from(coff_header.optional_header_size))?;

    let Some(resource_table_data_directory) = optional_header.resource_table_data_directory()
    else {
        return Ok(None);
    };

    for _ in 0..coff_header.number_of_sections {
        let entry = SectionTableEntry::read(reader)?;

        // The table entry offsets within the resource table are relative to
        // the start of the table, so it's easier to read the data into a buffer
        // and then seek around inside it.
        if let Some(table_data) = resource_table_data_directory.read_data(reader, &entry)? {
            return read_version_data(&entry, &table_data);
        }
    }

    Ok(None)
}

fn read_version_data(
    table_entry: &SectionTableEntry,
    resource_table_data: &[u8],
) -> std::io::Result<Option<Vec<u8>>> {
    let mut cursor = std::io::Cursor::new(&resource_table_data);

    let Some(version_data_entry) = read_resource_tables(&mut cursor)? else {
        return Ok(None);
    };

    // Unlike the table entry offsets, the version data's offset is given
    // relative to the start of the loaded executable's virtual address.
    let data_offset = version_data_entry.data_rva - table_entry.virtual_address;

    cursor.seek(SeekFrom::Start(u64::from(data_offset)))?;

    let mut version_data = vec![0; to_usize(version_data_entry.size)];
    cursor.read_exact(&mut version_data)?;

    Ok(Some(version_data))
}

/// This expects to be given a reader that's at the start of the resource table
/// data.
fn read_resource_tables<T: Read + Seek>(
    reader: &mut T,
) -> std::io::Result<Option<ResourceDataEntry>> {
    let root_table = ResourceDirectoryTable::read(reader)?;

    for root_entry in root_table.entries {
        if root_entry.name_offset_or_id == ResourceDirectoryEntry::RT_VERSION
            && root_entry.is_table()
        {
            reader.seek(SeekFrom::Start(u64::from(root_entry.offset())))?;

            let version_name_table = ResourceDirectoryTable::read(reader)?;

            for name_entry in version_name_table.entries {
                if name_entry.is_table() {
                    reader.seek(SeekFrom::Start(u64::from(name_entry.offset())))?;

                    let version_language_table = ResourceDirectoryTable::read(reader)?;

                    for language_entry in version_language_table.entries {
                        if !language_entry.is_table() {
                            reader.seek(SeekFrom::Start(u64::from(language_entry.offset())))?;

                            return ResourceDataEntry::read(reader).map(Some);
                        }
                    }
                }
            }
        }
    }

    Ok(None)
}

#[derive(Debug)]
struct CoffFileHeader {
    number_of_sections: u16,
    optional_header_size: u16,
}

impl CoffFileHeader {
    fn read<T: Read + Seek>(reader: &mut T) -> std::io::Result<Self> {
        let mut word = [0u8; 2];

        // Skip machine.
        reader.seek_relative(2)?;

        reader.read_exact(&mut word)?;
        let number_of_sections = u16::from_le_bytes(word);

        // Skip time_date_stamp, symbol_table_offset and number_of_symbols.
        reader.seek_relative(12)?;

        reader.read_exact(&mut word)?;
        let optional_header_size = u16::from_le_bytes(word);

        // Optional header is required for executables.
        if optional_header_size == 0 {
            return Err(std::io::Error::other(
                "COFF optional header is unexpectedly missing",
            ));
        }

        // Skip characteristics.
        reader.seek_relative(2)?;

        Ok(Self {
            number_of_sections,
            optional_header_size,
        })
    }
}

#[derive(Debug)]
struct OptionalHeader {
    image_data_directories: Vec<ImageDataDirectory>,
}

impl OptionalHeader {
    const PE32_MAGIC: u16 = 0x10b;
    const RESOURCE_TABLE_DATA_DIRECTORY_OFFSET: usize = 2;

    /// Ensure that reading the optional header is restricted to the declared
    /// size of the header, since otherwise an invalid number_of_rva_and_sizes
    /// field value could cause the reader to go past the declared end of the
    /// header.
    fn read<T: Read + Seek>(reader: &mut T, header_size: u64) -> std::io::Result<Self> {
        let reader = &mut reader.take(header_size);
        let mut word = [0u8; 2];

        reader.read_exact(&mut word)?;
        let magic = u16::from_le_bytes(word);

        // Skip many fields.
        if magic == Self::PE32_MAGIC {
            reader.seek_relative(90)?;
        } else {
            reader.seek_relative(106)?;
        }

        let mut dword = [0u8; 4];

        reader.read_exact(&mut dword)?;
        let number_of_rva_and_sizes = u32::from_le_bytes(dword);

        let mut image_data_directories = Vec::with_capacity(to_usize(number_of_rva_and_sizes));
        for _ in 0..number_of_rva_and_sizes {
            image_data_directories.push(ImageDataDirectory::read(reader)?);
        }

        Ok(OptionalHeader {
            image_data_directories,
        })
    }

    fn resource_table_data_directory(&self) -> Option<&ImageDataDirectory> {
        self.image_data_directories
            .get(OptionalHeader::RESOURCE_TABLE_DATA_DIRECTORY_OFFSET)
    }
}

#[derive(Debug)]
struct ImageDataDirectory {
    virtual_address: u32,
    size: u32,
}

impl ImageDataDirectory {
    fn read<T: Read>(reader: &mut T) -> std::io::Result<Self> {
        let mut dword = [0u8; 4];

        reader.read_exact(&mut dword)?;
        let virtual_address = u32::from_le_bytes(dword);

        reader.read_exact(&mut dword)?;
        let size = u32::from_le_bytes(dword);

        Ok(Self {
            virtual_address,
            size,
        })
    }

    fn read_data<T: Read + Seek>(
        &self,
        reader: &mut T,
        entry: &SectionTableEntry,
    ) -> std::io::Result<Option<Vec<u8>>> {
        if entry.contains(self) {
            let table_offset = self.virtual_address - entry.virtual_address;

            reader.seek(std::io::SeekFrom::Start(u64::from(
                entry.raw_data_offset + table_offset,
            )))?;

            let mut data = vec![0; to_usize(self.size)];
            reader.read_exact(&mut data)?;

            Ok(Some(data))
        } else {
            Ok(None)
        }
    }
}

#[derive(Debug)]
struct SectionTableEntry {
    virtual_size: u32,
    virtual_address: u32,
    raw_data_size: u32,
    raw_data_offset: u32,
}

impl SectionTableEntry {
    fn read<T: Read + Seek>(reader: &mut T) -> std::io::Result<Self> {
        let mut name = [0u8; 8];
        reader.read_exact(&mut name)?;

        let mut dword = [0u8; 4];

        reader.read_exact(&mut dword)?;
        let virtual_size = u32::from_le_bytes(dword);

        reader.read_exact(&mut dword)?;
        let virtual_address = u32::from_le_bytes(dword);

        reader.read_exact(&mut dword)?;
        let raw_data_size = u32::from_le_bytes(dword);

        reader.read_exact(&mut dword)?;
        let raw_data_offset = u32::from_le_bytes(dword);

        // Skip relocations_offset, line_numbers_offset, relocations_count,
        // line_numbers_count and characteristics.
        reader.seek_relative(16)?;

        Ok(Self {
            virtual_size,
            virtual_address,
            raw_data_size,
            raw_data_offset,
        })
    }

    fn contains(&self, image_data_directory: &ImageDataDirectory) -> bool {
        let section_end = self.virtual_address + self.actual_size();
        let directory_end = image_data_directory.virtual_address + image_data_directory.size;

        image_data_directory.virtual_address >= self.virtual_address && directory_end <= section_end
    }

    fn actual_size(&self) -> u32 {
        std::cmp::min(self.raw_data_size, self.virtual_size)
    }
}

#[derive(Debug)]
struct ResourceDirectoryTable {
    entries: Vec<ResourceDirectoryEntry>,
}

impl ResourceDirectoryTable {
    fn read<T: Read + Seek>(reader: &mut T) -> std::io::Result<Self> {
        // Skip characteristics, time_date_stamp, major_version and minor_version.
        reader.seek_relative(12)?;

        let mut word = [0u8; 2];

        reader.read_exact(&mut word)?;
        let name_entry_count = u16::from_le_bytes(word);

        reader.read_exact(&mut word)?;
        let id_entry_count = u16::from_le_bytes(word);

        let mut entries = Vec::with_capacity(usize::from(name_entry_count + id_entry_count));
        for _ in 0..name_entry_count {
            entries.push(ResourceDirectoryEntry::read(reader)?);
        }

        for _ in 0..id_entry_count {
            entries.push(ResourceDirectoryEntry::read(reader)?);
        }

        Ok(Self { entries })
    }
}

#[derive(Debug)]
struct ResourceDirectoryEntry {
    name_offset_or_id: u32,
    data_entry_or_subdirectory_offset: u32,
}

impl ResourceDirectoryEntry {
    const RT_VERSION: u32 = 16;

    fn read<T: Read>(reader: &mut T) -> std::io::Result<Self> {
        let mut dword = [0u8; 4];

        reader.read_exact(&mut dword)?;
        let name_offset_or_id = u32::from_le_bytes(dword);

        reader.read_exact(&mut dword)?;
        let data_entry_or_subdirectory_offset = u32::from_le_bytes(dword);

        Ok(Self {
            name_offset_or_id,
            data_entry_or_subdirectory_offset,
        })
    }

    fn is_table(&self) -> bool {
        (self.data_entry_or_subdirectory_offset & (1 << 31)) != 0
    }

    fn offset(&self) -> u32 {
        if self.is_table() {
            self.data_entry_or_subdirectory_offset ^ (1 << 31)
        } else {
            self.data_entry_or_subdirectory_offset
        }
    }
}

#[derive(Debug)]
struct ResourceDataEntry {
    data_rva: u32,
    size: u32,
}

impl ResourceDataEntry {
    fn read<T: Read + Seek>(reader: &mut T) -> std::io::Result<Self> {
        let mut dword = [0u8; 4];

        reader.read_exact(&mut dword)?;
        let data_rva = u32::from_le_bytes(dword);

        reader.read_exact(&mut dword)?;
        let size = u32::from_le_bytes(dword);

        // Skip codepage and reserved.
        reader.seek_relative(8)?;

        Ok(Self { data_rva, size })
    }
}

#[expect(
    clippy::as_conversions,
    reason = "A compile-time assertion ensures that this conversion will be lossless on all relevant target platforms"
)]
const fn to_usize(value: u32) -> usize {
    // Error at compile time if this conversion isn't lossless.
    const _: () = assert!(u32::BITS <= usize::BITS, "cannot fit a u32 into a usize!");
    value as usize
}

fn subslice(bytes: &[u8], offset: usize, size: usize) -> Result<&[u8], String> {
    bytes.get(offset..offset + size).ok_or_else(|| {
        format!("Invalid subslice of size {size} at offset {offset} of bytes {bytes:X?}")
    })
}

// <https://learn.microsoft.com/en-us/windows/win32/menurc/vs-versioninfo>
pub(super) fn read_file_version(data: &[u8]) -> Result<Option<Version>, String> {
    let StructHeaders { value_length, .. } = read_vs_version_info_headers(data)?;

    if value_length == 0 {
        Ok(None)
    } else {
        let fixed_file_info = subslice(data, 40, value_length)?;
        Some(read_vs_fixed_file_info(fixed_file_info)).transpose()
    }
}

fn read_vs_version_info_headers(data: &[u8]) -> Result<StructHeaders, String> {
    let headers = read_struct_headers(data)?;

    if headers.length != data.len() {
        return Err(format!(
            "Unexpected length of VS_VERSIONINFO struct, got {} but buffer length is {}",
            headers.length,
            data.len()
        ));
    }

    if !has_subslice_at(
        data,
        KEY_OFFSET,
        b"V\0S\0_\0V\0E\0R\0S\0I\0O\0N\0_\0I\0N\0F\0O\0\0\0",
    ) {
        return Err(format!(
            "The szKey field's value is not valid for a VS_VERSIONINFO struct: {data:X?}"
        ));
    }

    Ok(headers)
}

fn read_struct_headers(data: &[u8]) -> Result<StructHeaders, String> {
    let [l0, l1, vl0, vl1, ..] = data else {
        return Err(format!(
            "The buffer was too small to hold a struct: {data:X?}"
        ));
    };

    let length = usize::from(u16::from_le_bytes([*l0, *l1]));

    let value_length = usize::from(u16::from_le_bytes([*vl0, *vl1]));

    Ok(StructHeaders {
        length,
        value_length,
    })
}

fn has_subslice_at(haystack: &[u8], offset: usize, needle: &[u8]) -> bool {
    haystack
        .get(offset..offset + needle.len())
        .is_some_and(|s| s == needle)
}

// <learn.microsoft.com/en-us/windows/win32/api/verrsrc/ns-verrsrc-vs_fixedfileinfo>
fn read_vs_fixed_file_info(data: &[u8]) -> Result<Version, String> {
    const VS_FIXEDFILEINFO_SIZE: usize = 0x34;
    const VS_FIXEDFILEINFO_SIGNATURE: [u8; 4] = [0xBD, 0x04, 0xEF, 0xFE];
    const FILE_VERSION_OFFSET: usize = 8;
    const FILE_VERSION_LENGTH: usize = 8;

    if data.len() != VS_FIXEDFILEINFO_SIZE {
        return Err(format!(
            "Unexpected length of VS_VERSIONINFO value, got {} but expected {}",
            data.len(),
            VS_FIXEDFILEINFO_SIZE
        ));
    }

    if !has_subslice_at(data, 0, &VS_FIXEDFILEINFO_SIGNATURE) {
        return Err(format!(
            "Unexpected first four bytes of VS_VERSIONINFO struct {data:X?}"
        ));
    }

    let Some(
        [file_minor_0, file_minor_1, file_major_0, file_major_1, file_build_0, file_build_1, file_patch_0, file_patch_1],
    ) = data.get(FILE_VERSION_OFFSET..FILE_VERSION_OFFSET + FILE_VERSION_LENGTH)
    else {
        return Err(format!(
            "The buffer was too small to hold a VS_FIXEDFILEINFO struct: {data:X?}"
        ));
    };

    let file_minor = u16::from_le_bytes([*file_minor_0, *file_minor_1]);
    let file_major = u16::from_le_bytes([*file_major_0, *file_major_1]);
    let file_build = u16::from_le_bytes([*file_build_0, *file_build_1]);
    let file_patch = u16::from_le_bytes([*file_patch_0, *file_patch_1]);

    Ok(Version {
        release_ids: vec![
            ReleaseId::Numeric(u32::from(file_major)),
            ReleaseId::Numeric(u32::from(file_minor)),
            ReleaseId::Numeric(u32::from(file_patch)),
            ReleaseId::Numeric(u32::from(file_build)),
        ],
        pre_release_ids: Vec::new(),
    })
}

// <https://learn.microsoft.com/en-us/windows/win32/menurc/vs-versioninfo>
pub(super) fn read_product_version(data: &[u8]) -> Result<Option<Version>, String> {
    const CHILDREN_BASE_OFFSET: usize = 40;

    let StructHeaders {
        length,
        value_length,
    } = read_vs_version_info_headers(data)?;

    let mut children = subslice(
        data,
        CHILDREN_BASE_OFFSET + value_length,
        length - (CHILDREN_BASE_OFFSET + value_length),
    )?;

    while !children.is_empty() {
        let next_offset = match read_next_child(children)? {
            ReadResult::NewOffset(offset) => offset,
            ReadResult::Version(version) => return Ok(Some(Version::from(version))),
        };

        children = offset(children, next_offset)?;
    }

    Ok(None)
}

fn read_next_child(children: &[u8]) -> Result<ReadResult, String> {
    const STRING_FILE_INFO_KEY: &[u8; 30] = b"S\0t\0r\0i\0n\0g\0F\0i\0l\0e\0I\0n\0f\0o\0\0\0";

    let child_length = read_struct_size(children)?;

    if has_subslice_at(children, KEY_OFFSET, STRING_FILE_INFO_KEY) {
        // <https://learn.microsoft.com/en-us/windows/win32/menurc/stringfileinfo>
        const STRING_TABLES_OFFSET: usize = KEY_OFFSET + STRING_FILE_INFO_KEY.len();

        if child_length < STRING_TABLES_OFFSET {
            return Err(format!(
                "The StringFileInfo struct's header is too small: {child_length}"
            ));
        }

        let mut string_tables = subslice(
            children,
            STRING_TABLES_OFFSET,
            child_length - STRING_TABLES_OFFSET,
        )?;

        while !string_tables.is_empty() {
            let next_offset = match read_next_string_table(string_tables)? {
                ReadResult::NewOffset(offset) => offset,
                ReadResult::Version(version) => return Ok(ReadResult::Version(version)),
            };

            string_tables = offset(children, next_offset)?;
        }
    }

    Ok(ReadResult::NewOffset(new_aligned_offset(child_length)))
}

fn read_struct_size(buffer: &[u8]) -> Result<usize, String> {
    buffer
        .first_chunk::<2>()
        .map(|c| usize::from(u16::from_le_bytes(*c)))
        .ok_or_else(
            || format!("The buffer was too small to hold a struct size field: {buffer:X?}",),
        )
}

// <https://learn.microsoft.com/en-us/windows/win32/menurc/stringtable>
fn read_next_string_table(string_tables: &[u8]) -> Result<ReadResult, String> {
    const STRINGS_OFFSET: usize = 24;

    let string_table_length = read_struct_size(string_tables)?;

    if string_table_length < STRINGS_OFFSET {
        return Err(format!(
            "The StringTable struct's header is too small: {string_table_length}"
        ));
    }

    let mut strings = subslice(
        string_tables,
        STRINGS_OFFSET,
        string_table_length - STRINGS_OFFSET,
    )?;

    while !strings.is_empty() {
        let next_offset = match read_next_string(strings)? {
            ReadResult::NewOffset(offset) => offset,
            ReadResult::Version(version) => return Ok(ReadResult::Version(version)),
        };

        strings = offset(strings, next_offset)?;
    }

    Ok(ReadResult::NewOffset(new_aligned_offset(
        string_table_length,
    )))
}

// <https://learn.microsoft.com/en-us/windows/win32/menurc/string-str>
fn read_next_string(strings: &[u8]) -> Result<ReadResult, String> {
    const PRODUCT_VERSION_KEY: &[u8; 30] = b"P\0r\0o\0d\0u\0c\0t\0V\0e\0r\0s\0i\0o\0n\0\0\0";
    const VALUE_OFFSET: usize = KEY_OFFSET + PRODUCT_VERSION_KEY.len();

    let Ok(headers) = read_struct_headers(strings) else {
        return Err(format!(
            "The buffer was too small to hold a String struct: {strings:X?}"
        ));
    };

    if has_subslice_at(strings, KEY_OFFSET, PRODUCT_VERSION_KEY) {
        let string_bytes = subslice(strings, VALUE_OFFSET, headers.value_length * 2)?;
        let utf8_string = read_utf16_string(string_bytes).map_err(|e| e.to_string())?;
        return Ok(ReadResult::Version(utf8_string));
    }

    Ok(ReadResult::NewOffset(new_aligned_offset(headers.length)))
}

fn offset(bytes: &[u8], offset: usize) -> Result<&[u8], String> {
    bytes
        .get(offset..)
        .ok_or_else(|| format!("Failed to get subslice at offset {offset} of {bytes:X?}"))
}

fn new_aligned_offset(length_read: usize) -> usize {
    if length_read.is_multiple_of(4) {
        length_read
    } else {
        length_read + 2
    }
}

fn read_utf16_string(bytes: &[u8]) -> Result<String, std::string::FromUtf16Error> {
    // This could be made more efficient by checking alignment and transmuting
    // the slice if aligned, but that involves unsafe, and there isn't a
    // significant performance difference.
    let mut u16_vec: Vec<u16> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| u16::from_le_bytes(*c))
        .collect();

    // We don't want to keep the trailing null u16.
    u16_vec.pop();

    String::from_utf16(&u16_vec)
}
