//! HTTP byte range parsing.

/// Parsed HTTP byte-range spec before it is applied to a representation length.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ByteRangeSpec {
    /// `first-last` or `first-`.
    From {
        /// First byte position.
        first: u64,
        /// Optional last byte position.
        last: Option<u64>,
    },
    /// `-suffix_length`.
    Suffix {
        /// Number of bytes requested from the end.
        len: u64,
    },
}

/// A satisfiable inclusive byte range.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SatisfiableRange {
    /// First byte position.
    pub start: u64,
    /// Last byte position.
    pub end: u64,
}

impl SatisfiableRange {
    /// Returns the number of bytes covered by this range.
    pub fn len(self) -> u64 {
        self.end - self.start + 1
    }

    /// Returns true when the range is empty.
    pub fn is_empty(self) -> bool {
        false
    }
}

/// Range parsing/application errors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RangeError {
    /// The header is not a single `bytes=` range.
    Invalid,
    /// The range cannot be satisfied for the representation length.
    Unsatisfiable,
}

/// Parses a single HTTP `Range` header value.
pub fn parse_range_header(input: &str) -> Result<ByteRangeSpec, RangeError> {
    let value = input.trim();
    let spec = value.strip_prefix("bytes=").ok_or(RangeError::Invalid)?;
    if spec.contains(',') {
        return Err(RangeError::Invalid);
    }
    let (start, end) = spec.split_once('-').ok_or(RangeError::Invalid)?;
    if start.is_empty() {
        let len = parse_u64(end)?;
        if len == 0 {
            return Err(RangeError::Unsatisfiable);
        }
        return Ok(ByteRangeSpec::Suffix { len });
    }
    let first = parse_u64(start)?;
    let last = if end.is_empty() {
        None
    } else {
        Some(parse_u64(end)?)
    };
    if let Some(last) = last {
        if last < first {
            return Err(RangeError::Unsatisfiable);
        }
    }
    Ok(ByteRangeSpec::From { first, last })
}

impl ByteRangeSpec {
    /// Applies this range to a representation length.
    pub fn apply(self, total_len: u64) -> Result<SatisfiableRange, RangeError> {
        if total_len == 0 {
            return Err(RangeError::Unsatisfiable);
        }
        match self {
            Self::From { first, last } => {
                if first >= total_len {
                    return Err(RangeError::Unsatisfiable);
                }
                let end = last.map_or(total_len - 1, |last| last.min(total_len - 1));
                Ok(SatisfiableRange { start: first, end })
            }
            Self::Suffix { len } => {
                let len = len.min(total_len);
                Ok(SatisfiableRange {
                    start: total_len - len,
                    end: total_len - 1,
                })
            }
        }
    }
}

fn parse_u64(input: &str) -> Result<u64, RangeError> {
    if input.is_empty() || !input.bytes().all(|b| b.is_ascii_digit()) {
        return Err(RangeError::Invalid);
    }
    input.parse().map_err(|_| RangeError::Invalid)
}
