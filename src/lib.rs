//! Ergonomic and thread safe version of Box<dyn Any>.


use std::{any::{Any, TypeId}, fmt::Debug, mem};
use downcast_rs::{impl_downcast, Downcast};

const _: Option<Box<dyn DynObject>> = None;

/// A type that can be boxed into a [`Object`].
/// 
/// The trait bounds [`Clone`], [`Debug`] and [`PartialEq`] are required for maximum usability.
pub trait DynObject: Downcast + Debug + Send + Sync + 'static {
    fn dyn_clone(&self) -> Box<dyn DynObject>;
    fn dyn_eq(&self, other: &dyn DynObject) -> bool;
}

impl_downcast!(DynObject);

impl Clone for Box<dyn DynObject> {
    fn clone(&self) -> Self {
        self.as_ref().dyn_clone()
    }
}

impl PartialEq for dyn DynObject {
    fn eq(&self, other: &Self) -> bool {
        self.dyn_eq(other)
    }
}

impl<T> DynObject for T where T: Debug + Clone + PartialEq + Send + Sync + 'static{
    fn dyn_clone(&self) -> Box<dyn DynObject> {
        Box::new(self.clone())
    }

    fn dyn_eq(&self, other: &dyn DynObject) -> bool {
        match other.downcast_ref::<T>() {
            Some(some) => some == self,
            None => false,
        }
    }
}

/// A type that can converted to and from [`Object`].
pub trait AsObject: Sized + Debug + Clone + Send + Sync + 'static {
    fn cloned(obj: &Object) -> Option<Self>;
    fn get_ref(obj: &Object) -> Option<&Self>;
    fn get_mut(obj: &mut Object) -> Option<&mut Self>;
    fn from_object(obj: Object) -> Option<Self>;
    fn into_object(self) -> Object;
    fn as_dyn_inner(&self) -> Option<&dyn DynObject>;
}

impl<T> AsObject for T where T: DynObject + Clone {
    fn cloned(obj: &Object) -> Option<Self> {
        if TypeId::of::<T>() == TypeId::of::<Object>() {
            if obj.is_none() { return None; };
            Some((obj as &dyn Any).downcast_ref::<T>().unwrap().clone())
        } else {
            obj.0.as_ref().and_then(|x| x.downcast_ref::<T>().cloned())
        }
    }

    fn get_ref(obj: &Object) -> Option<&Self> {
        if TypeId::of::<T>() == TypeId::of::<Object>() {
            if obj.is_none() { return None; };
            Some((obj as &dyn Any).downcast_ref::<T>().unwrap())
        } else {
            obj.0.as_ref().and_then(|x| x.downcast_ref())
        }
    }
    
    fn get_mut(obj: &mut Object) -> Option<&mut Self> {
        if TypeId::of::<T>() == TypeId::of::<Object>() {
            if obj.is_none() { return None; };
            Some((obj as &mut dyn Any).downcast_mut::<T>().unwrap())
        } else {
            obj.0.as_mut().and_then(|x| x.downcast_mut())
        }
    }
    
    fn from_object(obj: Object) -> Option<Self> {
        if TypeId::of::<T>() == TypeId::of::<Object>() {
            if obj.is_none() { return None; };
            Some(*(Box::new(obj) as Box<dyn Any>).downcast::<T>().unwrap())
        } else {
            obj.0.and_then(|x| x.downcast().map(|x| *x).ok())
        }
    }

    fn into_object(self) -> Object {
        if TypeId::of::<T>() == TypeId::of::<Object>() {
            *(Box::new(self) as Box<dyn Any>).downcast::<Object>().unwrap()
        } else {
            Object(Some(Box::new(self)))
        }
    }

    fn as_dyn_inner(&self) -> Option<&dyn DynObject> {
        if TypeId::of::<T>() == TypeId::of::<Object>() {
            (self as &dyn Any)
                .downcast_ref::<Object>()
                .unwrap()
                .0
                .as_ref()
                .map(|x| x.as_ref())
        } else {
            Some(self)
        }
    }
}

/// A boxed type erased nullable dynamic object.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Object(Option<Box<dyn DynObject>>);

impl Object {
    /// A `None` object.
    /// 
    /// ```
    /// # use dyn_object::Object;
    /// assert!(!Object::NONE.is_some());
    /// assert!(Object::NONE.is_none());
    /// ```
    pub const NONE: Self = Self(None);

    /// Create an unnameable [`Object`] that is never equal to anything.
    /// 
    /// ```
    /// # use dyn_object::Object;
    /// assert!(Object::unnameable().is_some());
    /// assert_ne!(Object::unnameable(), Object::unnameable());
    /// ```
    pub fn unnameable() -> Self {
        #[derive(Debug, Clone)]
        struct UnnameableUnequal;

        impl PartialEq for UnnameableUnequal{
            fn eq(&self, _: &Self) -> bool {
                false
            }
        }
        Self(Some(Box::new(UnnameableUnequal)))
    }

