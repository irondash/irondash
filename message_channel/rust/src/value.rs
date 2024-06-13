use std::{
    any::TypeId, cmp::Ordering, collections::HashMap, convert::Infallible, fmt::Display,
    hash::Hash, num::TryFromIntError, ops::Deref, sync::Arc,
};

use irondash_dart_ffi::raw;

use crate::FinalizableHandle;

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum Value {
    Null,
    Bool(bool),
    I64(i64),
    F64(f64),
    String(String),
    I8List(Vec<i8>),
    U8List(Vec<u8>),
    I16List(Vec<i16>),
    U16List(Vec<u16>),
    I32List(Vec<i32>),
    U32List(Vec<u32>),
    I64List(Vec<i64>),
    F32List(Vec<f32>),
    F64List(Vec<f64>),
    List(Vec<Value>),
    // Map is stored as a list of tuples. It can be converted from and into HashMap
    // if required. For usual flow (convert struct into value -> send to dart,
    // receive from dart, convert into struct) we don't really need HashMap
    // functionality and we'll save time building HashMap that is not used.
    Map(ValueTupleList),

    /// Special Dart objects. These can only be sent from Rust to Dart
    Dart(DartObject),

    /// Can only be send from Rust to Dart. On Dart side this will be a
    /// `FinalizableHandle` instance. When handle gets garbage collected, the
    /// finalizer closure that [`FinalizableHandle`] was created with will be invoked.
    ///
    /// You can send single `FinalizableHandle` instance to Dart more than once
    /// and it will always result in the same Dart object.
    ///
    /// If the [`FinalizableHandle`] has already finalized it will be received as `null`.
    FinalizableHandle(Arc<FinalizableHandle>),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Hash)]
pub enum DartObject {
    SendPort(raw::DartCObjectSendPort),
    Capability(raw::DartCObjectCapability),
}

/// Wrapper for Value tuple that ensures that the underyling list is sorted
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Hash)]
pub struct ValueTupleList(Vec<(Value, Value)>);

impl Default for Value {
    fn default() -> Self {
        Value::Null
    }
}

macro_rules! impl_from {
    ($variant:path, $for_type:ty) => {
        impl From<$for_type> for Value {
            fn from(v: $for_type) -> Value {
                $variant(v.into())
            }
        }
    };
}

impl_from!(Value::Bool, bool);
impl_from!(Value::I64, i8);
impl_from!(Value::I64, u8);
impl_from!(Value::I64, i16);
impl_from!(Value::I64, u16);
impl_from!(Value::I64, i32);
impl_from!(Value::I64, u32);
impl_from!(Value::I64, i64);
impl_from!(Value::F64, f32);
impl_from!(Value::F64, f64);
impl_from!(Value::String, String);
impl_from!(Value::String, &str);
impl_from!(Value::Map, Vec<(Value, Value)>);
impl_from!(Value::Dart, DartObject);
impl_from!(Value::FinalizableHandle, Arc<FinalizableHandle>);

impl<T: Into<Value>> From<Option<T>> for Value {
    fn from(v: Option<T>) -> Self {
        match v {
            Some(v) => v.into(),
            None => Value::Null,
        }
    }
}

impl<T: Into<Value> + 'static> From<Vec<T>> for Value {
    fn from(vec: Vec<T>) -> Self {
        // TODO(knopp): Convert to match once const_type_id is stabilized;
        // Hopefully one day Rust supports specializations and the code
        // below can disappear completely
        // https://github.com/rust-lang/rust/issues/31844
        let type_id = TypeId::of::<T>();
        if type_id == TypeId::of::<i8>() {
            Value::I8List(unsafe { std::mem::transmute::<Vec<T>, Vec<i8>>(vec) })
        } else if type_id == TypeId::of::<u8>() {
            Value::U8List(unsafe { std::mem::transmute::<Vec<T>, Vec<u8>>(vec) })
        } else if type_id == TypeId::of::<i16>() {
            Value::I16List(unsafe { std::mem::transmute::<Vec<T>, Vec<i16>>(vec) })
        } else if type_id == TypeId::of::<u16>() {
            Value::U16List(unsafe { std::mem::transmute::<Vec<T>, Vec<u16>>(vec) })
        } else if type_id == TypeId::of::<i32>() {
            Value::I32List(unsafe { std::mem::transmute::<Vec<T>, Vec<i32>>(vec) })
        } else if type_id == TypeId::of::<u32>() {
            Value::U32List(unsafe { std::mem::transmute::<Vec<T>, Vec<u32>>(vec) })
        } else if type_id == TypeId::of::<i64>() {
            Value::I64List(unsafe { std::mem::transmute::<Vec<T>, Vec<i64>>(vec) })
        } else if type_id == TypeId::of::<f32>() {
            Value::F32List(unsafe { std::mem::transmute::<Vec<T>, Vec<f32>>(vec) })
        } else if type_id == TypeId::of::<f64>() {
            Value::F64List(unsafe { std::mem::transmute::<Vec<T>, Vec<f64>>(vec) })
        } else {
            Value::List(vec.into_iter().map(|v| v.into()).collect())
        }
    }
}

