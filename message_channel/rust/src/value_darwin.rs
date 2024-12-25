#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]

use std::ffi::c_void;

use core_foundation::{
    base::{CFGetTypeID, CFTypeRef},
    boolean::{CFBooleanGetTypeID, CFBooleanRef},
    number::{
        kCFNumberFloat64Type, kCFNumberSInt64Type, CFBooleanGetValue, CFNumberGetValue, CFNumberRef,
    },
};

use objc2::{
    extern_class,
    mutability::Immutable,
    rc::{autoreleasepool, Id},
    runtime::{NSObject, NSObjectProtocol},
    ClassType,
};
use objc2_foundation::{NSArray, NSCopying, NSData, NSDictionary, NSNull, NSNumber, NSString};

use crate::{TryFromError, Value};

use self::sys::from_nsstring;

/// Trait for converting Value from and to Objective C objects.
pub trait ValueObjcConversion: Sized {
    fn to_objc(&self) -> Result<Option<Id<NSObject>>, TryFromError>;
    /// # Safety
    /// This function dereferences a raw pointer. Caller is responsible for
    /// ensuring that the pointer is valid.
    unsafe fn from_objc(objc: Option<Id<NSObject>>) -> Result<Self, TryFromError>;
}

impl ValueObjcConversion for Value {
    fn to_objc(&self) -> Result<Option<Id<NSObject>>, TryFromError> {
        autoreleasepool(|_| unsafe { _value_to_objc(self) })
    }

    unsafe fn from_objc(obj: Option<Id<NSObject>>) -> Result<Self, TryFromError> {
        autoreleasepool(|_| unsafe { _value_from_objc(obj) })
    }
}

//
//
//

extern "C" {
    pub fn CFNumberIsFloatType(number: CFNumberRef) -> bool;
}

unsafe fn _value_from_objc(obj: Option<Id<NSObject>>) -> Result<Value, TryFromError> {
    let obj = match obj {
        Some(obj) => obj,
        None => {
            return Ok(Value::Null);
        }
    };
    if obj == Id::cast::<NSObject>(NSNull::null()) {
        Ok(Value::Null)
    } else if obj.is_kind_of::<NSNumber>() {
        let cf = Id::as_ptr(&obj) as CFNumberRef;
        if CFGetTypeID(cf as CFTypeRef) == CFBooleanGetTypeID() {
            Ok(Value::Bool(CFBooleanGetValue(cf as CFBooleanRef)))
        } else if CFNumberIsFloatType(cf) {
            let mut value = 0f64;
            CFNumberGetValue(
                cf as CFNumberRef,
                kCFNumberFloat64Type,
                &mut value as *mut _ as *mut c_void,
            );
            Ok(Value::F64(value))
        } else {
            let mut value = 0i64;
            CFNumberGetValue(
                cf as CFNumberRef,
                kCFNumberSInt64Type,
                &mut value as *mut _ as *mut c_void,
            );
            Ok(Value::I64(value))
        }
    } else if obj.is_kind_of::<NSString>() {
        Ok(Value::String(from_nsstring(&Id::cast::<NSString>(obj))))
    } else if obj.is_kind_of::<NSData>() {
        Ok(Value::U8List(Id::cast::<NSData>(obj).bytes().into()))
    } else if obj.is_kind_of::<NSArray>() {
        let array = Id::cast::<NSArray<NSObject>>(obj);
        let len = array.count();
        let mut res = Vec::<Value>::with_capacity(len);
        for i in 0..len {
            let item = array.objectAtIndex(i);
            res.push(_value_from_objc(Some(item))?);
        }
        Ok(Value::List(res))
    } else if obj.is_kind_of::<NSDictionary>() {
        let dict = Id::cast::<NSDictionary<NSObject, NSObject>>(obj);
        let mut entries = Vec::<(Value, Value)>::new();
        let mut keys = dict.keyEnumerator();
        loop {
            let key = keys.nextObject();
            match key {
                Some(key) => {
                    let value = dict.objectForKey(&key);
                    entries.push((_value_from_objc(Some(key))?, _value_from_objc(value)?));
                }
                None => {
                    break;
                }
            }
        }

        Ok(entries.into())
    } else {
        let class_name = obj.class().name();
        Err(TryFromError::OtherError(format!(
            "Unable to convert {} to Value",
            class_name,
        )))
    }
}

