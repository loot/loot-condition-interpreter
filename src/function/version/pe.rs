use std::path::Path;

use object::{
    pe::{ImageDataDirectory, ImageResourceDataEntry, IMAGE_DIRECTORY_ENTRY_RESOURCE},
    read::pe::{ImageNtHeaders, PeFile32, PeFile64, ResourceDirectoryEntryData},
    LittleEndian,
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

pub(super) fn read_version_info_data(file_path: &Path) -> Result<Option<Vec<u8>>, Error> {
    let file =
        std::fs::File::open(file_path).map_err(|e| Error::IoError(file_path.to_path_buf(), e))?;
    let reader = object::ReadCache::new(std::io::BufReader::new(file));

    let file_kind = object::FileKind::parse(&reader)
        .map_err(|e| Error::PeParsingError(file_path.to_path_buf(), Box::new(e)))?;

    match file_kind {
        object::FileKind::Pe32 => {
            let file = PeFile32::parse(&reader)
                .map_err(|e| Error::PeParsingError(file_path.to_path_buf(), Box::new(e)))?;

            read_version_info_data_from_file(&file)
                .map_err(|e| Error::PeParsingError(file_path.to_path_buf(), e))
        }
        object::FileKind::Pe64 => {
            let file = PeFile64::parse(&reader)
                .map_err(|e| Error::PeParsingError(file_path.to_path_buf(), Box::new(e)))?;

            read_version_info_data_from_file(&file)
                .map_err(|e| Error::PeParsingError(file_path.to_path_buf(), e))
        }
        _ => Err(Error::PeParsingError(
            file_path.to_path_buf(),
            "Invalid PE optional header magic".into(),
        )),
    }
}

fn read_version_info_data_from_file<'a, T: ImageNtHeaders, R: object::ReadRef<'a>>(
    file: &object::read::pe::PeFile<'a, T, R>,
) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error + Send + Sync + 'static>> {
    const RT_VERSION: u16 = 16;

    let Some(resources) = file
        .data_directories()
        .resource_directory(file.data(), &file.section_table())?
    else {
        return Ok(None);
    };

    let resources_root = resources.root()?;

    let Some(resource_directory_entry) =
        file.data_directories().get(IMAGE_DIRECTORY_ENTRY_RESOURCE)
    else {
        return Ok(None);
    };

    // The entries in the root level determine the resource type ID.
    for type_table_entry in resources_root.entries {
        if type_table_entry.name_or_id().id() == Some(RT_VERSION) {
            let data = type_table_entry.data(resources)?;

            if let ResourceDirectoryEntryData::Table(name_table) = data {
                // The entries in the second level determine the resource name.
                for name_table_entry in name_table.entries {
                    let data = name_table_entry.data(resources)?;

                    if let ResourceDirectoryEntryData::Table(language_table) = data {
                        // The entries in the second level determine the resource language.
                        for language_table_entry in language_table.entries {
                            let data = language_table_entry.data(resources)?;

                            if let ResourceDirectoryEntryData::Data(data) = data {
                                return read_version_table_data(
                                    file,
                                    *resource_directory_entry,
                                    data,
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(None)
}

fn read_version_table_data<'a, T: ImageNtHeaders, R: object::ReadRef<'a>>(
    file: &object::read::pe::PeFile<'a, T, R>,
    resource_directory_entry: ImageDataDirectory,
    data: &ImageResourceDataEntry,
) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error + Send + Sync + 'static>> {
    let bytes = resource_directory_entry.data(file.data(), &file.section_table())?;

    let offset_start = to_usize(
        data.offset_to_data.get(LittleEndian) - resource_directory_entry.address_range().0,
    );
    let bytes_subslice = subslice(bytes, offset_start, to_usize(data.size.get(LittleEndian)))?;

    Ok(Some(bytes_subslice.to_vec()))
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
