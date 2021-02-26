use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;

#[derive(Debug)]
pub enum FileError {
  DecodeError,
  // EncodeError,
  SaveError(String),
  FileLoadError,
  NotFound,
}

impl ToString for FileError {
  fn to_string(&self) -> String {
    match self {
      FileError::DecodeError => "Decode error".into(),
      // FileError::EncodeError => "Encode error".into(),
      FileError::SaveError(e) => format!("Save error {}", e),
      FileError::FileLoadError => "File load error".into(),
      FileError::NotFound => "A megadott file nem található!".into(),
    }
  }
}

pub fn base64_decode(input: &str) -> Result<Vec<u8>, FileError> {
  base64::decode(input).map_err(|_| FileError::DecodeError)
}

pub fn base64_encode(input: &Vec<u8>) -> String {
  base64::encode(input)
}

pub fn save_file(bytes: Vec<u8>, path: PathBuf) -> Result<(), FileError> {
  let mut file = File::create(path).map_err(|e| FileError::SaveError(e.to_string()))?;
  file
    .write_all(&bytes)
    .map_err(|e| FileError::SaveError(e.to_string()))?;
  file
    .flush()
    .map_err(|e| FileError::SaveError(e.to_string()))?;
  Ok(())
}

pub fn load_invoice_base64(id: &str) -> Result<String, FileError> {
  let mut file = File::open(format!("data/{}/{}.pdf", crate::PDF_FOLDER_NAME, id))
    .map_err(|_| FileError::NotFound)?;

  let mut file_buf: Vec<u8> = Vec::new();

  file
    .read_to_end(&mut file_buf)
    .map_err(|_| FileError::FileLoadError)?;

  Ok(base64_encode(&file_buf))
}
