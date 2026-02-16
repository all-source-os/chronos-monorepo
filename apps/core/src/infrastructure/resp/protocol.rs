//! RESP3 protocol parser and serializer.
//!
//! Implements a subset of the Redis RESP3 wire protocol sufficient for:
//! - Parsing client commands (arrays of bulk strings)
//! - Serializing responses (simple strings, errors, integers, bulk strings,
//!   arrays, maps, and null)
//!
//! Reference: <https://github.com/redis/redis-specifications/blob/master/protocol/RESP3.md>

use std::collections::BTreeMap;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};

/// A value in the RESP3 protocol.
#[derive(Debug, Clone, PartialEq)]
pub enum RespValue {
    /// `+OK\r\n`
    SimpleString(String),
    /// `-ERR message\r\n`
    Error(String),
    /// `:<number>\r\n`
    Integer(i64),
    /// `$<len>\r\n<data>\r\n` or `$-1\r\n` for null
    BulkString(Vec<u8>),
    /// `*<count>\r\n...`
    Array(Vec<RespValue>),
    /// `%<count>\r\n...` (RESP3 map type)
    Map(BTreeMap<String, RespValue>),
    /// `_\r\n` (RESP3 null)
    Null,
}

impl RespValue {
    /// Interpret this value as a UTF-8 string (for command parsing).
    pub fn as_str(&self) -> Option<&str> {
        match self {
            RespValue::BulkString(b) => std::str::from_utf8(b).ok(),
            RespValue::SimpleString(s) => Some(s.as_str()),
            _ => None,
        }
    }

    /// Encode this value into RESP3 wire format.
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        self.encode_into(&mut buf);
        buf
    }

    fn encode_into(&self, buf: &mut Vec<u8>) {
        match self {
            RespValue::SimpleString(s) => {
                buf.push(b'+');
                buf.extend_from_slice(s.as_bytes());
                buf.extend_from_slice(b"\r\n");
            }
            RespValue::Error(s) => {
                buf.push(b'-');
                buf.extend_from_slice(s.as_bytes());
                buf.extend_from_slice(b"\r\n");
            }
            RespValue::Integer(n) => {
                buf.push(b':');
                buf.extend_from_slice(n.to_string().as_bytes());
                buf.extend_from_slice(b"\r\n");
            }
            RespValue::BulkString(data) => {
                buf.push(b'$');
                buf.extend_from_slice(data.len().to_string().as_bytes());
                buf.extend_from_slice(b"\r\n");
                buf.extend_from_slice(data);
                buf.extend_from_slice(b"\r\n");
            }
            RespValue::Array(items) => {
                buf.push(b'*');
                buf.extend_from_slice(items.len().to_string().as_bytes());
                buf.extend_from_slice(b"\r\n");
                for item in items {
                    item.encode_into(buf);
                }
            }
            RespValue::Map(map) => {
                buf.push(b'%');
                buf.extend_from_slice(map.len().to_string().as_bytes());
                buf.extend_from_slice(b"\r\n");
                for (k, v) in map {
                    RespValue::BulkString(k.as_bytes().to_vec()).encode_into(buf);
                    v.encode_into(buf);
                }
            }
            RespValue::Null => {
                buf.extend_from_slice(b"_\r\n");
            }
        }
    }
}

/// Parse a single RESP value from an async buffered reader.
///
/// Returns `None` if the connection was closed (EOF).
pub async fn parse_value<R: AsyncBufRead + Unpin>(
    reader: &mut R,
) -> std::io::Result<Option<RespValue>> {
    let mut line = String::new();
    let n = reader.read_line(&mut line).await?;
    if n == 0 {
        return Ok(None); // EOF
    }
    let line = line.trim_end_matches("\r\n").trim_end_matches('\n');

    if line.is_empty() {
        return Ok(None);
    }

    let type_byte = line.as_bytes()[0];
    let rest = &line[1..];

    match type_byte {
        b'+' => Ok(Some(RespValue::SimpleString(rest.to_string()))),
        b'-' => Ok(Some(RespValue::Error(rest.to_string()))),
        b':' => {
            let n: i64 = rest.parse().map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid integer")
            })?;
            Ok(Some(RespValue::Integer(n)))
        }
        b'$' => {
            let len: i64 = rest.parse().map_err(|_| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "invalid bulk string length",
                )
            })?;
            if len < 0 {
                return Ok(Some(RespValue::Null));
            }
            let len = len as usize;
            let mut data = vec![0u8; len + 2]; // +2 for trailing \r\n
            reader.read_exact(&mut data).await?;
            data.truncate(len); // remove \r\n
            Ok(Some(RespValue::BulkString(data)))
        }
        b'*' => {
            let count: i64 = rest.parse().map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid array length")
            })?;
            if count < 0 {
                return Ok(Some(RespValue::Null));
            }
            let mut items = Vec::with_capacity(count as usize);
            for _ in 0..count {
                match Box::pin(parse_value(reader)).await? {
                    Some(v) => items.push(v),
                    None => {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::UnexpectedEof,
                            "unexpected EOF in array",
                        ));
                    }
                }
            }
            Ok(Some(RespValue::Array(items)))
        }
        b'_' => Ok(Some(RespValue::Null)),
        _ => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("unknown RESP type byte: {}", type_byte as char),
        )),
    }
}