impl From<()> for Value {
    fn from(_: ()) -> Self {
        Value::Null
    }
}

// Allow converting any HashMap to Value as long as both key and value are
// convertible to Value.
impl<K: Into<Value>, V: Into<Value>> From<HashMap<K, V>> for Value {
    fn from(map: HashMap<K, V>) -> Self {
        let values: Vec<(Value, Value)> =
            map.into_iter().map(|(k, v)| (k.into(), v.into())).collect();
        Value::Map(values.into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TryFromError {
    BadType,
    IntConversionError,
    FloatConversionError,
    OtherError(String),
}

impl Display for TryFromError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TryFromError::BadType => write!(f, "Could not convert value from unrelated type."),
            TryFromError::IntConversionError => {
                write!(f, "Could not convert integer value to a smaller type.")
            }
            TryFromError::FloatConversionError => {
                write!(f, "Could not convert float value to a smaller type.")
            }
            TryFromError::OtherError(str) => {
                write!(f, "{str}")
            }
        }
    }
}

impl std::error::Error for TryFromError {}

impl From<TryFromIntError> for TryFromError {
    fn from(_: TryFromIntError) -> Self {
        Self::IntConversionError
    }
}

impl From<Infallible> for TryFromError {
    fn from(_: Infallible) -> Self {
        panic!("Must never happen")
    }
}

macro_rules! impl_try_from {
    ($variant:path, $for_type:ty) => {
        impl TryFrom<Value> for $for_type {
            type Error = TryFromError;
            fn try_from(v: Value) -> Result<Self, Self::Error> {
                match v {
                    $variant(d) => Ok(d.into()),
                    _ => Err(TryFromError::BadType),
                }
            }
        }
    };
}

macro_rules! impl_try_from2 {
    ($variant:path, $for_type:ty) => {
        impl TryFrom<Value> for $for_type {
            type Error = TryFromError;
            fn try_from(v: Value) -> Result<Self, Self::Error> {
                use ::core::convert::TryInto;
                match v {
                    $variant(d) => Ok(d.try_into().map_err(TryFromError::from)?),
                    _ => Err(TryFromError::BadType),
                }
            }
        }
    };
}

impl TryFrom<Value> for () {
    type Error = TryFromError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Null => Ok(()),
            _ => Err(TryFromError::BadType),
        }
    }
}

impl_try_from!(Value::Bool, bool);
impl_try_from2!(Value::I64, u8);
impl_try_from2!(Value::I64, i8);
impl_try_from2!(Value::I64, u16);
impl_try_from2!(Value::I64, i16);
impl_try_from2!(Value::I64, i32);
impl_try_from2!(Value::I64, u32);
impl_try_from!(Value::I64, i64);
impl_try_from!(Value::F64, f64);
impl_try_from!(Value::String, String);
impl_try_from!(Value::Map, ValueTupleList);
impl_try_from!(Value::Map, Vec<(Value, Value)>);
impl_try_from!(Value::Dart, DartObject);
impl_try_from!(Value::FinalizableHandle, Arc<FinalizableHandle>);

impl TryFrom<Value> for f32 {
    type Error = TryFromError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::F64(v) => {
                if v.is_nan() {
                    Ok(f32::NAN)
                } else if v.is_infinite() {
                    Ok(f32::INFINITY)
                } else {
                    let f = v as f32;
                    if (f as f64) != v {
                        Err(Self::Error::FloatConversionError)
                    } else {
                        Ok(f)
                    }
                }
            }
            _ => Err(Self::Error::BadType),
        }
    }
}

