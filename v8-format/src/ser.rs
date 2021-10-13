#![allow(dead_code)]
#![allow(unused_variables)]

use crate::common::Error;
use serde::ser;
use serde::Serialize;
use std::collections::HashMap;
use std::collections::HashSet;

use integer_encoding::VarInt;

use crate::common::ArrayBufferViewType;
use crate::common::ErrorType;
use crate::common::Value;

pub const FORMAT_VERSION: u8 = 0xD0;

pub fn to_vec<T: Serialize>(value: T) -> Result<Vec<u8>, Error> {
    let mut serializer = Serializer {
        data: vec![0xFF, FORMAT_VERSION],
        start_pos: 0,
        current_len: None,
        lazy_len: 0,
    };
    value.serialize(&mut serializer)?;
    Ok(serializer.data)
}

pub struct Serializer {
    data: Vec<u8>,
    start_pos: usize,
    current_len: Option<usize>,
    lazy_len: usize,
}

// Old impl, non-serde one.
impl Serializer {
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
        self.data.push('{' as u8);
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

    fn serialize(mut self, value: Value) -> Vec<u8> {
        self.write_value(value);
        self.data
    }
}

impl<'a> ser::Serializer for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        match v {
            true => self.data.push('T' as u8),
            false => self.data.push('F' as u8),
        }
        Ok(())
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        self.data.push('I' as u8);
        self.data.extend(v.encode_var_vec());
        Ok(())
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        self.data.push('I' as u8);
        self.data.extend(v.encode_var_vec());
        Ok(())
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        self.data.push('I' as u8);
        self.data.extend(v.encode_var_vec());
        Ok(())
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        self.data.push('Z' as u8);

        let mut flags = 0u32;
        flags |= 1 << 0; // signed
        flags |= 8 * 2; // bits

        self.data.extend(flags.encode_var_vec());
        self.data.extend((v as u64).to_le_bytes());
        Ok(())
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        self.data.push('U' as u8);
        self.data.extend(v.encode_var_vec());
        Ok(())
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        self.data.push('U' as u8);
        self.data.extend(v.encode_var_vec());
        Ok(())
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        self.data.push('U' as u8);
        self.data.extend(v.encode_var_vec());
        Ok(())
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        self.data.push('Z' as u8);

        let mut flags = 0u32;
        flags |= 8 * 2; // bits

        self.data.extend(flags.encode_var_vec());
        self.data.extend(v.to_le_bytes());
        Ok(())
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        self.data.push('N' as u8);
        self.data.extend((v as f64).to_ne_bytes());
        Ok(())
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        self.data.push('N' as u8);
        self.data.extend(v.to_ne_bytes());
        Ok(())
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        self.data.push('"' as u8);
        self.data.push(1);
        self.data.push(v as u8);
        Ok(())
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        self.data.push('"' as u8);
        self.data.extend(v.len().encode_var_vec());
        self.data.extend(v.as_bytes());
        Ok(())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        self.data.push('B' as u8);
        if v.len() > u32::MAX as usize {
            return Err(Error::Message(String::from(
                "Bytes cannot be larger than u32::MAX",
            )));
        }
        self.data.extend((v.len() as u32).encode_var_vec());
        self.data.extend(v);
        Ok(())
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        self.data.push('_' as u8);
        Ok(())
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        self.serialize_none()
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        // empty object
        self.data.push('o' as u8);
        self.data.push('{' as u8);
        self.data.push(0);
        Ok(())
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.data.push('o' as u8);
        self.serialize_str(variant)?;
        self.serialize_none()?;
        self.data.push('{' as u8);
        self.data.push(1);
        Ok(())
    }

    fn serialize_newtype_struct<T: ?Sized>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        self.data.push('o' as u8);
        self.serialize_str(variant)?;
        value.serialize(&mut *self)?;
        self.data.push('{' as u8);
        self.data.push(1);
        Ok(())
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        self.data.push('A' as u8);
        self.current_len = len;
        self.lazy_len = 0;
        if let Some(len) = len {
            self.data.extend((len as u32).encode_var_vec());
        } else {
            self.start_pos = self.data.len();
        }
        Ok(self)
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.data.push('A' as u8);
        self.data.extend((len as u32).encode_var_vec());
        self.current_len = Some(len);
        Ok(self)
    }

    fn serialize_tuple_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.data.push('A' as u8);
        self.data.extend((len as u32).encode_var_vec());
        self.current_len = Some(len);
        Ok(self)
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        self.data.push('o' as u8);
        self.serialize_str(variant)?;
        self.data.push('A' as u8);
        self.data.extend((len as u32).encode_var_vec());
        self.current_len = Some(len);
        Ok(self)
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        self.data.push(';' as u8);
        self.current_len = len;
        self.lazy_len = 0;
        Ok(self)
    }

    fn serialize_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        self.data.push('o' as u8);
        self.current_len = Some(len);
        self.lazy_len = 0;
        Ok(self)
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        self.data.push('o' as u8);
        self.serialize_str(variant)?;
        self.data.push('o' as u8);
        self.current_len = Some(len);
        self.lazy_len = 0;
        Ok(self)
    }
}

