use alkahest::{private::*, Deserialize, Formula, Serialize};
use std::{
    fmt::Debug,
    ops::Drop,
    sync::{Arc, Mutex, MutexGuard},
};
#[derive(Debug)]
pub struct SendMut<T: ?Sized> {
    inner: Arc<Mutex<T>>,
}
impl<T: ?Sized + Formula> Formula for SendMut<T> {
    const EXACT_SIZE: bool = <T as Formula>::EXACT_SIZE;
    const HEAPLESS: bool = <T as Formula>::HEAPLESS;
    const MAX_STACK_SIZE: Option<usize> = <T as Formula>::MAX_STACK_SIZE;
}
impl<T: Formula> BareFormula for SendMut<T> {}
impl<'de, T> Deserialize<'de, Self> for SendMut<T>
where
    T: Deserialize<'de, T> + Formula + Clone,
{
    fn deserialize(mut de: Deserializer<'de>) -> Result<Self, DeserializeError>
    where
        Self: Sized,
    {
        let formula = with_formula(|s: &T| match s {
            _ => s,
        });
        let inner_t = formula.read_field(&mut de, true)?;
        Ok(SendMut::new(inner_t))
    }
    fn deserialize_in_place(&mut self, mut de: Deserializer<'de>) -> Result<(), DeserializeError> {
        let formula = with_formula(|s: &T| match s {
            _ => s,
        });
        let inner_t = formula.read_field(&mut de, true)?;
        *self.inner.lock().unwrap() = inner_t;
        Ok(())
    }
}
impl<T: Formula + Serialize<T> + Clone> Serialize<Self> for SendMut<T> {
    fn serialize<B>(self, sizes: &mut Sizes, buffer: B) -> Result<(), B::Error>
    where
        Self: Sized,
        B: Buffer,
    {
        let inner_t = self.inner.lock().unwrap().clone();
        let formula = with_formula(|s: &T| match s {
            _ => s,
        });
        formula.write_field(inner_t, sizes, buffer, true)
    }
    fn size_hint(&self) -> Option<Sizes> {
        if let Some(sizes) = formula_fast_sizes::<Self>() {
            return Some(sizes);
        }
        let formula = with_formula(|s: &T| match s {
            _ => s,
        });
        let inner_t = formula.size_hint(&*self.inner.lock().unwrap(), true)?;
        Some(inner_t)
    }
}
impl<T: Formula + Serialize<T> + Clone> SerializeRef<Self> for SendMut<T> {
    fn serialize<B>(&self, sizes: &mut Sizes, buffer: B) -> Result<(), B::Error>
    where
        B: Buffer,
    {
        let inner_t = self.inner.lock().unwrap().clone();
        let formula = with_formula(|s: &T| match s {
            _ => s,
        });
        formula.write_field(inner_t, sizes, buffer, true)
    }
    fn size_hint(&self) -> Option<Sizes> {
        if let Some(sizes) = formula_fast_sizes::<Self>() {
            return Some(sizes);
        }
        let formula = with_formula(|s: &T| match s {
            _ => s,
        });
        let inner_t = formula.size_hint(&*self.inner.lock().unwrap(), true)?;
        Some(inner_t)
    }
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
        // Дедлок -> громкая паника с бэктрейсом вместо тихого зависания:
        // двойной .get() на одну troops-ячейку виден сразу по стеку.
        let start = std::time::Instant::now();
        loop {
            match self.inner.try_lock() {
                Ok(guard) => return guard,
                Err(std::sync::TryLockError::WouldBlock) => {
                    if start.elapsed() > std::time::Duration::from_secs(5) {
                        panic!("SendMut deadlock: lock held >5s (duplicate lock on same troop)");
                    }
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                Err(std::sync::TryLockError::Poisoned(e)) => return e.into_inner(),
            }
        }
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
        Self {
            inner: Arc::new(Mutex::new(Vec::new())),
        }
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
