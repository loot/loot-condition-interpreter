use std::cmp::Ordering;
use std::path::Path;

use Error;

#[derive(Debug)]
pub struct Version {
    string: String,
}

impl Version {
    pub fn read_file_version(file_path: &Path) -> Result<Self, Error> {
        // TODO: Actually read the file's File Version field.
        Ok(Version {
            string: format!("{}", file_path.display()),
        })
    }
}

impl<'a> From<&'a str> for Version {
    fn from(string: &'a str) -> Self {
        Version {
            string: string.to_string(),
        }
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Version) -> Option<Ordering> {
        // TODO: Compare with same behaviour as pseudosem.
        self.string.partial_cmp(&other.string)
    }
}

impl PartialEq for Version {
    fn eq(&self, other: &Version) -> bool {
        // TODO: Compare with same behaviour as pseudosem.
        self.string == other.string
    }
}