impl<'a> ser::SerializeSeq for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: ?Sized + Serialize,
    {
        if self.current_len.is_none() {
            self.lazy_len += 1;
        }
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<(), Error> {
        // It was a lazy one, so insert len at start_pos
        if self.current_len.is_none() {
            let mut pos = self.start_pos;
            for byte in (self.lazy_len as u32).encode_var_vec() {
                self.data.insert(pos, byte);
                pos += 1;
            }
        }
        self.data.push('$' as u8);
        self.data.push(0);
        self.data
            .extend((self.current_len.unwrap_or(self.lazy_len) as u32).encode_var_vec());
        Ok(())
    }
}

impl<'a> ser::SerializeTuple for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<(), Error> {
        self.data.push('$' as u8);
        self.data.push(0);
        self.data
            .extend((self.current_len.unwrap() as u32).encode_var_vec());
        Ok(())
    }
}

impl<'a> ser::SerializeTupleStruct for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<(), Error> {
        self.data.push('$' as u8);
        self.data.push(0);
        self.data
            .extend((self.current_len.unwrap() as u32).encode_var_vec());
        Ok(())
    }
}

impl<'a> ser::SerializeTupleVariant for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<(), Error> {
        // End the tuple
        self.data.push('$' as u8);
        self.data.push(0);
        self.data
            .extend((self.current_len.unwrap() as u32).encode_var_vec());
        // End the enum variant object
        self.data.push('{' as u8);
        self.data.push(1);
        Ok(())
    }
}

impl<'a> ser::SerializeMap for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<(), Error>
    where
        T: ?Sized + Serialize,
    {
        key.serialize(&mut **self)
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: ?Sized + Serialize,
    {
        if self.current_len.is_none() {
            self.lazy_len += 1;
        }
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<(), Error> {
        self.data.push(':' as u8);
        // Actually * 2 length is used here because its two values per entry.
        self.data
            .extend(((self.current_len.unwrap_or(self.lazy_len) * 2) as u32).encode_var_vec());
        Ok(())
    }
}

impl<'a> ser::SerializeStruct for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Error>
    where
        T: ?Sized + Serialize,
    {
        use serde::Serializer;
        self.serialize_str(key)?;
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<(), Error> {
        self.data.push('{' as u8);
        self.data
            .extend((self.current_len.unwrap() as u32).encode_var_vec());
        Ok(())
    }
}

impl<'a> ser::SerializeStructVariant for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Error>
    where
        T: ?Sized + Serialize,
    {
        use serde::Serializer;
        self.serialize_str(key)?;
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<(), Error> {
        // End inner object (variant's value)
        self.data.push('{' as u8);
        self.data
            .extend((self.current_len.unwrap() as u32).encode_var_vec());
        // End outer object (variant)
        self.data.push('{' as u8);
        self.data.push(1);
        Ok(())
    }
}

pub trait SerializeDateExt {
    type Ok;
    type Error: std::error::Error;

    fn serialize_date<T: ?Sized>(&mut self, date: std::time::Instant) -> Result<(), Self::Error>
    where
        T: Serialize;
}

impl<'a> SerializeDateExt for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_date<T: ?Sized>(&mut self, date: std::time::Instant) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.data.push('D' as u8);
        self.data
            .extend((date.elapsed().as_millis() as f64).to_ne_bytes());
        Ok(())
    }
}