// Allow converting to any Kind of HashMap as long as key and value
// are types that can be converted from Value.
impl<
        K: TryFrom<Value, Error = E1> + Eq + Hash,
        V: TryFrom<Value, Error = E2>,
        E1: Into<TryFromError>,
        E2: Into<TryFromError>,
    > TryFrom<Value> for HashMap<K, V>
{
    type Error = TryFromError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Map(map) => map
                .into_iter()
                .map(|(k, v)| {
                    Ok((
                        k.try_into().map_err(|e: E1| e.into())?,
                        v.try_into().map_err(|e: E2| e.into())?,
                    ))
                })
                .collect(),
            _ => Err(TryFromError::BadType),
        }
    }
}

fn try_extract<T: 'static, V: 'static>(list: Vec<T>) -> Result<Vec<V>, TryFromError> {
    if TypeId::of::<V>() == TypeId::of::<T>() {
        Ok(unsafe { std::mem::transmute::<Vec<T>, Vec<V>>(list) })
    } else {
        Err(TryFromError::BadType)
    }
}

impl<V: TryFrom<Value, Error = E> + 'static, E: Into<TryFromError>> TryFrom<Value> for Vec<V> {
    type Error = TryFromError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::List(list) => list
                .into_iter()
                .map(|v| v.try_into().map_err(|e: E| e.into()))
                .collect(),
            Value::I8List(list) => try_extract(list),
            Value::U8List(list) => try_extract(list),
            Value::I16List(list) => try_extract(list),
            Value::U16List(list) => try_extract(list),
            Value::I32List(list) => try_extract(list),
            Value::U32List(list) => try_extract(list),
            Value::I64List(list) => try_extract(list),
            Value::F32List(list) => try_extract(list),
            Value::F64List(list) => try_extract(list),
            _ => Err(TryFromError::BadType),
        }
    }
}

impl Eq for Value {}

fn hash_f64<H: std::hash::Hasher>(value: f64, state: &mut H) {
    // normalize NAN
    let value: f64 = if value.is_nan() { f64::NAN } else { value };
    let transmuted: u64 = value.to_bits();
    state.write_u64(transmuted);
}

fn hash_f32<H: std::hash::Hasher>(value: f32, state: &mut H) {
    // normalize NAN
    let value: f32 = if value.is_nan() { f32::NAN } else { value };
    let transmuted: u32 = value.to_bits();
    state.write_u32(transmuted);
}

#[allow(renamed_and_removed_lints)]
#[allow(clippy::derive_hash_xor_eq)]
impl std::hash::Hash for Value {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Value::Null => state.write_u64(640),
            Value::Bool(v) => v.hash(state),
            Value::I64(v) => v.hash(state),
            Value::F64(v) => hash_f64(*v, state),
            Value::String(v) => v.hash(state),
            Value::I8List(v) => v.hash(state),
            Value::U8List(v) => v.hash(state),
            Value::I16List(v) => v.hash(state),
            Value::U16List(v) => v.hash(state),
            Value::I32List(v) => v.hash(state),
            Value::U32List(v) => v.hash(state),
            Value::I64List(v) => v.hash(state),
            Value::F32List(v) => v.iter().for_each(|x| hash_f32(*x, state)),
            Value::F64List(v) => v.iter().for_each(|x| hash_f64(*x, state)),
            Value::List(v) => v.hash(state),
            Value::Map(v) => v.hash(state),
            Value::Dart(v) => v.hash(state),
            Value::FinalizableHandle(v) => v.hash(state),
        }
    }
}

impl ValueTupleList {
    pub fn new(mut value: Vec<(Value, Value)>) -> Self {
        // Sort the list so tht hash and compares are deterministic
        if value
            .windows(2)
            .any(|w| w[0].0.partial_cmp(&w[1].0) != Some(Ordering::Less))
        {
            value.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        }
        Self(value)
    }
}

