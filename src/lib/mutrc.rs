use std::{
    rc::Rc,
    cell::{RefCell, RefMut, Ref},
};


#[derive(Debug)]
pub struct MutRc<T: ?Sized>{
    rc: Rc<RefCell<T>>
}
impl<T> MutRc<T> where T: ?Sized + Clone{

    pub fn new(value: T) -> Self {
        Self {
            rc: Rc::new(RefCell::new(value))
    }   }
    pub fn borrow_mut(&self) -> RefMut<T> {
        self.rc.borrow_mut()
    }
    pub fn borrow(&self) -> Ref<T> {
        self.rc.borrow()
    }
    pub fn clone(&self) -> Self {
        self.rc.clone().into()
}   }
impl<T> Clone for MutRc<T> where T: ?Sized + Clone {
    fn clone(&self) -> MutRc<T> {
        self.clone()
}   }
impl<T> From<Rc<RefCell<T>>> for MutRc<T> {
    fn from(value: Rc<RefCell<T>>) -> Self {
        Self { rc: value }
}   }
impl<T> From<T> for MutRc<T> where T: ?Sized + Clone {
    fn from(value: T) -> Self {
        Self::new(value)
}   }