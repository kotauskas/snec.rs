use super::{Receiver, Handle};

/// Trait for type-level identifiers for config entries.
///
/// All implementors should be uninhabited types, as the creation of a type implementing `Entry` doesn't make sense: it's just a type-level marker identifying a configuration field. There are two ways to create such a type:
/// ```rust
/// enum MyUninhabited {}
/// # /*
/// // Works only on the nightly version of the compiler as of Rust 1.46
// FIXME remove this notice when the never type gets stabilized
/// struct MyUninhabited (!);
/// # */
/// ```
/// The first one is recommended, as it works on the stable version of the compiler. When the [`!` type gets stabilized], the second could be used as well, depending on your preference.
///
/// [`!` type gets stabilized]: https://github.com/rust-lang/rust/issues/35121 " "
pub trait Entry: Sized {
    /// The data value that the entry expects.
    type Data;
    /// The textual representation of the name of the entry. Should follow the same naming convention as struct fields and variables, i.e. `snake_case`.
    const NAME: &'static str;
}

/// Trait for getting handles to fields in config tables.
///
/// This trait is implemented by config tables for every `E` which is a field inside the table.
///
/// # Collisions with `GetExt`
/// Since the [`GetExt`] trait uses the same method names as `Get`, bringing both into scope will require disambiguation and thus will defeat the point of `GetExt`, which is to make turbofish syntax usage possible. For that reason, avoid bringing `Get` into scope and instead use a fully qualified `snec::Get`.
pub trait Get<E>
where E: Entry {
    /// The [receiver] which will be notified when modifications are performed via the handle.
    ///
    /// [receiver]: trait.Receiver.html " "
    type Receiver: Receiver<E>;
    /// Returns an unguarded immutable reference to the field.
    fn get_ref(&self) -> &E::Data;
    /// Returns a [`Handle`] to the field.
    ///
    /// [`Handle`]: struct.Handle.html " "
    fn get_handle(&mut self) -> Handle<'_, E, Self::Receiver>;
}

/// A convenience trait for using turbofish syntax to get handles to fields in config tables.
///
/// Using only [`Get`], getting handles to fields is inconvenient when there is no inference to help you, forcing you to use fully qualified trait call syntax. With `GetExt`, this becomes much easier:
/// ```
/// # use snec::{ConfigTable, Entry, Handle, EmptyReceiver};
/// # #[derive(ConfigTable, Default)]
/// # struct MyConfigTable {
/// #     #[snec]
/// #     my_entry: i32,
/// # }
/// use snec::GetExt as _;
/// let mut table = MyConfigTable::default();
/// // Using the Get trait directly:
/// let handle = <MyConfigTable as snec::Get<entries::MyEntry>>::get_handle(&mut table);
/// // Using the GetExt trait:
/// let handle = table.get_handle::<entries::MyEntry>();
/// ```
/// [`Get`]: trait.Get.html " "
pub trait GetExt {
    /// Returns an unguarded immutable reference to the field.
    #[inline(always)]
    fn get_ref<E>(&self) -> &E::Data
    where
        E: Entry,
        Self: Get<E> {
        <Self as Get<E>>::get_ref(self)
    }
    /// Returns a [`Handle`] to the field.
    ///
    /// [`Handle`]: struct.Handle.html " "
    #[inline(always)]
    fn get_handle<E>(&mut self) -> Handle<'_, E, <Self as Get<E>>::Receiver>
    where
        E: Entry,
        Self: Get<E> {
        <Self as Get<E>>::get_handle(self)
    }
}
impl<T: ?Sized> GetExt for T {}