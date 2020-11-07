use core::{
    fmt::{self, Formatter, Debug},
    ops::{Deref, DerefMut, Drop},
    marker::PhantomData,
};
use super::{Entry, Receiver};

/// A handle to a config entry value which is being watched by a receiver.
///
/// Such handles not only have the semantics of managing a value for a specific field, they also automatically notify the receiver whenever the value changes.
pub struct Handle<'a, E: Entry, R: Receiver<E>> {
    target: &'a mut E::Data,
    receiver: R,
    _phantom: PhantomData<E>,
}
impl<'a, E: Entry, R: Receiver<E>> Handle<'a, E, R> {
    /// Creates a handle pointing to the specified value and with the specified receiver.
    // FIXME make it a const fn when non-Sized bounds in const fn arguments get stabilized
    #[inline(always)]
    pub fn new(target: &'a mut E::Data, receiver: R) -> Self {
        Self {target, receiver, _phantom: PhantomData}
    }

    /// Sets the handle's pointee to the specified value, notifying the receiver.
    ///
    /// For large values where partial modification using a mutable reference would improve performance (`Vec` is a good example of such a type), [`modify`] or [`modify_with`] should be used instead.
    ///
    /// [`modify`]: #method.modify " "
    /// [`modify_with`]: #method.modify_with " "
    #[inline]
    pub fn set(&mut self, new_value: E::Data) {
        *self.target = new_value;
        self.receiver.receive(self.target);
    }
    /// Creates a [`ModificationScope`] for modifying the value inside without reallocating/moving and without a closure, while still notifying the receiver when modification is finished. The resulting `ModificationScope` acts like a mutable reference to the stored data, which allows direct modification.
    ///
    /// [`modify_with`] may be used instead. For small values like integers, [`set`] might be faster.
    ///
    /// [`ModificationScope`]: struct.ModificationScope.html " "
    /// [`modify_with`]: #method.modify_with " "
    /// [`set`]: #method.set " "
    #[inline(always)]
    pub fn modify<'b>(&'b mut self) -> ModificationScope<'a, 'b, E, R> {
        ModificationScope {handle: self}
    }
    /// Modifies the handle's pointee using the specified closure, notifying the receiver.
    ///
    /// [`modify`] may be used instead, for simplicity. For small values like integers, [`set`] might be faster.
    ///
    /// [`modify`]: #method.modify " "
    /// [`set`]: #method.set " "
    #[inline]
    pub fn modify_with<F>(&mut self, mut f: F)
    where F: FnMut(&mut E::Data) {
        f(&mut self.target);
        self.receiver.receive(self.target);
    }

    /// Sets the handle's pointee to the specified value without notifying the receiver. **Doing this is heavily discouraged and should only be used in special cases.**
    ///
    /// For large values where partial modification using a mutable reference would improve performance (`Vec` is a good example of such a type), [`modify_silently`] or [`modify_silently_with`] should be used instead.
    ///
    /// [`modify_silently`]: #method.modify_silently " "
    /// [`modify_silently_with`]: #method.modify_silently_with " "
    #[inline(always)]
    pub fn set_silently(&mut self, new_value: E::Data) {
        *self.target = new_value;
    }
    /// Returns a mutable reference to the handle's pointee. **This will not notify any receiver, which is heavily discouraged and should only be used in special cases.**
    ///
    /// [`modify_silently_with`] may be used instead. For small values like integers, [`set_silently`] might be faster.
    ///
    /// [`modify_silently_with`]: #method.modify_silently_with " "
    /// [`set_silently`]: #method.set_silently " "
    #[inline(always)]
    pub fn modify_silently(&mut self) -> &mut E::Data {
        self.target
    }
    /// Modifies the handle's pointee using the specified closure, without notifying the receiver. **Doing this is heavily discouraged and should only be used in special cases.**
    ///
    /// [`modify_silently`] may be used instead, for simplicity. For small values like integers, [`set_silently`] might be faster.
    ///
    /// [`modify_silently`]: #method.modify_silently " "
    /// [`set_silently`]: #method.set_silently " "
    #[inline(always)]
    pub fn modify_silently_with<F>(&mut self, mut f: F)
    where F: FnMut(&mut E::Data) {
        f(&mut self.target);
    }
}

impl<'a, E, R> Deref for Handle<'a, E, R>
where
    E: Entry,
    R: Receiver<E>,
    E::Data: Deref {
    type Target = E::Data;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.target
    }
}

impl<'a, E, R> Debug for Handle<'a, E, R>
where
    E: Entry,
    R: Receiver<E>,
    E::Data: Debug {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("EntryStorage")
            .field("name", &E::NAME)
            .field("value", &self.target)
            .finish()
    }
}

