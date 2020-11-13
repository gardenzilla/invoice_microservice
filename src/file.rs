use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;

#[derive(Debug)]
pub enum FileError {
    DecodeError,
    SaveError(String),
}

pub fn base64_decode(input: &str) -> Result<Vec<u8>, FileError> {
    base64::decode(input).map_err(|_| FileError::DecodeError)
}

pub fn save_file(bytes: Vec<u8>, path: PathBuf) -> Result<(), FileError> {
    let mut file = File::create(path).map_err(|e| FileError::SaveError(e.to_string()))?;
    file.write_all(&bytes)
        .map_err(|e| FileError::SaveError(e.to_string()))?;
    file.flush()
        .map_err(|e| FileError::SaveError(e.to_string()))?;
    Ok(())
}
