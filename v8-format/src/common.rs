use std::collections::{HashMap, HashSet};

pub enum ArrayBufferViewType {
    Int8Array,
    Uint8Array,
    Uint8ClampedArray,
    Int16Array,
    Uint16Array,
    Int32Array,
    Uint32Array,
    Float32Array,
    Float64Array,
    BigInt64Array,
    BigUint64Array,
    DataView,
}

pub enum ErrorType {
    EvalError,
    RangeError,
    ReferenceError,
    SyntaxError,
    TypeError,
    UriError,
    Unknown,
}

pub enum Value {
    Undefined,
    Null,
    Boolean(bool),
    Int32(i32),
    Uint32(u32),
    Double(f64),
    BigInt(i64),
    String(String, bool),
    ObjectReference {
        id: u32,
    },
    Object(HashMap<String, Value>),
    Array(Vec<Value>),
    Date(f64),
    NumberObject(f64),
    BigIntObject(),
    StringObject(String),
    RegExp {
        expr: String,
        flags: u32,
    },
    Map(HashMap<Value, Value>),
    Set(HashSet<Value>),
    ArrayBuffer(Vec<u8>),
    ArrayBufferTransfer {
        transfer_id: u32,
    },
    ArrayBufferView {
        ty: ArrayBufferViewType,
        byte_offset: u32,
        byte_length: u32,
        buffer: Vec<u8>,
    },
    SharedArrayBuffer {
        transfer_id: u32,
    },
    Error {
        ty: ErrorType,
        message: Option<String>,
        stack: Option<String>,
    },
}

impl Value {
    pub fn is_undefined(&self) -> bool {
        match self {
            &Value::Undefined => true,
            _ => false,
        }
    }

    pub fn is_null(&self) -> bool {
        match self {
            &Value::Null => true,
            _ => false,
        }
    }

    pub fn is_boolean(&self) -> bool {
        match self {
            &Value::Boolean(_) => true,
            _ => false,
        }
    }

    pub fn is_int32(&self) -> bool {
        match self {
            &Value::Int32(_) => true,
            _ => false,
        }
    }

    pub fn is_uint32(&self) -> bool {
        match self {
            &Value::Uint32(_) => true,
            _ => false,
        }
    }

    pub fn is_double(&self) -> bool {
        match self {
            &Value::Double(_) => true,
            _ => false,
        }
    }

    pub fn is_bigint(&self) -> bool {
        match self {
            &Value::BigInt(_) => true,
            _ => false,
        }
    }

    pub fn is_string(&self) -> bool {
        match self {
            &Value::String(_, _) => true,
            _ => false,
        }
    }

    pub fn is_object_reference(&self) -> bool {
        match self {
            &Value::ObjectReference { .. } => true,
            _ => false,
        }
    }

    pub fn is_object(&self) -> bool {
        match self {
            &Value::Object(_) => true,
            _ => false,
        }
    }

    pub fn is_array(&self) -> bool {
        match self {
            &Value::Array(_) => true,
            _ => false,
        }
    }

    pub fn is_date(&self) -> bool {
        match self {
            &Value::Date(_) => true,
            _ => false,
        }
    }

    pub fn is_number_object(&self) -> bool {
        match self {
            &Value::NumberObject(_) => true,
            _ => false,
        }
    }

    pub fn is_bigint_object(&self) -> bool {
        match self {
            &Value::BigIntObject() => true,
            _ => false,
        }
    }

    pub fn is_string_object(&self) -> bool {
        match self {
            &Value::StringObject(_) => true,
            _ => false,
        }
    }

    pub fn is_regexp(&self) -> bool {
        match self {
            &Value::RegExp { .. } => true,
            _ => false,
        }
    }

    pub fn is_map(&self) -> bool {
        match self {
            &Value::Map(_) => true,
            _ => false,
        }
    }

    pub fn is_set(&self) -> bool {
        match self {
            &Value::Set(_) => true,
            _ => false,
        }
    }

    pub fn is_array_buffer(&self) -> bool {
        match self {
            &Value::ArrayBuffer(_) => true,
            _ => false,
        }
    }

    pub fn is_array_buffer_transfer(&self) -> bool {
        match self {
            &Value::ArrayBufferTransfer { .. } => true,
            _ => false,
        }
    }

    pub fn is_array_buffer_view(&self) -> bool {
        match self {
            &Value::ArrayBufferView { .. } => true,
            _ => false,
        }
    }

    pub fn is_shared_array_buffer(&self) -> bool {
        match self {
            &Value::SharedArrayBuffer { .. } => true,
            _ => false,
        }
    }

    pub fn is_error(&self) -> bool {
        match self {
            &Value::Error { .. } => true,
            _ => false,
        }
    }
}
