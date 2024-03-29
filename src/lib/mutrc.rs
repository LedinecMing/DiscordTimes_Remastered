use std::{sync::{Arc, Mutex, MutexGuard}, ops::Drop, fmt::Debug};

#[derive(Debug)]
pub struct SendMut<T: ?Sized> {
    inner: Arc<Mutex<T>>,
}
impl<T> SendMut<T>
where
    T: ?Sized + Clone,
{
    pub fn new(value: T) -> Self {
        Self {
            inner: Arc::new(Mutex::new(value)),
        }
    }
    pub fn get(&self) -> MutexGuard<'_, T> {
        self.inner.lock().unwrap()
    }
    pub fn clone(&self) -> Self {
        self.inner.clone().into()
    }
}
impl<T> Clone for SendMut<T>
where
    T: ?Sized + Clone,
{
    fn clone(&self) -> SendMut<T> {
        self.clone()
    }
}
impl<T> From<Arc<Mutex<T>>> for SendMut<T> {
    fn from(value: Arc<Mutex<T>>) -> Self {
        Self { inner: value }
    }
}
impl<T> Default for SendMut<Vec<T>> {
    fn default() -> Self {
		Self { inner: Arc::new(Mutex::new(Vec::new())) }
	}
}
impl<T> From<T> for SendMut<T>
where
    T: ?Sized + Clone,
{
    fn from(value: T) -> Self {
        Self::new(value)
    }
}
impl<T: ?Sized> Drop for SendMut<T> {
	fn drop(&mut self) {}
}
