use crate::engine::core::Command;
use std::io::{BufRead, BufReader, Read, Write};

#[derive(Debug)]
pub enum LogError {
    Io(std::io::Error),
    Json(serde_json::Error),
}

impl std::fmt::Display for LogError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            LogError::Io(e) => write!(f, "log I/O error: {}", e),
            LogError::Json(e) => write!(f, "log JSON parse error: {}", e),
        }
    }
}

impl std::error::Error for LogError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            LogError::Io(e) => Some(e),
            LogError::Json(e) => Some(e),
        }
    }
}

impl From<std::io::Error> for LogError {
    fn from(e: std::io::Error) -> Self {
        LogError::Io(e)
    }
}

impl From<serde_json::Error> for LogError {
    fn from(e: serde_json::Error) -> Self {
        LogError::Json(e)
    }
}

pub fn append_command<W: Write>(writer: &mut W, command: &Command) -> Result<(), LogError> {
    let line = serde_json::to_string(command)?;
    writeln!(writer, "{}", line)?;
    Ok(())
}

pub fn read_commands<R: Read>(reader: R) -> Result<Vec<Command>, LogError> {
    let reader = BufReader::new(reader);
    reader
        .lines()
        .map(|line| -> Result<Command, LogError> {
            let line = line?;
            Ok(serde_json::from_str(&line)?)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::engine::core::Command::{CancelOrder, SubmitOrder};
    use crate::engine::core::OrderType::Limit;
    use crate::engine::core::Timestamp;
    use crate::engine::log::{append_command, read_commands};
    use crate::engine::order::{OrderId, Price, Qty, Side};

    #[test]
    fn commands_format_is_consistent_after_encoding_and_decoding_back() {
        let commands = vec![
            SubmitOrder {
                timestamp: Timestamp(1),
                quantity: Qty(3),
                side: Side::Buy,
                order_type: Limit(Price(100)),
            },
            CancelOrder {
                order_id: OrderId(1),
                timestamp: Timestamp(2),
            },
        ];

        let mut buffer: Vec<u8> = Vec::new();
        for command in &commands {
            append_command(&mut buffer, &command).unwrap();
        }

        let decoded = read_commands(&buffer[..]).unwrap();

        assert_eq!(commands, decoded);
    }
}
