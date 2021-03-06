use core::{
    fmt::{self, Formatter, Debug},
    ops::DerefMut,
    cell::RefCell,
    marker::PhantomData,
};
use alloc::{
    boxed::Box,
    sync::Arc,
    rc::Rc,
};
#[cfg(feature = "std")]
use std::{
    sync::{Mutex, RwLock},
};
use super::Entry;

/// Trait for types which wish to be notified when the specified configuration table entry changes.
///
/// Several reference types and standard library types implement `Receiver`:
/// - A mutable borrow of any type can be used as a receiver
pub trait Receiver<E: Entry> {
    /// Receive a notification about the value of the entry changing to the specified new value.
    ///
    /// This method shouldn't be called manually — please use [`EntryStorage`] instead, which will automatically call this method. It's a logic error to invoke this without actually setting the value to something new in the storage.
    ///
    /// [`EntryStorage`]: struct.EntryStorage.html " "
    fn receive(&mut self, new_value: &E::Data);
}

/// A [receiver] which calls a closure when notified.
///
/// [receiver]: trait.Receiver.html " "
#[allow(clippy::module_name_repetitions)]
pub struct FnReceiver<E: Entry, F: FnMut(&E::Data) = Box<dyn FnMut(&<E as Entry>::Data)>> {
    _phantom: PhantomData<E>,
    /// The closure which is called when the receiver is notified.
    pub closure: F,
}
impl<E: Entry, F: FnMut(&E::Data)> FnReceiver<E, F> {
    /// Creates a new receiver from the specified closure.
    // FIXME make it a const fn when non-Sized bounds in const fn arguments get stabilized
    #[inline(always)]
    pub fn new(closure: F) -> Self {
        Self {closure, _phantom: PhantomData}
    }
}
impl<E: Entry, F: FnMut(&E::Data)> Receiver<E> for FnReceiver<E, F> {
    #[inline(always)]
    fn receive(&mut self, new_value: &E::Data) {
        (self.closure)(new_value)
    }
}
impl<E: Entry, F: Fn(&E::Data)> Receiver<E> for &FnReceiver<E, F> {
    #[inline(always)]
    fn receive(&mut self, new_value: &E::Data) {
        (self.closure)(new_value)
    }
}
impl<E: Entry, F: FnMut(&E::Data) + Clone> Clone for FnReceiver<E, F> {
    #[inline(always)]
    fn clone(&self) -> Self {
        Self {closure: self.closure.clone(), _phantom: PhantomData}
    }
}
impl<E: Entry, F: FnMut(&E::Data) + Copy> Copy for FnReceiver<E, F>
{}

impl<E: Entry, F: FnMut(&E::Data) + Default> Default for FnReceiver<E, F> {
    #[inline(always)]
    fn default() -> Self {
        Self {closure: F::default(), _phantom: PhantomData}
    }
}

impl<E: Entry, F: FnMut(&E::Data) + Debug> Debug for FnReceiver<E, F> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("FnReceiver")
            .field("closure", &self.closure)
            .finish()
    }
}

/// A [receiver] which creates an iterator from a reference to the contained value and notifies all items which the iterator produces.
///
/// [receiver]: trait.Receiver.html " "
#[allow(clippy::module_name_repetitions)]
pub struct IterReceiver<E: Entry, I>
where
    for<'a> &'a mut I: IntoIterator,
    for<'a> <&'a mut I as IntoIterator>::Item: Receiver<E> {
    /// The iterable which produces iterators over the receivers.
    pub iter: I,
    _phantom: PhantomData<E>,
}
impl<E: Entry, I> Receiver<E> for IterReceiver<E, I>
where
    for<'a> &'a mut I: IntoIterator,
    for<'a> <&'a mut I as IntoIterator>::Item: Receiver<E> {
    #[inline]
    fn receive(&mut self, new_value: &E::Data) {
        for mut receiver in &mut self.iter {
            receiver.receive(new_value);
        }
    }
}
impl<E: Entry, I> Receiver<E> for &IterReceiver<E, I>
where
    for<'a> &'a mut I: IntoIterator,
    for<'a> &'a I: IntoIterator,
    for<'a> <&'a mut I as IntoIterator>::Item: Receiver<E>,
    for<'a> <&'a I as IntoIterator>::Item: Receiver<E> {
    #[inline]
    fn receive(&mut self, new_value: &E::Data) {
        for mut receiver in &self.iter {
            receiver.receive(new_value);
        }
    }
}
impl<E: Entry, I> IterReceiver<E, I>
where
    for<'a> &'a mut I: IntoIterator,
    for<'a> <&'a mut I as IntoIterator>::Item: Receiver<E> {
    /// Creates a new receiver which notifies the specified iterable of receivers.
    // FIXME make it a const fn when non-Sized bounds in const fn arguments get stabilized
    #[inline(always)]
    pub fn new(iter: I) -> Self {
        Self {iter, _phantom: PhantomData}
    }
}
impl<E: Entry, I: Clone> Clone for IterReceiver<E, I>
where
    for<'a> &'a mut I: IntoIterator,
    for<'a> <&'a mut I as IntoIterator>::Item: Receiver<E> {
    #[inline(always)]
    fn clone(&self) -> Self {
        Self {iter: self.iter.clone(), _phantom: PhantomData}
    }
}
impl<E: Entry, I: Copy> Copy for IterReceiver<E, I>
where
    for<'a> &'a mut I: IntoIterator,
    for<'a> <&'a mut I as IntoIterator>::Item: Receiver<E> {}

