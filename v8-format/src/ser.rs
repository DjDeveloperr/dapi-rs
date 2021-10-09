use std::collections::{HashMap, HashSet};

use integer_encoding::VarInt;

use crate::common::{ArrayBufferViewType, ErrorType, Value};

pub struct Serializer {
    data: Vec<u8>,
}

impl Serializer {
    pub fn new() -> Self {
        Self { data: vec![] }
    }

    fn write_undefined(&mut self) {
        self.data.push('_' as u8)
    }

    fn write_null(&mut self) {
        self.data.push('0' as u8)
    }

    fn write_boolean(&mut self, value: bool) {
        match value {
            true => self.data.push('T' as u8),
            false => self.data.push('F' as u8),
        }
    }

    fn write_int32(&mut self, value: i32) {
        self.data.push('I' as u8);
        self.data.extend(value.encode_var_vec());
    }

    fn write_uint32(&mut self, value: u32) {
        self.data.push('U' as u8);
        self.data.extend(value.encode_var_vec());
    }

    fn write_double(&mut self, value: f64) {
        self.data.push('N' as u8);
        self.data.extend(value.to_ne_bytes());
    }

    // todo: support i128 too
    fn write_bigint(&mut self, value: i64) {
        self.data.push('Z' as u8);

        let mut flags = 0u32;
        if value < 0 {
            flags |= 1 << 0; // signed
        }

        flags |= 8 * 2; // bits

        self.data.extend(flags.encode_var_vec());
        self.data.extend((value as u64).to_le_bytes()); // is this right
    }

    fn write_string(&mut self, value: String, utf16: bool) {
        self.data.push(if utf16 { 'c' } else { '"' } as u8);
        self.data.extend(value.len().encode_var_vec());
        self.data.extend(value.as_bytes());
    }

    fn write_object_reference(&mut self, id: u32) {
        self.data.push('^' as u8);
        self.data.extend(id.encode_var_vec());
    }

    fn write_object(&mut self, value: HashMap<String, Value>) {
        self.data.push('o' as u8);
        let size = value.len();
        for (k, v) in value {
            self.write_string(k, false);
            self.write_value(v);
        }
        self.data.push('}' as u8);
        self.data.extend((size as u32).encode_var_vec());
    }

    fn write_array(&mut self, value: Vec<Value>) {
        self.data.push('A' as u8);
        let len = value.len();
        let enc = len.encode_var_vec();
        self.data.extend(&enc);
        for val in value {
            self.write_value(val);
        }
        self.data.push(36);
        self.data.push(0);
        self.data.extend(enc);
    }

    fn write_date(&mut self, value: f64) {
        self.data.push('D' as u8);
        self.data.extend(value.to_ne_bytes()); // ne or le?
    }

    fn write_number_object(&mut self, value: f64) {
        self.data.push('n' as u8);
        self.data.extend(value.to_ne_bytes()); // ne or le?
    }

    fn write_bigint_object(&mut self) {
        self.data.push('z' as u8); // todo
    }

    fn write_string_object(&mut self, value: String) {
        self.data.push('s' as u8);
        self.data.extend((value.len() as u32).encode_var_vec());
        self.data.extend(value.as_bytes());
    }

    fn write_regexp(&mut self, expr: String, flags: u32) {
        self.data.push('R' as u8);
        self.data.extend((expr.len() as u32).encode_var_vec());
        self.data.extend(expr.as_bytes());
        self.data.extend(flags.encode_var_vec());
    }

    fn write_map(&mut self, value: HashMap<Value, Value>) {
        self.data.push(';' as u8);
        let size = value.len();
        for (k, v) in value {
            self.write_value(k);
            self.write_value(v);
        }
        self.data.push(':' as u8);
        self.data.extend((size as u32).encode_var_vec());
    }

    fn write_set(&mut self, value: HashSet<Value>) {
        self.data.push('\'' as u8);
        let size = value.len();
        for v in value {
            self.write_value(v);
        }
        self.data.push(',' as u8);
        self.data.extend((size as u32).encode_var_vec());
    }

    fn write_array_buffer(&mut self, value: Vec<u8>) {
        self.data.push('B' as u8);
        self.data.extend((value.len() as u32).encode_var_vec());
        self.data.extend(value);
    }