    /// Create a new object from a value.
    /// 
    /// ```
    /// # use dyn_object::Object;
    /// Object::new(1);
    /// Object::new("Ferris");
    /// ```
    /// 
    /// ## Guarantees
    /// 
    /// `Object::new` will move instead of box if given another object.
    /// 
    /// ```
    /// # use dyn_object::Object;
    /// assert_eq!(Object::new(1), Object::new(Object::new(1)));
    /// ```
    pub fn new<T: AsObject>(v: T) -> Self {
        AsObject::into_object(v)
    }

    /// Return `true` if object is not `None`.
    pub fn is_some(&self) -> bool {
        self.0.is_some()
    }

    /// Return `true` if object is `None`.
    pub fn is_none(&self) -> bool {
        self.0.is_none()
    }

    /// Try obtain the value by cloning.
    /// 
    /// ```
    /// # use dyn_object::Object;
    /// assert_eq!(Object::new(1).cloned::<i32>().unwrap(), 1);
    /// assert!(Object::new(1).cloned::<String>().is_none());
    /// assert_eq!(Object::new(1).cloned::<Object>().unwrap(), Object::new(1));
    /// assert!(Object::NONE.cloned::<Object>().is_none());
    /// ```
    pub fn cloned<T: AsObject>(&self) -> Option<T> {
        AsObject::cloned(self)
    }

    /// Try obtain the value's reference.
    /// 
    /// ```
    /// # use dyn_object::Object;
    /// assert_eq!(Object::new(1).get_ref::<i32>(), Some(&1));
    /// assert_eq!(Object::new(1).get_ref::<String>(), None);
    /// assert_eq!(Object::new(1).get_ref::<Object>(), Some(&Object::new(1)));
    /// assert_eq!(Object::NONE.get_ref::<Object>(), None);
    /// ```
    pub fn get_ref<T: AsObject>(&self) -> Option<&T> {
        AsObject::get_ref(self)
    }

    /// Try obtain the value's mutable reference.
    /// 
    /// ```
    /// # use dyn_object::Object;
    /// assert_eq!(Object::new(1).get_mut::<i32>(), Some(&mut 1));
    /// assert_eq!(Object::new(1).get_mut::<String>(), None);
    /// assert_eq!(Object::new(1).get_mut::<Object>(), Some(&mut Object::new(1)));
    /// assert_eq!(Object::NONE.get_mut::<Object>(), None);
    /// ```
    pub fn get_mut<T: AsObject>(&mut self) -> Option<&mut T> {
        AsObject::get_mut(self)
    }

    /// Remove the value from the object, leaving behind a `Object::NONE`.
    /// 
    /// ```
    /// # use dyn_object::Object;
    /// let mut obj = Object::new(4);
    /// assert!(obj.is_some());
    /// obj.clear();
    /// assert!(obj.is_none());
    /// ```
    pub fn clear(&mut self) {
        self.0.take();
    }

    /// Take the value from the object, leaving behind a `Object::NONE`.
    /// 
    /// ```
    /// # use dyn_object::Object;
    /// let mut obj = Object::new(5);
    /// assert!(obj.is_some());
    /// assert_eq!(obj.take(), Some(5));
    /// assert!(obj.is_none());
    /// ```
    pub fn take<T: AsObject>(&mut self) -> Option<T> {
        AsObject::from_object(mem::take(self))
    }

    /// Set the value of the object.
    pub fn set<T: AsObject>(&mut self, v: T) {
        *self = AsObject::into_object(v)
    }

    /// Swap the value of the object with another value.
    pub fn replace<A: AsObject, B: AsObject>(&mut self, v: A) -> Option<B>{
        let original = self.take();
        self.set(v);
        original
    }

    /// If none, box another value as a new object.
    pub fn or<T: AsObject>(self, item: T) -> Object {
        if self.is_none() {
            Object::new(item)
        } else {
            self
        }
    }

    /// If none, box another value as a new object.
    pub fn or_else<T: AsObject>(self, item: impl Fn() -> T) -> Object {
        if self.is_none() {
            Object::new(item())
        } else {
            self
        }
    }

    /// Compare Object to a value that can be converted to an object.
    /// 
    /// ```
    /// # use dyn_object::Object;
    /// let mut obj = Object::new(5);
    /// assert!(obj.equals(&5));
    /// assert!(!obj.equals(&6));
    /// ```
    pub fn equals<T: AsObject>(&self, other: &T) -> bool {
        match (&self.0, other.as_dyn_inner())  {
            (None, None) => true,
            (Some(a), Some(b)) => a.as_ref().dyn_eq(b),
            _ => false
        }
    }
}