impl<E: Entry, I: Default> Default for IterReceiver<E, I>
where
    for<'a> &'a mut I: IntoIterator,
    for<'a> <&'a mut I as IntoIterator>::Item: Receiver<E> {
    #[inline(always)]
    fn default() -> Self {
        Self {iter: I::default(), _phantom: PhantomData}
    }
}

impl<E: Entry, I: Debug> Debug for IterReceiver<E, I>
where
    for<'a> &'a mut I: IntoIterator,
    for<'a> <&'a mut I as IntoIterator>::Item: Receiver<E> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("IterReceiver")
            .field("iter", &self.iter)
            .finish()
    }
}

/// A [receiver] which does nothing when notified.
///
/// [receiver]: trait.Receiver.html " "
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct EmptyReceiver;
impl<E: Entry> Receiver<E> for EmptyReceiver {
    #[inline(always)]
    fn receive(&mut self, _: &E::Data) {}
}
impl<E: Entry> Receiver<E> for &EmptyReceiver {
    #[inline(always)]
    fn receive(&mut self, _: &E::Data) {}
}

//────────────────────────────────────────────────────—┐
// Receiver implementations for builtins and std types |
//─────────────────────────────────────────────────────┘

impl<E, R> Receiver<E> for &mut R
where
    E: Entry,
    R: Receiver<E> + ?Sized {
    #[inline(always)]
    fn receive(&mut self, new_value: &E::Data) {
        (*self).receive(new_value);
    }
}
impl<E, R> Receiver<E> for Option<R>
where
    E: Entry,
    R: Receiver<E> {
    #[inline]
    fn receive(&mut self, new_value: &E::Data) {
        if let Some(receiver) = self.as_mut() {
            receiver.receive(new_value);
        }
    }
}
impl<E, R> Receiver<E> for &Option<R>
where
    E: Entry,
    for<'a> &'a R: Receiver<E> {
    #[inline]
    fn receive(&mut self, new_value: &E::Data) {
        if let Some(mut receiver) = self.as_ref() {
            receiver.receive(new_value);
        }
    }
}
impl<E, R> Receiver<E> for Box<R>
where
    E: Entry,
    R: Receiver<E> + ?Sized {
    #[inline(always)]
    fn receive(&mut self, new_value: &E::Data) {
        self.deref_mut().receive(new_value);
    }
}
impl<E, R> Receiver<E> for &Box<R>
where
    E: Entry,
    R: ?Sized,
    for<'a> &'a R: Receiver<E> {
    #[inline(always)]
    fn receive(&mut self, new_value: &E::Data) {
        (&***self).receive(new_value);
    }
}
impl<E, R> Receiver<E> for Rc<R>
where
    E: Entry,
    R: ?Sized,
    for<'a> &'a R: Receiver<E> {
    #[inline(always)]
    fn receive(&mut self, new_value: &E::Data) {
        (&**self).receive(new_value);
    }
}
impl<E, R> Receiver<E> for Arc<R>
where
    E: Entry,
    R: ?Sized,
    for<'a> &'a R: Receiver<E> {
    #[inline(always)]
    fn receive(&mut self, new_value: &E::Data) {
        (&**self).receive(new_value);
    }
}

impl<E, R> Receiver<E> for RefCell<R>
where
    E: Entry,
    R: Receiver<E> + ?Sized {
    #[inline(always)]
    fn receive(&mut self, new_value: &E::Data) {
        self.get_mut().receive(new_value);
    }
}
impl<E, R> Receiver<E> for &RefCell<R>
where
    E: Entry,
    R: Receiver<E> + ?Sized {
    #[inline(always)]
    fn receive(&mut self, new_value: &E::Data) {
        (*self.borrow_mut()).receive(new_value);
    }
}

#[cfg(feature = "std")]
static POISONING_MSG: &str = "attempt to use a poisoned lock as a receiver";
#[cfg(feature = "std")]
impl<E, R> Receiver<E> for Mutex<R>
where
    E: Entry,
    R: Receiver<E> + ?Sized {
    #[inline(always)]
    fn receive(&mut self, new_value: &E::Data) {
        self.get_mut().expect(POISONING_MSG).receive(new_value);
    }
}
#[cfg(feature = "std")]
impl<E, R> Receiver<E> for &Mutex<R>
where
    E: Entry,
    R: Receiver<E> + ?Sized {
    #[inline(always)]
    fn receive(&mut self, new_value: &E::Data) {
        self.lock().expect(POISONING_MSG).receive(new_value);
    }
}
#[cfg(feature = "std")]
impl<E, R> Receiver<E> for RwLock<R>
where
    E: Entry,
    R: Receiver<E> + ?Sized {
    #[inline(always)]
    fn receive(&mut self, new_value: &E::Data) {
        self.get_mut().expect(POISONING_MSG).receive(new_value);
    }
}
#[cfg(feature = "std")]
impl<E, R> Receiver<E> for &RwLock<R>
where
    E: Entry,
    R: Receiver<E> + ?Sized {
    #[inline(always)]
    fn receive(&mut self, new_value: &E::Data) {
        self.write().expect(POISONING_MSG).receive(new_value);
    }
}