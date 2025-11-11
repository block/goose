use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

const GGUF_MAGIC: u32 = 0x46554747;
const GGUF_VERSION: u32 = 3; // We support version 3

/// This exists to let us read what context size to use for gguf models
/// Based on GGUF spec: <https://github.com/ggerganov/ggml/blob/master/docs/gguf.md>
#[derive(Debug)]
pub enum MetadataValue {
    U8(u8),
    I8(i8),
    U16(u16),
    I16(i16),
    U32(u32),
    I32(i32),
    U64(u64),
    I64(i64),
    F32(f32),
    F64(f64),
    Bool(bool),
    String(String),
    Array(Vec<MetadataValue>),
}

pub fn read_context_length(path: impl AsRef<Path>) -> Result<Option<u64>> {
    let metadata = read_metadata(path)?;

    for (key, value) in &metadata {
        if key.ends_with(".context_length") {
            if let Some(ctx_len) = value_to_u64(value) {
                tracing::info!(
                    "Found context length in GGUF metadata: {} = {}",
                    key,
                    ctx_len
                );
                return Ok(Some(ctx_len));
            }
        }
    }

    if let Some(value) = metadata.get("context_length") {
        if let Some(ctx_len) = value_to_u64(value) {
            tracing::info!(
                "Found context length in GGUF metadata: context_length = {}",
                ctx_len
            );
            return Ok(Some(ctx_len));
        }
    }

    tracing::debug!(
        "No context_length found in GGUF metadata. Available keys: {:?}",
        metadata.keys().collect::<Vec<_>>()
    );

    Ok(None)
}

fn value_to_u64(value: &MetadataValue) -> Option<u64> {
    match value {
        MetadataValue::U8(v) => Some(*v as u64),
        MetadataValue::U16(v) => Some(*v as u64),
        MetadataValue::U32(v) => Some(*v as u64),
        MetadataValue::U64(v) => Some(*v),
        MetadataValue::I8(v) if *v >= 0 => Some(*v as u64),
        MetadataValue::I16(v) if *v >= 0 => Some(*v as u64),
        MetadataValue::I32(v) if *v >= 0 => Some(*v as u64),
        MetadataValue::I64(v) if *v >= 0 => Some(*v as u64),
        _ => None,
    }
}

fn read_metadata(path: impl AsRef<Path>) -> Result<HashMap<String, MetadataValue>> {
    let file = File::open(path.as_ref())
        .with_context(|| format!("Failed to open GGUF file: {:?}", path.as_ref()))?;
    let mut reader = BufReader::new(file);

    let magic = read_u32(&mut reader)?;
    if magic != GGUF_MAGIC {
        anyhow::bail!(
            "Invalid GGUF magic number: expected 0x{:08X}, got 0x{:08X}",
            GGUF_MAGIC,
            magic
        );
    }

    let version = read_u32(&mut reader)?;
    if version != GGUF_VERSION {
        anyhow::bail!(
            "Unsupported GGUF version: {}. Only version {} is supported.",
            version,
            GGUF_VERSION
        );
    }

    let _tensor_count = read_u64(&mut reader)?;
    let metadata_count = read_u64(&mut reader)?;

    let mut metadata = HashMap::new();
    for _ in 0..metadata_count {
        let key = read_string(&mut reader)?;
        let value_type = read_u32(&mut reader)?;
        let value = read_metadata_value(&mut reader, value_type)?;
        metadata.insert(key, value);
    }

    Ok(metadata)
}

fn read_metadata_value<R: Read>(reader: &mut R, value_type: u32) -> Result<MetadataValue> {
    match value_type {
        0 => Ok(MetadataValue::U8(read_u8(reader)?)),
        1 => Ok(MetadataValue::I8(read_i8(reader)?)),
        2 => Ok(MetadataValue::U16(read_u16(reader)?)),
        3 => Ok(MetadataValue::I16(read_i16(reader)?)),
        4 => Ok(MetadataValue::U32(read_u32(reader)?)),
        5 => Ok(MetadataValue::I32(read_i32(reader)?)),
        6 => Ok(MetadataValue::F32(read_f32(reader)?)),
        7 => Ok(MetadataValue::Bool(read_bool(reader)?)),
        8 => Ok(MetadataValue::String(read_string(reader)?)),
        9 => {
            // Array type
            let array_type = read_u32(reader)?;
            let array_len = read_u64(reader)?;
            let mut values = Vec::new();
            for _ in 0..array_len {
                values.push(read_metadata_value(reader, array_type)?);
            }
            Ok(MetadataValue::Array(values))
        }
        10 => Ok(MetadataValue::U64(read_u64(reader)?)),
        11 => Ok(MetadataValue::I64(read_i64(reader)?)),
        12 => Ok(MetadataValue::F64(read_f64(reader)?)),
        _ => anyhow::bail!("Unknown metadata value type: {}", value_type),
    }
}

fn read_string<R: Read>(reader: &mut R) -> Result<String> {
    let len = read_u64(reader)?;
    let mut buf = vec![0u8; len as usize];
    reader.read_exact(&mut buf)?;
    String::from_utf8(buf).context("Invalid UTF-8 in string")
}

fn read_u8<R: Read>(reader: &mut R) -> Result<u8> {
    let mut buf = [0u8; 1];
    reader.read_exact(&mut buf)?;
    Ok(buf[0])
}

fn read_i8<R: Read>(reader: &mut R) -> Result<i8> {
    let mut buf = [0u8; 1];
    reader.read_exact(&mut buf)?;
    Ok(buf[0] as i8)
}

fn read_u16<R: Read>(reader: &mut R) -> Result<u16> {
    let mut buf = [0u8; 2];
    reader.read_exact(&mut buf)?;
    Ok(u16::from_le_bytes(buf))
}

fn read_i16<R: Read>(reader: &mut R) -> Result<i16> {
    let mut buf = [0u8; 2];
    reader.read_exact(&mut buf)?;
    Ok(i16::from_le_bytes(buf))
}

fn read_u32<R: Read>(reader: &mut R) -> Result<u32> {
    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf)?;
    Ok(u32::from_le_bytes(buf))
}

fn read_i32<R: Read>(reader: &mut R) -> Result<i32> {
    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf)?;
    Ok(i32::from_le_bytes(buf))
}

fn read_u64<R: Read>(reader: &mut R) -> Result<u64> {
    let mut buf = [0u8; 8];
    reader.read_exact(&mut buf)?;
    Ok(u64::from_le_bytes(buf))
}

fn read_i64<R: Read>(reader: &mut R) -> Result<i64> {
    let mut buf = [0u8; 8];
    reader.read_exact(&mut buf)?;
    Ok(i64::from_le_bytes(buf))
}

fn read_f32<R: Read>(reader: &mut R) -> Result<f32> {
    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf)?;
    Ok(f32::from_le_bytes(buf))
}

fn read_f64<R: Read>(reader: &mut R) -> Result<f64> {
    let mut buf = [0u8; 8];
    reader.read_exact(&mut buf)?;
    Ok(f64::from_le_bytes(buf))
}

fn read_bool<R: Read>(reader: &mut R) -> Result<bool> {
    let val = read_u8(reader)?;
    Ok(val != 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_to_u64() {
        assert_eq!(value_to_u64(&MetadataValue::U32(8192)), Some(8192));
        assert_eq!(value_to_u64(&MetadataValue::U64(131072)), Some(131072));
        assert_eq!(value_to_u64(&MetadataValue::I32(32768)), Some(32768));
        assert_eq!(value_to_u64(&MetadataValue::I32(-1)), None);
    }
}
