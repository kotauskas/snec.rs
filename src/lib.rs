//! Configuration system with compile-time field lookup and modification notifications.
//!
//! # Overview
//! Snec is a configuration system focused on compile-time guarantees and a way of notifying a running system that a configurable value changed. Most of its power is implemented via macros, which is why those are exported by default.
//!
//! While no built-in serialization support is provided, the architecture by itself is serialization-agnostic — using Serde and Snec for the same config table structure will work just fine.
//!
//! Snec's architecture consists of those key components:
//! - **Config table** — the structure which contains the configuration data for the program. Config tables implement the `Get` trait to access its fields, which allows them to hand out `Handle`s to its fields. Handles ensure that the assigned receiver gets notified when the field changes, unless it's explicitly prompted to perform a silent modification.
//! - **Entry** — an uninhabited type (type with no possible values) implementing the `Entry` trait, representing an identifier for a field inside of a config table.
//! - **Receiver** — type implementing the `Receiver` trait which will receive notifications whenever a entry in a config table it's interested in is modified.
//!
//! # Basic example
//! ```
//! use snec::{ConfigTable, Entry, GetExt as _};
//! use std::time::{SystemTime, Duration};
//! #[derive(ConfigTable)]
//! struct MyConfigTable {
//!     #[snec]
//!     when: SystemTime,
//!     #[snec]
//!     who: String,
//!     #[snec]
//!     in_which_country: String,
//! }
//! let mut config_table = MyConfigTable {
//!     when: SystemTime::UNIX_EPOCH + Duration::from_secs(566_200_800),
//!     who: "Jeremy".to_string(),
//!     in_which_country: "USA".to_string(),
//! };
//!
//! // To access the fields of our config table, we need to use the get_handle method from
//! // the GetExt trait (which is a nicer way to use the Get trait). The `entries` part is
//! // a module generated by the `#[derive(ConfigTable)]`. In most cases, it's desirable
//! // to reexport the contents of the module in a public module with a different name and
//! // some documentation, or simply in the containing module if you want the entry
//! // identifiers to be in the same module as the config table.
//! let mut handle = config_table.get_handle_to::<entries::InWhichCountry>();
//! // After we got the handle, we can use it to get a
//! // mutable reference to the field and modify it:
//! {
//!     let mut in_which_country = handle.modify();
//!     *in_which_country = "Britain".to_string();
//! }
//! // The reason why we put that in a scope and why we had to do this entire two-step process
//! // is because otherwise we'd implicitly avoid notifying any receivers, which is something
//! // that we'll look into in the next example. Since we don't have any, it won't really
//! // hurt if we did this as well:
//! {
//!     let in_which_country = handle.modify_silently();
//!     *in_which_country = "Australia".to_string();
//! }
//! ```
//! Using receivers:
//! ```
//! use snec::{ConfigTable, Receiver, Entry, GetExt as _};
//! use std::time::{SystemTime, Duration};
//! #[derive(ConfigTable)]
//! #[snec(
//!     // Any expression can be used in the braces. After the colon, the type is supplied.
//!     receiver({MyReceiver}: MyReceiver)
//! )]
//! struct MyConfigTable {
//!     #[snec]
//!     which_year: i64,
//!     #[snec(entry, receiver({snec::EmptyReceiver}: snec::EmptyReceiver))]
//!     why: String,
//!     #[snec]
//!     random_integer_that_i_like: u128,
//! }
//!
//! struct MyReceiver;
//! impl Receiver<entries::RandomIntegerThatILike> for MyReceiver {
//!     fn receive(&mut self, new_value: &u128) {
//!         println!("My integer has been changed to {}!!", new_value)
//!     }
//! }
//! impl Receiver<entries::WhichYear> for MyReceiver {
//!     fn receive(&mut self, new_value: &i64) {
//!         println!("Resceduled to {}", new_value)
//!     }
//! }
//!
//! let mut config_table = MyConfigTable {
//!     which_year: 1987,
//!     why: "Accident".to_string(),
//!     random_integer_that_i_like: 687_800,
//! };
//! // Now we have receivers which will immediately react to any changes in the values:
//! let mut handle = config_table.get_handle_to::<entries::WhichYear>();
//! {
//!     let mut which_year = handle.modify();
//!     *which_year = 1983;
//! }
//! // When the scope ends, the `which_year` guard is dropped and the receiver is informed.
//! ```

#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

mod entry;
mod handle;
mod receiver;
pub use entry::*;
pub use handle::*;
pub use receiver::*;

#[cfg(feature = "macros")]
pub extern crate snec_macros as macros;
#[doc(inline)]
pub use macros::*;

// To make derive macros work when called from inside of Snec itself.
extern crate self as snec;