extern_class!(
    #[derive(PartialEq, Eq, Hash)]
    pub struct Copyable;

    unsafe impl ClassType for Copyable {
        type Super = NSObject;
        type Mutability = Immutable;
    }
);

unsafe impl NSCopying for Copyable {}

unsafe fn _value_to_objc(value: &Value) -> Result<Option<Id<NSObject>>, TryFromError> {
    unsafe fn fix_null(v: Option<Id<NSObject>>) -> Id<NSObject> {
        match v {
            Some(v) => v,
            None => Id::cast(NSNull::null()),
        }
    }
    unsafe fn transform_slice<T>(s: &[T]) -> &[u8] {
        std::slice::from_raw_parts(s.as_ptr() as *const u8, std::mem::size_of_val(s))
    }

    match value {
        Value::Null => Ok(None),
        Value::Bool(v) => Ok(Some(Id::cast(NSNumber::numberWithBool(*v)))),
        Value::I64(v) => Ok(Some(Id::cast(NSNumber::numberWithLongLong(*v)))),
        Value::F64(v) => Ok(Some(Id::cast(NSNumber::numberWithDouble(*v)))),
        Value::String(s) => Ok(Some(Id::cast(NSString::from_str(s)))),
        Value::U8List(d) => Ok(Some(Id::cast(NSData::with_bytes(d)))),
        Value::I8List(d) => Ok(Some(Id::cast(NSData::with_bytes(transform_slice(d))))),
        Value::U16List(d) => Ok(Some(Id::cast(NSData::with_bytes(transform_slice(d))))),
        Value::I16List(d) => Ok(Some(Id::cast(NSData::with_bytes(transform_slice(d))))),
        Value::U32List(d) => Ok(Some(Id::cast(NSData::with_bytes(transform_slice(d))))),
        Value::I32List(d) => Ok(Some(Id::cast(NSData::with_bytes(transform_slice(d))))),
        Value::I64List(d) => Ok(Some(Id::cast(NSData::with_bytes(transform_slice(d))))),
        Value::F32List(d) => Ok(Some(Id::cast(NSData::with_bytes(transform_slice(d))))),
        Value::F64List(d) => Ok(Some(Id::cast(NSData::with_bytes(transform_slice(d))))),
        Value::List(items) => {
            let res = items
                .iter()
                .map(|v| _value_to_objc(v).map(|v| fix_null(v)))
                .collect::<Result<Vec<_>, TryFromError>>()?;

            Ok(Some(Id::cast(NSArray::from_vec(res))))
        }
        Value::Map(items) => {
            let mut keys = Vec::<Id<Copyable>>::with_capacity(items.len());
            let mut objects = Vec::<Id<NSObject>>::with_capacity(items.len());
            for item in items.iter() {
                keys.push(Id::cast(fix_null(_value_to_objc(&item.0)?)));
                objects.push(fix_null(_value_to_objc(&item.1)?));
            }
            let key_refs = keys.iter().map(|v| v.as_ref()).collect::<Vec<_>>();
            let dict = NSDictionary::from_vec(&key_refs, objects);
            Ok(Some(Id::cast(dict)))
        }
        other => Err(TryFromError::OtherError(format!(
            "Unable to convert {other:?} to Objc",
        ))),
    }
}

#[cfg(test)]
mod test {
    use objc2::{rc::Id, runtime::NSObject};
    use objc2_foundation::{ns_string, NSArray, NSData, NSDictionary, NSNull, NSNumber, NSString};

    use crate::Value;

    use super::ValueObjcConversion;

    #[test]
    #[cfg(target_endian = "little")]
    fn test_coerce_data() {
        use objc2::rc::Id;
        use objc2_foundation::NSData;

        use crate::{value_darwin::ValueObjcConversion, Value};

        let v: Value = vec![1i8, 2i8, 3i8].into();
        assert!(unsafe {
            v.to_objc().unwrap() == Some(Id::cast(NSData::with_bytes(&[1u8, 2u8, 3u8])))
        });

        let v: Value = vec![1i8, 2i8, 3i8].into();
        assert!(unsafe {
            v.to_objc().unwrap() == Some(Id::cast(NSData::with_bytes(&[1u8, 2u8, 3u8])))
        });

        unsafe fn transform_slice<T>(s: &[T]) -> &[u8] {
            std::slice::from_raw_parts(s.as_ptr() as *const u8, std::mem::size_of_val(s))
        }

        let v: Value = vec![1f32, 2f32].into();
        assert!(unsafe {
            v.to_objc().unwrap()
                == Some(Id::cast(NSData::with_bytes(transform_slice(&[1f32, 2f32]))))
        });

        let v: Value = vec![1f64, 2f64].into();
        assert!(unsafe {
            v.to_objc().unwrap()
                == Some(Id::cast(NSData::with_bytes(transform_slice(&[1f64, 2f64]))))
        });
    }

