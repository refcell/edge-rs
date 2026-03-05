//! Locations
//!
//! The locations module contains data location annotations for the Edge Language.
//!
//! ## Overview
//!
//! Data locations can be grouped into two broad categories, buffers and maps.
//!
//! The various types of data locations are described on the [Location] enum.
//!
//! ## Transitions
//!
//! Transitioning from map to memory buffer is performed by loading each element
//! from the map to the stack and storing each stack item in memory O(N).
//!
//! Transitioning from memory buffer to a map is performed by loading each element
//! from memory to the stack and storing each stack item in the map O(N).
//!
//! Transitioning from any other buffer to a map is performed by copying the
//! buffer's data into memory then transitioning the data from memory into the map
//! O(N+1).
//!
//! ## Pointer Bit Sizes
//!
//! Pointers to different data locations consist of different sizes based on the
//! properties of that data location. In depth semantics of each data location are
//! specified in the type system documents.
//!
//! | Location          | Size (bits) | Description
//! |-------------------|-------------|--------------------------------
//! | Persistent Storage| 256         | Storage is 256 bit key value hashmap
//! | Transient Storage | 256         | Transient storage is 256 bit key value hashmap
//! | Memory            | 32          | Theoretical maximum memory size does not grow to 0xffffffff
//! | Calldata          | 32          | Theoretical maximum calldata size does not grow to 0xffffffff
//! | Returndata        | 32          | Maximum returndata size is equal to maximum memory size
//! | Internal Code     | 16          | Code size is less than 0xffff
//! | External Code     | 176         | Contains 160 bit address and 16 bit code pointer

use derive_more::Display;

/// Data Location
///
/// The [Location] is a data location annotation indicating to which data
/// location a pointer's data exists. We define seven distinct annotations
/// for data location pointers. This is a divergence from general purpose
/// programming languages to more accurately represent the EVM execution
/// environment.
#[derive(Debug, Clone, PartialOrd, PartialEq, Eq, Hash, Display)]
pub enum Location {
    /// Persistent Storage
    ///
    /// Part of the map category, 256 bit keys map to 256 bit values.
    /// May be written or read one word at a time.
    #[display("&s")]
    PersistentStorage,
    /// Transient Storage
    ///
    /// Part of the map category, 256 bit keys map to 256 bit values.
    /// May be written or read one word at a time.
    #[display("&t")]
    TransientStorage,
    /// Memory
    ///
    /// A linear data buffer.
    /// May be read to the stack, copied to memory, and written to.
    #[display("&m")]
    Memory,
    /// Calldata
    ///
    /// A linear data buffer.
    /// May be read to the stack and copied to memory.
    #[display("&cd")]
    Calldata,
    /// Returndata
    ///
    /// A linear data buffer.
    /// May only be copied to memory.
    #[display("&rd")]
    Returndata,
    /// Internal (local) code
    ///
    /// A linear data buffer.
    /// May only be copied to memory.
    #[display("&ic")]
    InternalCode,
    /// External code
    ///
    /// A linear data buffer.
    /// May only be copied to memory.
    #[display("&ec")]
    ExternalCode,
}
