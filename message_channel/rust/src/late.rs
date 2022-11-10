use std::cell::UnsafeCell;

/// Cell implementation that supports late initialization and can only be set once;
/// Panics if data is accessed before set has been called or if set is called more than once.
pub struct Late<T> {
    value: UnsafeCell<Option<T>>,
}

impl<T> Late<T> {
    /// Creates a new empty Late cell. You must call `set()` in order be able to
    /// dereference the cell and access teh value.
    pub fn new() -> Self {
        Self {
            value: UnsafeCell::new(None),
        }
    }

    /// Returns whether this Late cell has value already set.
    pub fn is_set(&self) -> bool {
        let value = unsafe { &mut *self.value.get() };
        value.is_some()
    }

    /// Sets the value of this cell. Panics if called more than once.
    pub fn set(&self, new_value: T) {
        let value = unsafe { &mut *self.value.get() };
        match value {
            Some(_) => {
                panic!("Value is already set");
            }
            None => *value = Some(new_value),
        }
    }
}

impl<T> std::ops::Deref for Late<T> {
    type Target = T;

    fn deref(&self) -> &T {
        let value = unsafe { &*self.value.get() };
        match value {
            Some(value) => value,
            None => panic!("Late Value has not been set"),
        }
    }
}
