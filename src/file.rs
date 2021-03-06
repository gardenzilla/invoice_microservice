use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;

#[derive(Debug)]
pub enum FileError {
  DecodeError,
  // EncodeError,
  SaveError(String),
  NotFound,
}

impl ToString for FileError {
  fn to_string(&self) -> String {
    match self {
      FileError::DecodeError => "Decode error".into(),
      // FileError::EncodeError => "Encode error".into(),
      FileError::SaveError(e) => format!("Save error {}", e),
      FileError::NotFound => "A megadott file nem található!".into(),
    }
  }
}

pub fn base64_decode(input: &str) -> Result<Vec<u8>, FileError> {
  base64::decode(input).map_err(|_| FileError::DecodeError)
}

pub fn base64_encode(input: &[u8]) -> String {
  base64::encode(input)
}

pub async fn save_file(bytes: Vec<u8>, path: PathBuf) -> Result<(), FileError> {
  // Create file
  let mut file = File::create(path)
    .await
    .map_err(|e| FileError::SaveError(e.to_string()))?;

  // Write content
  file
    .write_all(&bytes)
    .await
    .map_err(|e| FileError::SaveError(e.to_string()))?;

  // Flush its content
  file
    .flush()
    .await
    .map_err(|e| FileError::SaveError(e.to_string()))?;

  // Return nothing when Ok
  Ok(())
}

pub async fn load_invoice_base64(id: &str) -> Result<String, FileError> {
  let id = id.to_owned();

  let mut file = File::open(format!("data/{}/{}.pdf", crate::PDF_FOLDER_NAME, id))
    .await
    .map_err(|_| FileError::NotFound)?;

  let mut file_buf: Vec<u8> = Vec::new();

  file
    .read_to_end(&mut file_buf)
    .await
    .map_err(|_| FileError::DecodeError)?;

  Ok(base64_encode(&file_buf))
}