    fn write_array_buffer_transfer(&mut self, transfer_id: u32) {
        self.data.push('t' as u8);
        self.data.extend(transfer_id.encode_var_vec());
    }

    fn write_array_buffer_view(
        &mut self,
        ty: ArrayBufferViewType,
        byte_offset: u32,
        byte_length: u32,
        buffer: Vec<u8>,
    ) {
        self.write_array_buffer(buffer);
        self.data.push('V' as u8);
        self.data.push(match ty {
            ArrayBufferViewType::Int8Array => 'b',
            ArrayBufferViewType::Uint8Array => 'B',
            ArrayBufferViewType::Uint8ClampedArray => 'C',
            ArrayBufferViewType::Int16Array => 'w',
            ArrayBufferViewType::Uint16Array => 'W',
            ArrayBufferViewType::Int32Array => 'd',
            ArrayBufferViewType::Uint32Array => 'D',
            ArrayBufferViewType::Float32Array => 'f',
            ArrayBufferViewType::Float64Array => 'F',
            ArrayBufferViewType::BigInt64Array => 'q',
            ArrayBufferViewType::BigUint64Array => 'Q',
            ArrayBufferViewType::DataView => '?',
        } as u8);
        self.data.extend(byte_offset.encode_var_vec());
        self.data.extend(byte_length.encode_var_vec());
    }

    fn write_shared_array_buffer(&mut self, transfer_id: u32) {
        self.data.push('u' as u8);
        self.data.extend(transfer_id.encode_var_vec());
    }

    fn write_error(&mut self, ty: ErrorType, message: Option<String>, stack: Option<String>) {
        self.data.push('r' as u8);
        if let Some(ch) = match ty {
            ErrorType::EvalError => Some('E'),
            ErrorType::RangeError => Some('R'),
            ErrorType::ReferenceError => Some('F'),
            ErrorType::SyntaxError => Some('C'),
            ErrorType::TypeError => Some('T'),
            ErrorType::UriError => Some('U'),
            ErrorType::Unknown => None,
        } {
            self.data.push(ch as u8);
        }
        
        if let Some(message) = message {
            self.data.push('m' as u8);
            self.write_string(message, false);
        }

        if let Some(stack) = stack {
            self.data.push('s' as u8);
            self.write_string(stack, false);
        }

        self.data.push('.' as u8);
    }

    fn write_value(&mut self, value: Value) {
        match value {
            Value::Undefined => self.write_undefined(),
            Value::Null => self.write_null(),
            Value::Boolean(value) => self.write_boolean(value),
            Value::Int32(value) => self.write_int32(value),
            Value::Uint32(value) => self.write_uint32(value),
            Value::Double(value) => self.write_double(value),
            Value::BigInt(value) => self.write_bigint(value),
            Value::String(value, utf16) => self.write_string(value, utf16),
            Value::ObjectReference { id } => self.write_object_reference(id),
            Value::Object(value) => self.write_object(value),
            Value::Array(value) => self.write_array(value),
            Value::Date(value) => self.write_date(value),
            Value::NumberObject(value) => self.write_number_object(value),
            Value::BigIntObject() => self.write_bigint_object(),
            Value::StringObject(value) => self.write_string_object(value),
            Value::RegExp { expr, flags } => self.write_regexp(expr, flags),
            Value::Map(value) => self.write_map(value),
            Value::Set(value) => self.write_set(value),
            Value::ArrayBuffer(value) => self.write_array_buffer(value),
            Value::ArrayBufferTransfer { transfer_id } => {
                self.write_array_buffer_transfer(transfer_id)
            }
            Value::ArrayBufferView {
                ty,
                byte_offset,
                byte_length,
                buffer,
            } => self.write_array_buffer_view(ty, byte_offset, byte_length, buffer),
            Value::SharedArrayBuffer { transfer_id } => self.write_shared_array_buffer(transfer_id),
            Value::Error { ty, message, stack } => self.write_error(ty, message, stack),
        }
    }

    pub fn serialize(mut self, value: Value) -> Vec<u8> {
        self.write_value(value);
        self.data
    }
}
