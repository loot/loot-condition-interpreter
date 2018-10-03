use std::path::PathBuf;

use regex::Regex;

pub mod eval;
pub mod parse;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComparisonOperator {
    Equal,
    NotEqual,
    LessThan,
    GreaterThan,
    LessThanOrEqual,
    GreaterThanOrEqual,
}

#[derive(Debug)]
pub enum Function {
    FilePath(PathBuf),
    FileRegex(PathBuf, Regex),
    ActivePath(PathBuf),
    ActiveRegex(Regex),
    Many(PathBuf, Regex),
    ManyActive(Regex),
    Checksum(PathBuf, u32),
    Version(PathBuf, String, ComparisonOperator),
}
