use anyhow::{bail, ensure, Result};

/// Finds the end of the GGUF framing without interpreting model tensors.
/// An incomplete prefix returns the next required byte position.
pub fn header_end(bytes: &[u8]) -> Result<Result<usize, usize>> {
    Ok(inspect_header(bytes, &mut None, &mut Default::default())?.map(|header| header.end))
}

pub fn header_architecture(bytes: &[u8]) -> Result<Option<String>> {
    inspect_header(bytes, &mut None, &mut Default::default())?
        .map(|header| header.architecture)
        .map_err(|_| anyhow::anyhow!("Incomplete GGUF header"))
}

struct Header {
    end: usize,
    architecture: Option<String>,
}

pub fn prefix_architecture(bytes: &[u8]) -> Result<Option<String>> {
    let mut architecture = None;
    let _ = inspect_header(bytes, &mut architecture, &mut Default::default())?;
    Ok(architecture)
}

/// Scalar metadata and array lengths available in a bounded prefix, without copying token arrays.
pub fn prefix_metadata(bytes: &[u8]) -> Result<serde_json::Map<String, serde_json::Value>> {
    let mut metadata = Default::default();
    let _ = inspect_header(bytes, &mut None, &mut metadata)?;
    Ok(metadata)
}

fn inspect_header(
    bytes: &[u8],
    hint: &mut Option<String>,
    scalars: &mut serde_json::Map<String, serde_json::Value>,
) -> Result<Result<Header, usize>> {
    struct Cursor<'a> {
        bytes: &'a [u8],
        offset: usize,
    }
    impl Cursor<'_> {
        fn skip(&mut self, length: usize) -> Result<bool> {
            self.offset = self
                .offset
                .checked_add(length)
                .ok_or_else(|| anyhow::anyhow!("GGUF header overflow"))?;
            ensure!(
                self.offset <= 16 * 1024 * 1024,
                "GGUF header exceeds inspection limit"
            );
            Ok(self.offset <= self.bytes.len())
        }
        fn number(&mut self, length: usize) -> Result<Option<u64>> {
            let start = self.offset;
            if !self.skip(length)? {
                return Ok(None);
            }
            let mut number = [0; 8];
            number[..length].copy_from_slice(&self.bytes[start..self.offset]);
            Ok(Some(u64::from_le_bytes(number)))
        }
        fn string(&mut self) -> Result<bool> {
            let Some(length) = self.number(8)? else {
                return Ok(false);
            };
            self.skip(usize::try_from(length)?)
        }
        fn value(&mut self, kind: u64, depth: u8) -> Result<bool> {
            ensure!(depth <= 4, "GGUF metadata nesting exceeds inspection limit");
            let width = match kind {
                0 | 1 | 7 => 1,
                2 | 3 => 2,
                4..=6 => 4,
                10..=12 => 8,
                8 => return self.string(),
                9 => {
                    let Some(element) = self.number(4)? else {
                        return Ok(false);
                    };
                    let Some(count) = self.number(8)? else {
                        return Ok(false);
                    };
                    ensure!(count <= 2_000_000, "GGUF array exceeds inspection limit");
                    for _ in 0..count {
                        if !self.value(element, depth + 1)? {
                            return Ok(false);
                        }
                    }
                    return Ok(true);
                }
                _ => bail!("Invalid GGUF metadata type"),
            };
            self.skip(width)
        }
    }
    let mut cursor = Cursor { bytes, offset: 0 };
    macro_rules! number {
        ($n:expr) => {
            match cursor.number($n)? {
                Some(n) => n,
                None => return Ok(Err(cursor.offset)),
            }
        };
    }
    macro_rules! complete {
        ($expr:expr) => {
            if !$expr? {
                return Ok(Err(cursor.offset));
            }
        };
    }
    ensure!(
        number!(4) == u32::from_le_bytes(*b"GGUF") as u64,
        "Invalid GGUF magic"
    );
    ensure!(matches!(number!(4), 2 | 3), "Unsupported GGUF version");
    let tensors = number!(8);
    let metadata = number!(8);
    ensure!(
        tensors <= 1_000_000 && metadata <= 1_000_000,
        "GGUF catalog exceeds inspection limit"
    );
    let mut architecture = None;
    for _ in 0..metadata {
        let key_start = cursor.offset;
        complete!(cursor.string());
        let key = std::str::from_utf8(&bytes[key_start + 8..cursor.offset])?;
        let is_architecture = key == "general.architecture";
        let kind = number!(4);
        let value_start = cursor.offset;
        if kind == 9 && bytes.len() >= value_start + 12 {
            let count = u64::from_le_bytes(bytes[value_start + 4..value_start + 12].try_into()?);
            scalars.insert(format!("{key}.length"), count.into());
        }
        complete!(cursor.value(kind, 0));
        let value = &bytes[value_start..cursor.offset];
        let scalar = match kind {
            0 => Some(serde_json::Value::from(value[0])),
            2 => Some(u16::from_le_bytes(value.try_into()?).into()),
            4 => Some(u32::from_le_bytes(value.try_into()?).into()),
            10 => Some(u64::from_le_bytes(value.try_into()?).into()),
            8 => Some(std::str::from_utf8(&value[8..])?.into()),
            _ => None,
        };
        if let Some(value) = scalar {
            ensure!(
                scalars.insert(key.into(), value).is_none(),
                "Duplicate GGUF metadata key"
            );
        }
        if is_architecture {
            ensure!(
                kind == 8 && architecture.is_none(),
                "Invalid GGUF architecture metadata"
            );
            architecture =
                Some(std::str::from_utf8(&bytes[value_start + 8..cursor.offset])?.to_owned());
            *hint = architecture.clone();
        }
    }
    for _ in 0..tensors {
        complete!(cursor.string());
        let dimensions = number!(4);
        ensure!(dimensions <= 4, "Invalid GGUF tensor dimensions");
        complete!(cursor.skip(dimensions as usize * 8 + 4 + 8));
    }
    Ok(Ok(Header {
        end: cursor.offset,
        architecture,
    }))
}
