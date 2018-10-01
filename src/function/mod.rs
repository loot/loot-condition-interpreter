use std::path::PathBuf;

use regex::Regex;

pub mod eval;
pub mod parse;

#[derive(Debug, PartialEq, Eq)]
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
    FileRegex(Regex),
    ActivePath(PathBuf),
    ActiveRegex(Regex),
    Many(Regex),
    ManyActive(Regex),
    Checksum(PathBuf, u32),
    Version(PathBuf, String, ComparisonOperator),
}