    #[test]
    #[cfg(target_endian = "little")]
    fn test_coerce_data_l() {
        use objc2::rc::Id;
        use objc2_foundation::NSData;

        use crate::{value_darwin::ValueObjcConversion, Value};

        let v: Value = vec![1u16, 2u16, 3u16].into();
        assert!(unsafe {
            v.to_objc().unwrap()
                == Some(Id::cast(NSData::with_bytes(&[
                    1u8, 0u8, 2u8, 0u8, 3u8, 0u8,
                ])))
        });

        let v: Value = vec![1i16, 2i16, 3i16].into();
        assert!(unsafe {
            v.to_objc().unwrap()
                == Some(Id::cast(NSData::with_bytes(&[
                    1u8, 0u8, 2u8, 0u8, 3u8, 0u8,
                ])))
        });

        let v: Value = vec![1u32].into();
        assert!(unsafe {
            v.to_objc().unwrap() == Some(Id::cast(NSData::with_bytes(&[1u8, 0u8, 0u8, 0u8])))
        });

        let v: Value = vec![1i32].into();
        assert!(unsafe {
            v.to_objc().unwrap() == Some(Id::cast(NSData::with_bytes(&[1u8, 0u8, 0u8, 0u8])))
        });

        let v: Value = vec![1i64].into();
        assert!(unsafe {
            v.to_objc().unwrap()
                == Some(Id::cast(NSData::with_bytes(&[
                    1u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                ])))
        });
    }

    #[test]
    fn tests() {
        let object1 = unsafe {
            NSDictionary::<NSString, NSObject>::from_vec(
                &[ns_string!("Key")],
                vec![Id::cast(NSArray::<NSObject>::from_id_slice(&[
                    Id::cast(NSString::from_str("Obj1")),
                    Id::cast(NSNumber::numberWithBool(false)),
                    Id::cast(NSNumber::numberWithBool(true)),
                    Id::cast(NSNumber::numberWithInt(5)),
                    Id::cast(NSNumber::numberWithFloat(10.0f32)),
                    Id::cast(NSNumber::numberWithDouble(15.0f64)),
                    Id::cast(NSData::with_bytes(&[1, 2, 3])),
                    Id::cast(NSNull::null()),
                ]))],
            )
        };
        let value = unsafe { Value::from_objc(Some(Id::cast(object1.clone()))).unwrap() };
        let our_value: Value = vec![(
            "Key".into(),
            vec![
                Value::String("Obj1".into()),
                false.into(),
                true.into(),
                5i64.into(),
                10f64.into(),
                15f64.into(),
                vec![1u8, 2u8, 3u8].into(),
                Value::Null,
            ]
            .into(),
        )]
        .into();

        assert_eq!(value, our_value);

        let objc = our_value.to_objc().unwrap();
        assert!(objc == unsafe { Some(Id::cast(object1)) });
    }
}

mod sys {
    use std::{ffi::c_char, slice};

    use objc2::rc::Id;
    use objc2_foundation::{NSString, NSUTF8StringEncoding};

    pub unsafe fn from_nsstring(ns_string: &Id<NSString>) -> String {
        let bytes: *const c_char = ns_string.UTF8String();
        let bytes = bytes as *const u8;

        let len = ns_string.lengthOfBytesUsingEncoding(NSUTF8StringEncoding);

        // IntelliJ likes to put NULL terminator in the string, because why not.
        let mut bytes = slice::from_raw_parts(bytes, len);
        while bytes.last() == Some(&0) {
            bytes = &bytes[..bytes.len() - 1];
        }
        std::str::from_utf8(bytes).unwrap().into()
    }
}
