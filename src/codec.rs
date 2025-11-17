// Copyright 2025
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use zerocopy::{FromBytes, Immutable, IntoBytes};

#[derive(Debug, PartialEq)]
pub enum PldmCodecError {
    BufferTooShort,
    Unsupported,
    InvalidData,
}

/// A trait for encoding and decoding PLDM (Platform Level Data Model) messages.
///
/// This trait provides methods for encoding a PLDM message into a byte buffer
/// and decoding a PLDM message from a byte buffer. Implementers of this trait
/// must also implement the `Debug` trait and be `Sized`.
pub trait PldmCodec: core::fmt::Debug + Sized {
    /// Encodes the PLDM message into the provided byte buffer.
    ///
    /// # Arguments
    ///
    /// * `buffer` - A mutable reference to a byte slice where the encoded message will be stored.
    ///
    /// # Returns
    ///
    /// A `Result` containing the size of the encoded message on success, or a `PldmCodecError` on failure.
    fn encode(&self, buffer: &mut [u8]) -> Result<usize, PldmCodecError>;

    /// Decodes a PLDM message from the provided byte buffer.
    ///
    /// # Arguments
    ///
    /// * `buffer` - A reference to a byte slice containing the encoded message.
    ///
    /// # Returns
    ///
    /// A `Result` containing the decoded message on success, or a `PldmCodecError` on failure.
    fn decode(buffer: &[u8]) -> Result<Self, PldmCodecError>;
}

/// A trait for encoding and decoding PLDM messages with explicit lifetime requirements.
///
/// This trait is similar to `PldmCodec` but supports types that borrow data from the buffer
/// during decoding, requiring an explicit lifetime parameter.
pub trait PldmCodecWithLifetime<'a>: core::fmt::Debug + Sized {
    /// Encodes the PLDM message into the provided byte buffer.
    ///
    /// # Arguments
    ///
    /// * `buffer` - A mutable reference to a byte slice where the encoded message will be stored.
    ///
    /// # Returns
    ///
    /// A `Result` containing the size of the encoded message on success, or a `PldmCodecError` on failure.
    fn encode(&self, buffer: &mut [u8]) -> Result<usize, PldmCodecError>;

    /// Decodes a PLDM message from the provided byte buffer.
    ///
    /// # Arguments
    ///
    /// * `buffer` - A reference to a byte slice containing the encoded message. The decoded
    ///   type may hold references to this buffer.
    ///
    /// # Returns
    ///
    /// A `Result` containing the decoded message on success, or a `PldmCodecError` on failure.
    fn decode(buffer: &'a [u8]) -> Result<Self, PldmCodecError>;
}

// Default implementation of PldmCodec for types that can leverage zerocopy.
// TODO: can we generalize this to use sub-struct encodes when possible?
// There are structs like PldmFirmwareString that contain variable-length data
// that would need special handling.
impl<T> PldmCodec for T
where
    T: core::fmt::Debug + Sized + FromBytes + IntoBytes + Immutable,
{
    fn encode(&self, buffer: &mut [u8]) -> Result<usize, PldmCodecError> {
        self.write_to_prefix(buffer)
            .map_err(|_| PldmCodecError::BufferTooShort)
            .map(|_| core::mem::size_of::<T>())
    }

    fn decode(buffer: &[u8]) -> Result<Self, PldmCodecError> {
        Ok(Self::read_from_prefix(buffer)
            .map_err(|_| PldmCodecError::BufferTooShort)?
            .0)
    }
}