#[test]
fn test_boolean() {
    assert_eq!(to_vec(true).unwrap(), vec![0xFF, FORMAT_VERSION, 84]);
    assert_eq!(to_vec(false).unwrap(), vec![0xFF, FORMAT_VERSION, 70]);
}

#[test]
fn test_numbers() {
    // Upto 32 bit numbers, encoding is same.
    assert_eq!(to_vec(1u32).unwrap(), vec![0xFF, FORMAT_VERSION, 85, 1]);
    assert_eq!(to_vec(1i32).unwrap(), vec![0xFF, FORMAT_VERSION, 73, 2]);

    // For 64 bit (and above), it's different
    assert_eq!(
        to_vec(1u64).unwrap(),
        vec![0xFF, FORMAT_VERSION, 90, 16, 1, 0, 0, 0, 0, 0, 0, 0]
    );
    assert_eq!(
        to_vec(1i64).unwrap(),
        vec![0xFF, FORMAT_VERSION, 90, 17, 1, 0, 0, 0, 0, 0, 0, 0]
    );

    // For floats too it is different encoding, but it's at least same for both f32 and f64.
    // But there's a quirk with this format: it uses native endianness
    // making this format non-portable. So we need a cfg directive here.
    #[cfg(target_endian = "little")]
    let expected = vec![0xFF, FORMAT_VERSION, 78, 31, 133, 235, 81, 184, 30, 9, 64];
    #[cfg(target_endian = "big")]
    let expected = vec![0xFF, FORMAT_VERSION, 78, 64, 9, 30, 184, 81, 235, 133, 31];
    assert_eq!(to_vec(3.14f64).unwrap(), expected);
}

#[test]
fn test_char() {
    assert_eq!(to_vec('h').unwrap(), vec![0xFF, FORMAT_VERSION, 34, 1, 104]);
}

#[test]
fn test_str() {
    assert_eq!(
        to_vec("test").unwrap(),
        vec![0xFF, FORMAT_VERSION, 34, 4, 116, 101, 115, 116]
    );
}

#[test]
fn test_bytes() {
    use serde_bytes::Bytes;

    let bytes: [u8; 3] = [1, 2, 3];
    assert_eq!(
        to_vec(Bytes::new(&bytes)).unwrap(),
        vec![0xFF, FORMAT_VERSION, 66, 3, 1, 2, 3]
    );
}

#[test]
fn test_undefined() {
    assert_eq!(
        to_vec(None as Option<()>).unwrap(),
        vec![0xFF, FORMAT_VERSION, 95]
    );
    assert_eq!(to_vec(()).unwrap(), vec![0xFF, FORMAT_VERSION, 95]);
}

#[test]
fn test_some() {
    assert_eq!(to_vec(Some(true)).unwrap(), vec![0xFF, FORMAT_VERSION, 84]);
}

#[test]
fn test_unit_struct() {
    #[derive(Serialize)]
    struct Unit;
    let unit = Unit;
    assert_eq!(
        to_vec(unit).unwrap(),
        vec![0xFF, FORMAT_VERSION, 111, 123, 0]
    );
}

