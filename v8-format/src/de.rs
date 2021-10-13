#![allow(dead_code)]

use crate::common::Value;
use crate::common::Error;
use crate::common::Result;

pub struct Deserializer<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> Deserializer<'a> {
    pub fn new() -> Self {
        Self {
            data: &[],
            offset: 0,
        }
    }

    #[inline(always)]
    fn byte(&self) -> u8 {
        self.data[self.offset]
    }

    fn expect_next(&mut self, to_be: u8) -> Result<()> {
        if self.byte() == to_be {
            self.next();
            Ok(())
        } else {
            Err(Error::Expected {
                to_be,
                but_got: self.byte(),
            })
        }
    }

    #[inline(always)]
    fn next(&mut self) -> usize {
        self.offset += 1;
        self.offset
    }

    fn is_version(&self) -> bool {
        self.byte() == 0xFF
    }

    fn is_undefined(&self) -> bool {
        self.byte() == '_' as u8
    }

    fn parse_undefined(&mut self) -> Result<Value> {
        self.expect_next('_' as u8)?;
        Ok(Value::Undefined)
    }

    fn is_null(&self) -> bool {
        self.byte() == '0' as u8
    }

    fn parse_null(&mut self) -> Result<Value> {
        self.expect_next('0' as u8)?;
        Ok(Value::Undefined)
    }

    fn is_bool(&self) -> bool {
        let byte = self.byte();
        byte == 'T' as u8 || byte == 'F' as u8
    }

    fn parse_bool(&mut self) -> Result<Value> {
        let byte = self.byte();

        let res = match byte as char {
            'T' => Ok(Value::Boolean(true)),
            'F' => Ok(Value::Boolean(false)),
            _ => Err(Error::Unexpected {
                byte,
                at: self.offset,
            }),
        };

        if res.is_ok() {
            self.next();
        }

        res
    }

    fn is_int32(&self) -> bool {
        self.byte() == 'I' as u8
    }

    fn is_uint32(&self) -> bool {
        self.byte() == 'U' as u8
    }

    fn is_double(&self) -> bool {
        self.byte() == 'N' as u8
    }

    fn is_bigint(&self) -> bool {
        self.byte() == 'Z' as u8
    }

    fn is_utf8_string(&self) -> bool {
        self.byte() == 'S' as u8
    }

    fn is_one_byte_string(&self) -> bool {
        self.byte() == '"' as u8
    }

    fn is_two_byte_string(&self) -> bool {
        self.byte() == 'c' as u8
    }

    fn is_object_reference(&self) -> bool {
        self.byte() == '^' as u8
    }

    fn is_object(&self) -> bool {
        self.byte() == 'o' as u8
    }

    fn parse(&mut self) -> Result<Value> {
        if self.is_undefined() {
            self.parse_undefined()
        } else if self.is_null() {
            self.parse_null()
        } else if self.is_bool() {
            self.parse_bool()
        } else {
            Err(Error::Unexpected {
                byte: self.byte(),
                at: self.offset,
            })
        }
    }

    pub fn deserialize(mut self, data: &'a [u8]) -> Result<Value> {
        self.data = data;
        self.offset = 0;

        // Version
        if self.is_version() {
            self.offset += 2;
        }

        // Parse Value
        self.parse()
    }
}
