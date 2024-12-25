/// Internal helpers for deriving TryFromValue and IntoValue
/// We need these traits to be specialized for <T> and Option<T>, see
/// https://lukaskalbertodt.github.io/2019/12/05/generalized-autoref-based-specialization.html
/// for details on how this works.
use std::{
    convert::{TryFrom, TryInto},
    result::Result,
};

use crate::{TryFromError, Value};

pub struct WrapMut<'a, T>(pub &'a mut T);

pub struct Wrap<'a, T>(pub &'a T);

pub trait Assign {
    fn assign(&mut self, value: Value, skip_if_empty: bool) -> Result<(), TryFromError>;
    fn set_optional_to_none(&mut self);
}

impl<T: TryFrom<Value, Error = E>, E> Assign for WrapMut<'_, Option<T>>
where
    E: Into<TryFromError>,
{
    fn assign(&mut self, value: Value, _skip_if_empty: bool) -> Result<(), TryFromError> {
        self.0.replace(value.try_into().map_err(|e: E| e.into())?);
        Ok(())
    }
    fn set_optional_to_none(&mut self) {}
}

impl<T: TryFrom<Value, Error = E>, E> Assign for &mut WrapMut<'_, Option<Option<T>>>
where
    E: Into<TryFromError>,
{
    fn assign(&mut self, value: Value, _skip_if_empty: bool) -> Result<(), TryFromError> {
        match value {
            Value::Null => self.0.replace(Option::<T>::None),
            v => self.0.replace(Some(v.try_into().map_err(|e: E| e.into())?)),
        };
        Ok(())
    }
    fn set_optional_to_none(&mut self) {
        if self.0.is_none() {
            self.0.replace(None);
        }
    }
}

impl Assign for &mut &mut WrapMut<'_, Option<Option<Value>>> {
    fn assign(&mut self, value: Value, skip_if_empty: bool) -> Result<(), TryFromError> {
        if skip_if_empty {
            self.0.replace(Some(value));
        } else {
            match value {
                Value::Null => self.0.replace(None),
                v => self.0.replace(Some(v)),
            };
        }
        Ok(())
    }
    fn set_optional_to_none(&mut self) {
        if self.0.is_none() {
            self.0.replace(None);
        }
    }
}

pub trait IsNone {
    fn is_none(&self) -> bool;
}

impl<T> IsNone for Wrap<'_, T> {
    fn is_none(&self) -> bool {
        false
    }
}

impl<T> IsNone for &Wrap<'_, Option<T>> {
    fn is_none(&self) -> bool {
        self.0.is_none()
    }
}
