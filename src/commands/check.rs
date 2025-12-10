use std::io;
use std::path::Path;

use crate::utils::hash::hash_file;

pub enum CheckResult {
    Match(),
    NoMatch {
        actual: String
    }
}

pub fn run(expected: &String, file: &Path) -> io::Result<CheckResult> {
    let actual: String = hash_file(file)?;
    let result: CheckResult = if actual == *expected { CheckResult::Match() } else {CheckResult::NoMatch { actual } };
    Ok(result)
}