impl Deref for ValueTupleList {
    type Target = Vec<(Value, Value)>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl IntoIterator for ValueTupleList {
    type Item = (Value, Value);

    type IntoIter = std::vec::IntoIter<(Value, Value)>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl From<Vec<(Value, Value)>> for ValueTupleList {
    fn from(vec: Vec<(Value, Value)>) -> Self {
        Self::new(vec)
    }
}

impl From<HashMap<Value, Value>> for ValueTupleList {
    fn from(map: HashMap<Value, Value>) -> Self {
        let vec: Vec<_> = map.into_iter().collect();
        vec.into()
    }
}

impl From<ValueTupleList> for Vec<(Value, Value)> {
    fn from(list: ValueTupleList) -> Self {
        list.0
    }
}

impl From<ValueTupleList> for HashMap<Value, Value> {
    fn from(value: ValueTupleList) -> Self {
        value.into_iter().collect()
    }
}

impl From<DartObject> for irondash_dart_ffi::DartValue {
    fn from(object: DartObject) -> Self {
        match object {
            DartObject::SendPort(port) => port.into(),
            DartObject::Capability(capability) => capability.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{TryFromError, Value};

    #[test]
    fn test_equality() {
        let v1 = Value::Map(vec![("key1".into(), 10.into()), ("key2".into(), 20.into())].into());
        let v2 = Value::Map(vec![("key2".into(), 20.into()), ("key1".into(), 10.into())].into());
        assert_eq!(v1, v2);
    }

    #[test]
    fn test_from_list() {
        let v: Value = (vec![1i8]).into();
        assert_eq!(v, Value::I8List(vec![1]));

        let v: Value = (vec![1u8]).into();
        assert_eq!(v, Value::U8List(vec![1]));

        let v: Value = (vec![1i16]).into();
        assert_eq!(v, Value::I16List(vec![1]));

        let v: Value = (vec![1u16]).into();
        assert_eq!(v, Value::U16List(vec![1]));

        let v: Value = (vec![1i32]).into();
        assert_eq!(v, Value::I32List(vec![1]));

        let v: Value = (vec![1u32]).into();
        assert_eq!(v, Value::U32List(vec![1]));

        let v: Value = (vec![1i64]).into();
        assert_eq!(v, Value::I64List(vec![1]));

        let v: Value = (vec![1f32]).into();
        assert_eq!(v, Value::F32List(vec![1.0]));

        let v: Value = (vec![1f64]).into();
        assert_eq!(v, Value::F64List(vec![1.0]));

        let v: Value = (vec![Value::I64(10)]).into();
        assert_eq!(v, Value::List(vec![Value::I64(10)]));

        let v: Value = (vec!["abc".to_owned()]).into();
        assert_eq!(v, Value::List(vec![Value::String("abc".into())]));
    }

    #[test]
    fn test_try_into_list() -> Result<(), TryFromError> {
        let v = Value::I8List(vec![1]);
        let r: Vec<i8> = v.try_into()?;
        assert_eq!(r, vec![1i8]);

        let v = Value::U8List(vec![1]);
        let r: Vec<u8> = v.try_into()?;
        assert_eq!(r, vec![1u8]);

        let v = Value::I16List(vec![1]);
        let r: Vec<i16> = v.try_into()?;
        assert_eq!(r, vec![1i16]);

        let v = Value::U16List(vec![1]);
        let r: Vec<u16> = v.try_into()?;
        assert_eq!(r, vec![1u16]);

        let v = Value::I32List(vec![1]);
        let r: Vec<i32> = v.try_into()?;
        assert_eq!(r, vec![1i32]);

        let v = Value::U32List(vec![1]);
        let r: Vec<u32> = v.try_into()?;
        assert_eq!(r, vec![1u32]);

        let v = Value::I64List(vec![1]);
        let r: Vec<i64> = v.try_into()?;
        assert_eq!(r, vec![1i64]);

        let v = Value::F32List(vec![1.0]);
        let r: Vec<f32> = v.try_into()?;
        assert_eq!(r, vec![1f32]);

        let v = Value::F64List(vec![1.0]);
        let r: Vec<f64> = v.try_into()?;
        assert_eq!(r, vec![1f64]);

        let v = Value::List(vec![Value::I64(10)]);
        let r: Vec<i64> = v.try_into()?;
        assert_eq!(r, vec![10i64]);

        let v = Value::List(vec![Value::I64(10)]);
        let r: Vec<Value> = v.try_into()?;
        assert_eq!(r, vec![Value::I64(10)]);

        let v = Value::List(vec![Value::String("Hello".into())]);
        let r: Vec<String> = v.try_into()?;
        assert_eq!(r, vec!["Hello".to_owned()]);

        let v = Value::List(vec![Value::I64(10), Value::String("Hello".into())]);
        let r: Vec<Value> = v.try_into()?;
        assert_eq!(r, vec![Value::I64(10), Value::String("Hello".into())]);

        // Try bad conversion
        let v = Value::I8List(vec![1]);
        let r: Result<Vec<u8>, _> = v.try_into();
        assert!(r.is_err());

        Ok(())
    }
}