/// Write a RESP value to an async writer.
pub async fn write_value<W: tokio::io::AsyncWrite + Unpin>(
    writer: &mut W,
    value: &RespValue,
) -> std::io::Result<()> {
    writer.write_all(&value.encode()).await?;
    writer.flush().await
}

// ── Helper constructors ─────────────────────────────────────────────────────

impl RespValue {
    pub fn ok() -> Self {
        RespValue::SimpleString("OK".to_string())
    }

    pub fn err(msg: impl Into<String>) -> Self {
        RespValue::Error(format!("ERR {}", msg.into()))
    }

    pub fn bulk(s: impl Into<Vec<u8>>) -> Self {
        RespValue::BulkString(s.into())
    }

    pub fn bulk_string(s: &str) -> Self {
        RespValue::BulkString(s.as_bytes().to_vec())
    }

    pub fn integer(n: i64) -> Self {
        RespValue::Integer(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::BufReader;

    #[test]
    fn test_encode_simple_string() {
        let v = RespValue::SimpleString("OK".to_string());
        assert_eq!(v.encode(), b"+OK\r\n");
    }

    #[test]
    fn test_encode_error() {
        let v = RespValue::Error("ERR bad command".to_string());
        assert_eq!(v.encode(), b"-ERR bad command\r\n");
    }

    #[test]
    fn test_encode_integer() {
        let v = RespValue::Integer(42);
        assert_eq!(v.encode(), b":42\r\n");
    }

    #[test]
    fn test_encode_bulk_string() {
        let v = RespValue::BulkString(b"hello".to_vec());
        assert_eq!(v.encode(), b"$5\r\nhello\r\n");
    }

    #[test]
    fn test_encode_null() {
        let v = RespValue::Null;
        assert_eq!(v.encode(), b"_\r\n");
    }

    #[test]
    fn test_encode_array() {
        let v = RespValue::Array(vec![
            RespValue::BulkString(b"SET".to_vec()),
            RespValue::BulkString(b"key".to_vec()),
        ]);
        assert_eq!(v.encode(), b"*2\r\n$3\r\nSET\r\n$3\r\nkey\r\n");
    }

    #[test]
    fn test_encode_map() {
        let mut map = BTreeMap::new();
        map.insert("key".to_string(), RespValue::BulkString(b"value".to_vec()));
        let v = RespValue::Map(map);
        let encoded = v.encode();
        assert_eq!(encoded, b"%1\r\n$3\r\nkey\r\n$5\r\nvalue\r\n");
    }

    #[tokio::test]
    async fn test_parse_simple_string() {
        let data = b"+OK\r\n";
        let mut reader = BufReader::new(&data[..]);
        let v = parse_value(&mut reader).await.unwrap().unwrap();
        assert_eq!(v, RespValue::SimpleString("OK".to_string()));
    }

    #[tokio::test]
    async fn test_parse_bulk_string() {
        let data = b"$5\r\nhello\r\n";
        let mut reader = BufReader::new(&data[..]);
        let v = parse_value(&mut reader).await.unwrap().unwrap();
        assert_eq!(v, RespValue::BulkString(b"hello".to_vec()));
    }

    #[tokio::test]
    async fn test_parse_array() {
        let data = b"*2\r\n$4\r\nPING\r\n$5\r\nhello\r\n";
        let mut reader = BufReader::new(&data[..]);
        let v = parse_value(&mut reader).await.unwrap().unwrap();
        assert_eq!(
            v,
            RespValue::Array(vec![
                RespValue::BulkString(b"PING".to_vec()),
                RespValue::BulkString(b"hello".to_vec()),
            ])
        );
    }

    #[tokio::test]
    async fn test_parse_integer() {
        let data = b":1000\r\n";
        let mut reader = BufReader::new(&data[..]);
        let v = parse_value(&mut reader).await.unwrap().unwrap();
        assert_eq!(v, RespValue::Integer(1000));
    }

    #[tokio::test]
    async fn test_parse_null_bulk_string() {
        let data = b"$-1\r\n";
        let mut reader = BufReader::new(&data[..]);
        let v = parse_value(&mut reader).await.unwrap().unwrap();
        assert_eq!(v, RespValue::Null);
    }

    #[tokio::test]
    async fn test_parse_eof() {
        let data = b"";
        let mut reader = BufReader::new(&data[..]);
        let v = parse_value(&mut reader).await.unwrap();
        assert!(v.is_none());
    }

    #[tokio::test]
    async fn test_roundtrip_array() {
        let original = RespValue::Array(vec![
            RespValue::BulkString(b"XADD".to_vec()),
            RespValue::BulkString(b"stream".to_vec()),
            RespValue::BulkString(b"*".to_vec()),
            RespValue::BulkString(b"field".to_vec()),
            RespValue::BulkString(b"value".to_vec()),
        ]);
        let encoded = original.encode();
        let mut reader = BufReader::new(&encoded[..]);
        let parsed = parse_value(&mut reader).await.unwrap().unwrap();
        assert_eq!(parsed, original);
    }

    #[test]
    fn test_as_str() {
        assert_eq!(RespValue::bulk_string("hello").as_str(), Some("hello"));
        assert_eq!(
            RespValue::SimpleString("world".to_string()).as_str(),
            Some("world")
        );
        assert_eq!(RespValue::Integer(42).as_str(), None);
    }
}
