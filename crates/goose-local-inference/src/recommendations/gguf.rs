use anyhow::{bail, ensure, Result};

/// Finds the end of the GGUF framing without interpreting model tensors.
/// An incomplete prefix returns the next required byte position.
pub fn header_end(bytes: &[u8]) -> Result<Result<usize, usize>> {
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
    for _ in 0..metadata {
        complete!(cursor.string());
        let kind = number!(4);
        complete!(cursor.value(kind, 0));
    }
    for _ in 0..tensors {
        complete!(cursor.string());
        let dimensions = number!(4);
        ensure!(dimensions <= 4, "Invalid GGUF tensor dimensions");
        complete!(cursor.skip(dimensions as usize * 8 + 4 + 8));
    }
    Ok(Ok(cursor.offset))
}
