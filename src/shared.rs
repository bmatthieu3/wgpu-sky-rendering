use std::rc::Rc;
use std::cell::RefCell;

pub struct Shared<T> {
    v: Rc<RefCell<T>>
}

impl<T> Clone for Shared<T> {
    fn clone(&self) -> Self {
        Shared {v: self.v.clone()}
    }
}

impl <T> Shared<T> {
    pub fn new(t: T)-> Shared<T> {
        Shared{ v: Rc::new(RefCell::new(t)) }
    }
}

use std::cell::{Ref, RefMut};
use std::fmt;
impl <T> Shared<T> {
    /*pub fn borrow(&self) -> Ref<T> {
        self.v.borrow()
    }

    pub fn borrow_mut(&self) -> RefMut<T> {
        self.v.borrow_mut()
    }*/

    pub fn as_ptr(&self) -> *mut T {
        self.v.as_ptr()
    }
}


impl <T: fmt::Display> fmt::Display for Shared<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.deref())
    }
}

impl <T> fmt::Debug for Shared<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "shared ptr")
    }
}
use std::ops::Deref;
impl <'a,T> Deref for Shared<T>{
    type Target = T;

    #[inline]
    fn deref(&self) -> &T {
        unsafe { self.as_ptr().as_ref().unwrap() }
    }
}

use std::ops::DerefMut;
impl <'a,T> DerefMut for Shared<T>{
    #[inline]
    fn deref_mut(&mut self) -> &mut T {
        unsafe { self.as_ptr().as_mut().unwrap() }
    }
}