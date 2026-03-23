use super::Parser;
use crate::runtime::{Module, Result, WasmError};

pub struct StreamingParser {
    buffer: Vec<u8>,
    module: Option<Module>,
    parser: Parser,
}

impl StreamingParser {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            module: None,
            parser: Parser::new(),
        }
    }

    pub fn parse_chunk(&mut self, data: &[u8]) -> Result<ParseState> {
        self.buffer.extend_from_slice(data);

        match self.parser.parse(&self.buffer) {
            Ok(module) => {
                self.module = Some(module);
                Ok(ParseState::Complete)
            }
            Err(error) if is_incomplete_input(&error) => {
                self.module = None;
                Ok(ParseState::NeedMoreData)
            }
            Err(error) => {
                self.module = None;
                Err(error)
            }
        }
    }

    pub fn module(&self) -> Option<&Module> {
        self.module.as_ref()
    }

    pub fn into_module(self) -> Option<Module> {
        self.module
    }
}

impl Default for StreamingParser {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseState {
    Complete,
    NeedMoreData,
}

fn is_incomplete_input(error: &WasmError) -> bool {
    matches!(error, WasmError::Load(message) if message.contains("unexpected end"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_parser() {
        let data = vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00];
        let mut parser = StreamingParser::new();
        let state = parser.parse_chunk(&data).unwrap();
        assert_eq!(state, ParseState::Complete);
        assert!(parser.module().is_some());
    }

    #[test]
    fn test_streaming_parser_needs_more_data() {
        let mut parser = StreamingParser::new();
        let state = parser.parse_chunk(&[0x00, 0x61, 0x73]).unwrap();
        assert_eq!(state, ParseState::NeedMoreData);
        assert!(parser.module().is_none());
    }

    #[test]
    fn test_streaming_parser_clears_stale_module_on_incomplete_input() {
        let mut parser = StreamingParser::new();
        let valid = [0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00];
        assert_eq!(parser.parse_chunk(&valid).unwrap(), ParseState::Complete);
        assert!(parser.module().is_some());

        let state = parser.parse_chunk(&[0x00]).unwrap();
        assert_eq!(state, ParseState::NeedMoreData);
        assert!(parser.module().is_none());
        assert!(parser.into_module().is_none());
    }
}