#[test]
fn test_unit_variant() {
    #[derive(Serialize)]
    enum Enum {
        Variant,
    }

    assert_eq!(
        to_vec(Enum::Variant).unwrap(),
        vec![
            0xFF,
            FORMAT_VERSION,
            111,
            34,
            7,
            86,
            97,
            114,
            105,
            97,
            110,
            116,
            95,
            123,
            1
        ]
    )
}

#[test]
fn test_newtype_struct() {
    #[derive(Serialize)]
    struct Newtype(bool);

    assert_eq!(
        to_vec(Newtype(true)).unwrap(),
        vec![0xFF, FORMAT_VERSION, 84]
    );
}

#[test]
fn test_newtype_variant() {
    #[derive(Serialize)]
    enum Enum {
        Variant(bool),
    }

    assert_eq!(
        to_vec(Enum::Variant(true)).unwrap(),
        vec![
            0xFF,
            FORMAT_VERSION,
            111,
            34,
            7,
            86,
            97,
            114,
            105,
            97,
            110,
            116,
            84,
            123,
            1
        ]
    )
}

#[test]
fn test_seq() {
    assert_eq!(
        to_vec([true, false]).unwrap(),
        vec![0xFF, FORMAT_VERSION, 65, 2, 84, 70, 36, 0, 2]
    );
}

#[test]
fn test_tuple() {
    assert_eq!(
        to_vec((false, true)).unwrap(),
        vec![0xFF, FORMAT_VERSION, 65, 2, 70, 84, 36, 0, 2]
    );
}

#[test]
fn test_tuple_struct() {
    #[derive(Serialize)]
    struct Tuple(bool, bool);

    assert_eq!(
        to_vec(Tuple(false, true)).unwrap(),
        vec![0xFF, FORMAT_VERSION, 65, 2, 70, 84, 36, 0, 2]
    );
}

#[test]
fn test_tuple_variant() {
    #[derive(Serialize)]
    enum Enum {
        Variant(bool, bool),
    }

    assert_eq!(
        to_vec(Enum::Variant(true, false)).unwrap(),
        vec![
            0xFF,
            FORMAT_VERSION,
            111,
            34,
            7,
            86,
            97,
            114,
            105,
            97,
            110,
            116,
            65,
            2,
            84,
            70,
            36,
            0,
            2,
            123,
            1
        ],
    );
}

#[test]
fn test_map() {
    let mut map = std::collections::HashMap::new();
    map.insert("Hello", "World");

    assert_eq!(
        to_vec(map).unwrap(),
        vec![
            0xFF,
            FORMAT_VERSION,
            59,
            34,
            5,
            72,
            101,
            108,
            108,
            111,
            34,
            5,
            87,
            111,
            114,
            108,
            100,
            58,
            2
        ],
    )
}

#[test]
fn test_struct() {
    #[derive(Serialize)]
    struct Point {
        x: i32,
        y: i32,
    }

    assert_eq!(
        to_vec(Point { x: 69, y: 70 }).unwrap(),
        vec![
            0xFF,
            FORMAT_VERSION,
            111,
            34,
            1,
            120,
            73,
            138,
            1,
            34,
            1,
            121,
            73,
            140,
            1,
            123,
            2
        ],
    )
}

#[test]
fn test_struct_variant() {
    #[derive(Serialize)]
    enum Enum {
        Point { x: i32, y: i32 },
    }

    assert_eq!(
        to_vec(Enum::Point { x: 69, y: 70 }).unwrap(),
        vec![
            0xFF,
            FORMAT_VERSION,
            111,
            34,
            5,
            80,
            111,
            105,
            110,
            116,
            111,
            34,
            1,
            120,
            73,
            138,
            1,
            34,
            1,
            121,
            73,
            140,
            1,
            123,
            2,
            123,
            1
        ]
    );
}

#[test]
fn test_date_ext() {
    // todo    
}