/// A drop guard for modifying data bahind a [`Handle`] using a mutable reference instead of moving in a new value.
///
/// Since `Storage` should notify a receiver whenever data inside of it is modified, it cannot simply hand out mutable references to the value, because that'd allow outside code to implicitly perform a silent storage modification. While ways to do so are also provided, it's heavily discouraged and reserved for special cases.
///
/// The solution to the problem is this struct: `ModificationScope`. It's a drop guard which is created by providing a receiver to the storage. While it has little to no differences to a mutable reference to the data inside in terms of functionality, it notifies the receiver when dropped, ensuring that it will get modified even if a panic or any other kind of early return happens.
///
/// [`Handle`]: struct.Handle.html " "
pub struct ModificationScope<'a, 'b, E, R>
where
    E: Entry,
    R: Receiver<E> {
    handle: &'b mut Handle<'a, E, R>,
}
impl<'a, 'b, E, R> Deref for ModificationScope<'a, 'b, E, R>
where
    E: Entry,
    R: Receiver<E> {
    type Target = E::Data;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.handle.target
    }
}
impl<'a, 'b, E, R> DerefMut for ModificationScope<'a, 'b, E, R>
where
    E: Entry,
    R: Receiver<E> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.handle.target
    }
}
impl<'a, 'b, E, R> AsRef<Handle<'a, E, R>> for ModificationScope<'a, 'b, E, R>
where
    E: Entry,
    R: Receiver<E> {
    fn as_ref(&self) -> &Handle<'a, E, R> {
        self.handle
    }
}
impl<'a, 'b, E, R> AsMut<Handle<'a, E, R>> for ModificationScope<'a, 'b, E, R>
where
    E: Entry,
    R: Receiver<E> {
    fn as_mut(&mut self) -> &mut Handle<'a, E, R> {
        self.handle
    }
}
impl<'a, 'b, E, R> Drop for ModificationScope<'a, 'b, E, R>
where
    E: Entry,
    R: Receiver<E> {
    fn drop(&mut self) {
        self.handle.receiver.receive(self.handle.target)
    }
}
impl<'a, 'b, E, R> Debug for ModificationScope<'a, 'b, E, R>
where
    E: Entry,
    E::Data: Debug,
    R: Receiver<E> + Debug {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("ModificationScope")
            .field("handle", &*self.handle)
            .finish()
    }
    }

/////////////////////////////////////////////////
// Trait implementation forwarding for Storage //
/////////////////////////////////////////////////
/*
impl<T: Entry> Clone for Storage<T>
where T::Data: Clone {
    #[inline(always)]
    fn clone(&self) -> Self {
        Self {value: self.value.clone(), _phantom: PhantomData}
    }
    #[inline(always)]
    fn clone_from(&mut self, source: &Self) {
        self.value.clone_from(&source.value)
    }
}

impl<T: Entry> Copy for Storage<T>
where T::Data: Copy {}

impl<T: Entry> Default for Storage<T>
where T::Data: Default {
    #[inline(always)]
    fn default() -> Self {
        Self {value: Default::default(), _phantom: PhantomData}
    }
}

impl<T: Entry> Hash for Storage<T>
where T::Data: Hash {
    #[inline(always)]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.value.hash(state)
    }
}

impl<T: Entry> PartialEq for Storage<T>
where T::Data: PartialEq {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
    // If the underlying type reimplements the != operator
    // for performance, we're not gonna intervene.
    #[allow(clippy::partialeq_ne_impl)]
    fn ne(&self, other: &Self) -> bool {
        self.value != other.value
    }
}
impl<T: Entry> Eq for Storage<T>
where T::Data: Eq {}

impl<T: Entry> PartialOrd for Storage<T>
where T::Data: PartialOrd {
    #[inline(always)]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.value.partial_cmp(&other.value)
    }
    #[inline(always)]
    fn gt(&self, other: &Self) -> bool {
        self.value > other.value
    }
    #[inline(always)]
    fn ge(&self, other: &Self) -> bool {
        self.value >= other.value
    }
    #[inline(always)]
    fn lt(&self, other: &Self) -> bool {
        self.value < other.value
    }
    #[inline(always)]
    fn le(&self, other: &Self) -> bool {
        self.value <= other.value
    }
}
impl<T: Entry> Ord for Storage<T>
where T::Data: Ord {
    #[inline(always)]
    fn cmp(&self, other: &Self) -> Ordering {
        self.value.cmp(&other.value)
    }
}

impl<T: Entry> Add for Storage<T>
where T::Data: Add<Output = T::Data> {
    type Output = Self;
    #[inline(always)]
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            value: self.value + rhs.value,
            _phantom: PhantomData,
        }
    }
}
impl<T: Entry> AddAssign for Storage<T>
where T::Data: AddAssign {
    #[inline(always)]
    fn add_assign(&mut self, rhs: Self) {
        self.value += rhs.value
    }
}

impl<T: Entry> Sub for Storage<T>
where T::Data: Sub<Output = T::Data> {
    type Output = Self;
    #[inline(always)]
    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            value: self.value - rhs.value,
            _phantom: PhantomData,
        }
    }
}
impl<T: Entry> SubAssign for Storage<T>
where T::Data: SubAssign {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: Self) {
        self.value -= rhs.value
    }
}

impl<T: Entry> Mul for Storage<T>
where T::Data: Mul<Output = T::Data> {
    type Output = Self;
    #[inline(always)]
    fn mul(self, rhs: Self) -> Self::Output {
        Self {
            value: self.value * rhs.value,
            _phantom: PhantomData,
        }
    }
}
impl<T: Entry> MulAssign for Storage<T>
where T::Data: MulAssign {
    #[inline(always)]
    fn mul_assign(&mut self, rhs: Self) {
        self.value *= rhs.value
    }
}

impl<T: Entry> Div for Storage<T>
where T::Data: Div<Output = T::Data> {
    type Output = Self;
    #[inline(always)]
    fn div(self, rhs: Self) -> Self::Output {
        Self {
            value: self.value / rhs.value,
            _phantom: PhantomData,
        }
    }
}
impl<T: Entry> DivAssign for Storage<T>
where T::Data: DivAssign {
    #[inline(always)]
    fn div_assign(&mut self, rhs: Self) {
        self.value /= rhs.value
    }
}

impl<T: Entry> Rem for Storage<T>
where T::Data: Rem<Output = T::Data> {
    type Output = Self;
    #[inline(always)]
    fn rem(self, rhs: Self) -> Self::Output {
        Self {
            value: self.value % rhs.value,
            _phantom: PhantomData,
        }
    }
}
impl<T: Entry> RemAssign for Storage<T>
where T::Data: RemAssign {
    #[inline(always)]
    fn rem_assign(&mut self, rhs: Self) {
        self.value %= rhs.value
    }
}

impl<T: Entry> Neg for Storage<T>
where T::Data: Neg<Output = T::Data> {
    type Output = Self;
    #[inline(always)]
    fn neg(self) -> Self::Output {
        Self {value: -self.value, _phantom: PhantomData}
    }
}

impl<T: Entry> Shl for Storage<T>
where T::Data: Shl<Output = T::Data> {
    type Output = Self;
    #[inline(always)]
    fn shl(self, rhs: Self) -> Self::Output {
        Self {
            value: self.value << rhs.value,
            _phantom: PhantomData,
        }
    }
}
impl<T: Entry> ShlAssign for Storage<T>
where T::Data: ShlAssign {
    #[inline(always)]
    fn shl_assign(&mut self, rhs: Self) {
        self.value <<= rhs.value
    }
}

impl<T: Entry> Shr for Storage<T>
where T::Data: Shr<Output = T::Data> {
    type Output = Self;
    #[inline(always)]
    fn shr(self, rhs: Self) -> Self::Output {
        Self {
            value: self.value >> rhs.value,
            _phantom: PhantomData,
        }
    }
}
impl<T: Entry> ShrAssign for Storage<T>
where T::Data: ShrAssign {
    #[inline(always)]
    fn shr_assign(&mut self, rhs: Self) {
        self.value >>= rhs.value
    }
}

impl<T: Entry> BitAnd for Storage<T>
where T::Data: BitAnd<Output = T::Data> {
    type Output = Self;
    #[inline(always)]
    fn bitand(self, rhs: Self) -> Self::Output {
        Self {
            value: self.value & rhs.value,
            _phantom: PhantomData,
        }
    }
}
impl<T: Entry> BitAndAssign for Storage<T>
where T::Data: BitAndAssign {
    #[inline(always)]
    fn bitand_assign(&mut self, rhs: Self) {
        self.value &= rhs.value
    }
}

impl<T: Entry> BitOr for Storage<T>
where T::Data: BitOr<Output = T::Data> {
    type Output = Self;
    #[inline(always)]
    fn bitor(self, rhs: Self) -> Self::Output {
        Self {
            value: self.value | rhs.value,
            _phantom: PhantomData,
        }
    }
}
impl<T: Entry> BitOrAssign for Storage<T>
where T::Data: BitOrAssign {
    #[inline(always)]
    fn bitor_assign(&mut self, rhs: Self) {
        self.value |= rhs.value
    }
}

impl<T: Entry> BitXor for Storage<T>
where T::Data: BitXor<Output = T::Data> {
    type Output = Self;
    #[inline(always)]
    fn bitxor(self, rhs: Self) -> Self::Output {
        Self {
            value: self.value ^ rhs.value,
            _phantom: PhantomData,
        }
    }
}
impl<T: Entry> BitXorAssign for Storage<T>
where T::Data: BitXorAssign {
    #[inline(always)]
    fn bitxor_assign(&mut self, rhs: Self) {
        self.value ^= rhs.value
    }
}

impl<T: Entry> Not for Storage<T>
where T::Data: Not<Output = T::Data> {
    type Output = Self;
    #[inline(always)]
    fn not(self) -> Self::Output {
        Self {value: !self.value, _phantom: PhantomData}
    }
}*/