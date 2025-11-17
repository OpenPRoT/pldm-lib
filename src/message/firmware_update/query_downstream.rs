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

use crate::codec::{PldmCodec, PldmCodecError, PldmCodecWithLifetime};
use crate::error::PldmError;
use crate::protocol::base::{
    InstanceId, PLDM_MSG_HEADER_LEN, PldmBaseCompletionCode, PldmMsgHeader, PldmMsgType,
    PldmSupportedType, TransferOperationFlag,
};

use crate::pldm_completion_code;

use crate::protocol::firmware_update::{
    ComponentActivationMethods, Descriptor, FirmwareDeviceCapability, FwUpdateCmd,
    FwUpdateCompletionCode, PldmFirmwareString,
};
use bitfield::bitfield;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, TryFromBytes};

/// QueryDownstreamDevices is used by the UA to obtain the firmware identifiers
/// for the downstream devices supported by the FDP. The entire list of all
/// attached downstream devices is provided by the response to
/// [QueryDownstreamIdentifiers] command. The FDP shall provide a response
/// message to this command in all states, including IDLE.
#[derive(Debug, Clone, FromBytes, IntoBytes, Immutable, PartialEq)]
#[repr(C, packed)]
pub struct QueryDownstreamDevicesRequest {
    pub hdr: PldmMsgHeader<[u8; PLDM_MSG_HEADER_LEN]>,
}

impl QueryDownstreamDevicesRequest {
    pub fn new(instance_id: InstanceId) -> Self {
        QueryDownstreamDevicesRequest {
            hdr: PldmMsgHeader::new(
                instance_id,
                PldmMsgType::Request,
                PldmSupportedType::FwUpdate,
                FwUpdateCmd::QueryDownstreamDevices as u8,
            ),
        }
    }
}

bitfield! {
    #[derive(Clone, Copy, FromBytes, IntoBytes, Immutable, PartialEq, Eq)]
    pub struct QueryDownstreamDevicesCapability(u32);
    impl Debug;
    pub u32, reserved, _: 31, 3;
    pub u32, update_simultaneous, set_update_simultaneous: 2;
    pub u32, dynamic_remove, set_dynamic_remove: 1;
    pub u32, dynamic_attach, set_dynamic_attach: 0;
}

#[derive(Debug, Clone, FromBytes, IntoBytes, Immutable, PartialEq)]
#[repr(C, packed)]
pub struct QueryDownstreamDeviceResponse {
    pub hdr: PldmMsgHeader<[u8; PLDM_MSG_HEADER_LEN]>,

    /// PLDM_BASE_CODES
    pub completion_code: u8,
    pub downstream_device_update_supported: u8,
    pub number_of_downstream_devices: u16,
    pub max_number_of_downstream_devices: u16,
    pub capabilities: QueryDownstreamDevicesCapability,
}

impl QueryDownstreamDeviceResponse {
    pub fn new(
        instance_id: InstanceId,
        completion_code: PldmBaseCompletionCode,
        downstream_device_update_supported: u8,
        number_of_downstream_devices: u16,
        maximum_number_of_downstream_devices: u16,
    ) -> Self {
        QueryDownstreamDeviceResponse {
            hdr: PldmMsgHeader::new(
                instance_id,
                PldmMsgType::Response,
                PldmSupportedType::FwUpdate,
                FwUpdateCmd::QueryDownstreamDevices as u8,
            ),
            completion_code: completion_code as u8,
            downstream_device_update_supported,
            number_of_downstream_devices,
            max_number_of_downstream_devices: maximum_number_of_downstream_devices,
            capabilities: QueryDownstreamDevicesCapability(0),
        }
    }
}

#[derive(Debug, Clone, FromBytes, IntoBytes, Immutable, PartialEq)]
#[repr(C, packed)]
pub struct QueryDownstreamIdentifiersRequest {
    pub hdr: PldmMsgHeader<[u8; PLDM_MSG_HEADER_LEN]>,
    pub downstream_data_device_handle: u32,
    pub transfer_op_flag: u8,
}

impl QueryDownstreamIdentifiersRequest {
    pub fn new(
        instance_id: InstanceId,
        downstream_data_device_handle: u32,
        transfer_op_flag: TransferOperationFlag,
    ) -> Self {
        QueryDownstreamIdentifiersRequest {
            hdr: PldmMsgHeader::new(
                instance_id,
                PldmMsgType::Request,
                PldmSupportedType::FwUpdate,
                FwUpdateCmd::QueryDownstreamIdentifiers as u8,
            ),
            downstream_data_device_handle,
            transfer_op_flag: transfer_op_flag as u8,
        }
    }
}

// instead of using heapless::Vec, let's just allocate a fixed-size slices for now
pub const DOWNSTREAM_DEVICE_PORTION_COUNT: usize = 4;
pub const DOWNSTREAM_DEVICE_COUNT: usize = 8;
pub const DOWNSTREAM_DESCRIPTOR_COUNT: usize = 4;

pldm_completion_code! {
    QueryDownstreamIdentifiersResponseCode {
        InvalidTransferHandle,
        InvalidTransferOperationFlag,
        DownstreamDeviceListChanged,
    }
}

#[derive(Debug, Clone, PartialEq)]
#[repr(C)]
/// The total structure for QueryDownstreamIdentifiersResponse looks as follows:
/// ```text
/// QueryDownstreamIdentifiersResponse
///  completion_code (u8)
///  next_data_transfer_handle (u32)
///  transfer_flag (u8)
///  --- portion (variable-length)
///    downstream_devices_length_i (u32)
///    number_of_downstream_devices_i (u16)
///    downstream_devices_index_i (u16)
///    ---
///      downstream_device_index_ij (u16)
///      downstream_descriptor_count_ij (u8)
///      ---
///        descriptor_type_ijk (u16)
///        descriptor_length_ijk (u16)
///        descriptor_data_ijk (variable-length L_ijk)
///           ...
///        descriptor_type_ij(k+1) (u16)
///        descriptor_length_ij(k+1) (u16)
///        descriptor_data_ij(k+1) (variable-length L_ij(k+1))
///           ...
///     ...
///     downstream_devices_index_i(j+1) (u16)
///     downstream_device_count_i(j+1) (u8)
///     ---
///       descriptor_type_(i(j+1)k) (u16)
///       descriptor_length_(i(j+1)k) (u16)
///       descriptor_data_(i(j+1)k) (variable-length L_(i(j+1)k))
///         ...
///      ...
///    downstream_devices_length_(i+1) (u32)
///    number_of_downstream_devices_(i+1) (u16)
///    downstream_devices_index_(i+1) (u16)
///    ...
/// ...
/// ```
pub struct QueryDownstreamIdentifiersResponse<'a> {
    pub hdr: PldmMsgHeader<[u8; PLDM_MSG_HEADER_LEN]>,

    /// PLDM_BASE_CODES, INVALID_TRANSFER_HANDLE, INVALID_TRANSFER_OPERATION_FLAG,
    /// DOWNSTREAM_DEVICE_LIST_CHANGED
    ///
    /// See [QueryDownstreamIdentifiersResponseCode].
    pub completion_code: u8,
    pub next_data_transfer_handle: u32,
    pub transfer_flag: u8,

    /// QueryDownstreamIdentifiersResponsePortion
    ///
    /// If the FDP has negotiated a PartSize as defined by DSP0240 and its NegotiateTransferParameters
    /// command, then the maximum size for this field shall be equal to or less than that negotiated value.
    /// Otherwise the FDP can determine the size for this field.
    // TODO: check for PartSize and make this dynamic, for now make it static
    // pub portions: &'a QueryDownstreamIdentifiersPortion<'a>,
    pub portion: &'a [u8],
    portion_iter_current: usize,
    portion_offset_next: usize,
}

impl<'a> QueryDownstreamIdentifiersResponse<'a> {
    pub fn new(
        instance_id: InstanceId,
        completion_code: QueryDownstreamIdentifiersResponseCode,
        next_data_transfer_handle: u32,
        transfer_flag: u8,
        portion: &'a [u8],
    ) -> Self {
        QueryDownstreamIdentifiersResponse {
            hdr: PldmMsgHeader::new(
                instance_id,
                PldmMsgType::Response,
                PldmSupportedType::FwUpdate,
                FwUpdateCmd::QueryDownstreamIdentifiers as u8,
            ),
            completion_code: completion_code.into(),
            next_data_transfer_handle,
            transfer_flag,
            portion,
            portion_iter_current: 0,
            portion_offset_next: 0,
        }
    }
}

#[derive(Debug, TryFromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(C, packed)]
struct PortionHeader {
    pub downstream_devices_length: u32,
    pub number_of_downstream_devices: u16,
}

#[derive(Debug, TryFromBytes, IntoBytes, Immutable, KnownLayout, PartialEq, Eq)]
#[repr(C, packed)]
struct DownstreamDevicesHeader {
    pub downstream_devices_index: u16,
    pub downstream_descriptor_count: u8,
}

// Idea: use get, at(i), try_at(i) and iterator to get elements dynamically from portions buffer
impl QueryDownstreamIdentifiersResponse<'_> {
    /// Parse the portions slice and get the header information.
    ///
    /// Returns an error of the slice is too small and the parsing failed.
    fn try_get_portion_header(&mut self) -> Result<PortionHeader, PldmError> {
        Ok(PortionHeader::try_read_from_prefix(self.portion)
            .map_err(|_| PldmError::InvalidData)?
            .0)
    }

    /// Try to get a [DownstreamDevicesHeader] from the given slice index.
    ///
    /// Returns an error if the slice is too small or the parsing failed.
    fn try_get_downstream_device_header(
        &self,
        buf: &[u8],
    ) -> Result<DownstreamDevicesHeader, PldmError> {
        Ok(DownstreamDevicesHeader::try_read_from_prefix(buf)
            .map_err(|_| PldmError::InvalidData)?
            .0)
    }

    /// Try to get a [DownstreamDevice] at a given index in the portion.
    ///
    /// If the index is out of range, return an [PldmError].
    pub fn try_at(&self, index: usize) -> Result<DownstreamDevice<'_>, PldmError> {
        for (device_index, device) in self.clone().enumerate() {
            if device_index == index {
                return Ok(device);
            }
        }
        Err(PldmError::InvalidData)
    }
}

impl<'a> Iterator for QueryDownstreamIdentifiersResponse<'a> {
    type Item = DownstreamDevice<'a>;

    /// Iterate over all available [DownstreamDevice] in the response portion.
    fn next(&mut self) -> Option<Self::Item> {
        let portion_hdr = self.try_get_portion_header().ok()?;
        if self.portion_iter_current >= portion_hdr.number_of_downstream_devices as usize {
            // at the end, let's reset for next iteration
            self.portion_iter_current = 0;
            self.portion_offset_next = 0;
            return None;
        }

        let offset = if self.portion_iter_current == 0 {
            size_of::<PortionHeader>()
        } else {
            self.portion_offset_next
        };
        if offset >= self.portion.len() {
            return None;
        }

        let hdr: DownstreamDevicesHeader = self
            .try_get_downstream_device_header(&self.portion[offset..])
            .ok()?;
        let device_header_size = size_of::<DownstreamDevicesHeader>();

        let desc_size = Descriptor::try_get_descriptor_length_from_blob(
            &self.portion[offset + device_header_size..],
            hdr.downstream_descriptor_count as usize,
        )
        .ok()?;

        let next_offset = offset + device_header_size + desc_size;
        if next_offset > self.portion.len() {
            self.portion_iter_current = 0;
            self.portion_offset_next = 0;
            return None;
        }

        let descriptor_start = offset + device_header_size;
        let descriptor_end = next_offset;

        self.portion_offset_next = next_offset;
        self.portion_iter_current += 1;

        let device = DownstreamDevice {
            downstream_device_index: hdr.downstream_devices_index,
            downstream_descriptor_count: hdr.downstream_descriptor_count,
            downstream_descriptors: &self.portion[descriptor_start..descriptor_end],
            _iter_dev_count: 0,
            _iter_offset: 0,
        };
        Some(device)
    }
}

impl<'a> PldmCodecWithLifetime<'a> for QueryDownstreamIdentifiersResponse<'a> {
    fn encode(&self, buffer: &mut [u8]) -> Result<usize, crate::codec::PldmCodecError> {
        let size = size_of::<PldmMsgHeader<[u8; PLDM_MSG_HEADER_LEN]>>()
            + size_of::<u8>()
            + size_of::<u32>()
            + size_of::<u8>()
            + self.portion.len();

        if buffer.len() < size {
            return Err(crate::codec::PldmCodecError::BufferTooShort);
        }

        let mut offset = 0;
        self.hdr
            .write_to(&mut buffer[offset..offset + PLDM_MSG_HEADER_LEN])
            .map_err(|_| crate::codec::PldmCodecError::BufferTooShort)?;
        offset += PLDM_MSG_HEADER_LEN;

        buffer[offset] = self.completion_code;
        offset += size_of::<u8>();
        buffer[offset..offset + size_of::<u32>()]
            .copy_from_slice(&self.next_data_transfer_handle.to_le_bytes());

        offset += size_of::<u32>();
        buffer[offset] = self.transfer_flag;

        offset += size_of::<u8>();
        buffer[offset..offset + self.portion.len()].copy_from_slice(self.portion);

        Ok(offset + self.portion.len())
    }

    fn decode(buffer: &'a [u8]) -> Result<Self, crate::codec::PldmCodecError> {
        let min_size = size_of::<PldmMsgHeader<[u8; PLDM_MSG_HEADER_LEN]>>()
            + size_of::<u8>()
            + size_of::<u32>()
            + size_of::<u8>();

        if buffer.len() < min_size {
            return Err(crate::codec::PldmCodecError::BufferTooShort);
        }

        let mut offset = 0;
        let hdr = PldmMsgHeader::<[u8; PLDM_MSG_HEADER_LEN]>::read_from_bytes(
            &buffer[offset..offset + PLDM_MSG_HEADER_LEN],
        )
        .map_err(|_| crate::codec::PldmCodecError::BufferTooShort)?;
        offset += PLDM_MSG_HEADER_LEN;

        let completion_code = buffer[offset];
        offset += size_of::<u8>();

        let next_data_transfer_handle = u32::from_le_bytes(
            buffer[offset..offset + size_of::<u32>()]
                .try_into()
                .map_err(|_| crate::codec::PldmCodecError::BufferTooShort)?,
        );
        offset += size_of::<u32>();

        let transfer_flag = buffer[offset];
        offset += size_of::<u8>();

        let portion = &buffer[offset..];

        Ok(QueryDownstreamIdentifiersResponse {
            hdr,
            completion_code,
            next_data_transfer_handle,
            transfer_flag,
            portion,
            portion_offset_next: 0,
            portion_iter_current: 0,
        })
    }
}

#[derive(Debug, Clone, FromBytes, IntoBytes, Immutable, PartialEq)]
pub struct DownstreamDeviceIndex(u16);

#[allow(unused)]
impl DownstreamDeviceIndex {
    /// 0x0000 – 0x0FFF = Downstream index number
    const FIRST_RESERVED: DownstreamDeviceIndex = DownstreamDeviceIndex(0x1000);

    /// 0x1000 – 0xFFFF = Reserved
    const LAST_RESERVED: DownstreamDeviceIndex = DownstreamDeviceIndex(0xFFFF);
}

impl TryFrom<u16> for DownstreamDeviceIndex {
    type Error = ();

    /// Create a valid DownstreamDeviceIndex from a u16 value.
    /// See [DownstreamDeviceIndex] with FIRST_RESERVED and LAST_RESERVED.
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        if value < Self::FIRST_RESERVED.0 {
            Ok(DownstreamDeviceIndex(value))
        } else {
            Err(())
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, FromBytes)]
#[repr(C)]
pub struct DownstreamDevice<'a> {
    pub downstream_device_index: u16,
    pub downstream_descriptor_count: u8,
    pub downstream_descriptors: &'a [u8],

    // Iterator state, ignore for parsing!!
    _iter_dev_count: usize,
    _iter_offset: usize,
}

impl<'a> DownstreamDevice<'a> {
    pub fn new(
        downstream_device_index: DownstreamDeviceIndex,
        downstream_descriptors: &'a [u8],
    ) -> Self {
        DownstreamDevice {
            downstream_device_index: downstream_device_index.0,
            downstream_descriptor_count: downstream_descriptors.len() as u8,
            downstream_descriptors,
            _iter_dev_count: 0,
            _iter_offset: 0,
        }
    }
}

impl Iterator for DownstreamDevice<'_> {
    type Item = Descriptor;

    fn next(&mut self) -> Option<Self::Item> {
        if self.downstream_descriptor_count == 0 {
            return None;
        }

        if self._iter_dev_count >= self.downstream_descriptor_count as usize {
            self._iter_dev_count = 0;
            self._iter_offset = 0;
            return None;
        }

        // Read the descriptor length while skipping the type field
        let descriptor_length = u16::read_from_bytes(
            &self.downstream_descriptors
                [self._iter_offset + size_of::<u16>()..self._iter_offset + size_of::<u16>() * 2],
        )
        .ok()? as usize;

        // check bounds
        if self._iter_offset + size_of::<u16>() * 2 + descriptor_length
            > self.downstream_descriptors.len()
        {
            return None;
        }

        let descriptor = Descriptor::decode(
            &self.downstream_descriptors
                [self._iter_offset..self._iter_offset + 2 * size_of::<u16>() + descriptor_length],
        )
        .ok()?;

        self._iter_offset += size_of::<u16>() * 2 + descriptor_length;
        self._iter_dev_count += 1;

        Some(descriptor)
    }
}

impl<'a> PldmCodecWithLifetime<'a> for DownstreamDevice<'a> {
    fn encode(&self, buffer: &mut [u8]) -> Result<usize, crate::codec::PldmCodecError> {
        let mut offset = 0;
        let size = size_of::<DownstreamDeviceIndex>()
            + size_of::<u8>()
            + self
                .downstream_descriptors
                .iter()
                .map(|d| d.encode(&mut []).unwrap_or(0))
                .sum::<usize>();

        if buffer.len() < size {
            return Err(crate::codec::PldmCodecError::BufferTooShort);
        }

        buffer[offset..offset + size_of::<DownstreamDeviceIndex>()]
            .copy_from_slice(&self.downstream_device_index.to_le_bytes());
        offset += size_of::<DownstreamDeviceIndex>();

        buffer[offset] = self.downstream_descriptor_count;
        offset += 1;

        for descriptor in self.downstream_descriptors.iter() {
            let bytes_written = descriptor.encode(&mut buffer[offset..])?;
            offset += bytes_written;
        }
        Ok(offset)
    }

    fn decode(buffer: &'a [u8]) -> Result<Self, crate::codec::PldmCodecError> {
        // min size: DownstreamDeviceIndex + descriptor count(0)
        let min_size = size_of::<DownstreamDeviceIndex>() + size_of::<u8>();
        let mut offset = 0;

        if buffer.len() < min_size {
            return Err(crate::codec::PldmCodecError::BufferTooShort);
        }

        let downstream_device_index = u16::from_le_bytes(
            buffer[offset..offset + size_of::<u16>()]
                .try_into()
                .map_err(|_| crate::codec::PldmCodecError::BufferTooShort)?,
        );
        offset += size_of::<u16>();

        let downstream_descriptor_count = buffer[offset];
        offset += 1;

        Ok(Self {
            downstream_device_index,
            downstream_descriptor_count,
            downstream_descriptors: &buffer[offset..],
            _iter_dev_count: 0,
            _iter_offset: 0,
        })
    }
}

#[derive(Debug, Clone, FromBytes, IntoBytes, Immutable, PartialEq)]
#[repr(C, packed)]
pub struct GetDownstreamFirmwareParametersRequest {
    pub hdr: PldmMsgHeader<[u8; PLDM_MSG_HEADER_LEN]>,
    pub data_transfer_handle: u32,
    pub transfer_op_flag: u8,
}

impl GetDownstreamFirmwareParametersRequest {
    pub fn new(
        instance_id: InstanceId,
        data_transfer_handle: u32,
        transfer_op_flag: TransferOperationFlag,
    ) -> Self {
        GetDownstreamFirmwareParametersRequest {
            hdr: PldmMsgHeader::new(
                instance_id,
                PldmMsgType::Request,
                PldmSupportedType::FwUpdate,
                FwUpdateCmd::GetDownstreamFirmwareParameters as u8,
            ),
            data_transfer_handle,
            transfer_op_flag: transfer_op_flag as u8,
        }
    }
}

pldm_completion_code! {
    GetDownstreamFirmwareParametersResponseCode {
        InvalidTransferHandle,
        InvalidTransferOperationFlag,
        DownstreamDeviceListChanged
    }
}

#[derive(Debug, Clone, PartialEq, FromBytes)]
#[repr(C)]
pub struct GetDownstreamFirmwareParametersResponse {
    pub hdr: PldmMsgHeader<[u8; PLDM_MSG_HEADER_LEN]>,

    /// PLDM_BASE_CODES, INVALID_TRANSFER_HANDLE, INVALID_TRANSFER_OPERATION_FLAG,
    /// DOWNSTREAM_DEVICE_LIST_CHANGED
    ///
    /// See [GetDownstreamFirmwareParametersResponseCode].
    pub completion_code: u8,
    pub next_data_transfer_handle: u32,
    pub transfer_flag: u8,

    /// GetDownstreamFirmwareParametersPortion
    ///
    /// If the FDP has negotiated a PartSize as defined by DSP0240 and its NegotiateTransferParameters
    /// command, then the maximum size for this field shall be equal to or less than that negotiated value.
    /// Otherwise the FDP can determine the size for this field.
    // TODO: check for PartSize and make this dynamic
    pub portion: GetDownstreamFirmwareParametersPortion,
}

// #[derive(Debug, Clone, FromBytes, IntoBytes, Immutable, PartialEq)]
#[derive(Debug, Clone, PartialEq, FromBytes)]
#[repr(C)]
pub struct GetDownstreamFirmwareParametersPortion {
    pub get_downstream_firmware_parameters_capability: FirmwareDeviceCapability,
    pub downstream_device_count: u16,
    pub downstream_device_parameter_table: DownstreamDeviceParameterTable,
}

impl GetDownstreamFirmwareParametersPortion {
    pub fn new(
        capability: FirmwareDeviceCapability,
        downstream_device_count: u16,
        downstream_device_parameter_table: DownstreamDeviceParameterTable,
    ) -> Self {
        GetDownstreamFirmwareParametersPortion {
            get_downstream_firmware_parameters_capability: capability,
            downstream_device_count,
            downstream_device_parameter_table,
        }
    }

    pub fn size(&self) -> usize {
        size_of::<FirmwareDeviceCapability>()
            + size_of::<u16>()
            + self.downstream_device_parameter_table.size()
    }
}

impl PldmCodec for GetDownstreamFirmwareParametersPortion {
    fn encode(&self, buffer: &mut [u8]) -> Result<usize, crate::codec::PldmCodecError> {
        if buffer.len() < self.size() {
            return Err(crate::codec::PldmCodecError::BufferTooShort);
        }

        let mut offset = 0;
        buffer[offset..offset + size_of::<FirmwareDeviceCapability>()].copy_from_slice(
            &self
                .get_downstream_firmware_parameters_capability
                .0
                .to_le_bytes(),
        );
        offset += size_of::<FirmwareDeviceCapability>();

        buffer[offset..offset + size_of::<u16>()]
            .copy_from_slice(&self.downstream_device_count.to_le_bytes());
        offset += size_of::<u16>();

        let bytes_written = self
            .downstream_device_parameter_table
            .encode(&mut buffer[offset..])?;

        Ok(offset + bytes_written)
    }

    fn decode(buffer: &[u8]) -> Result<Self, crate::codec::PldmCodecError> {
        let min_size = size_of::<FirmwareDeviceCapability>() + size_of::<u16>();
        let mut offset = 0;

        if buffer.len() < min_size {
            return Err(crate::codec::PldmCodecError::BufferTooShort);
        }

        let capability = u32::from_le_bytes(
            buffer[offset..offset + size_of::<u32>()]
                .try_into()
                .map_err(|_| crate::codec::PldmCodecError::InvalidData)?,
        );
        offset += size_of::<u32>();

        let downstream_device_count = u16::from_le_bytes(
            buffer[offset..offset + size_of::<u16>()]
                .try_into()
                .map_err(|_| crate::codec::PldmCodecError::InvalidData)?,
        );
        offset += size_of::<u16>();

        let downstream_device_parameter_table =
            DownstreamDeviceParameterTable::decode(&buffer[offset..])?;

        Ok(GetDownstreamFirmwareParametersPortion {
            get_downstream_firmware_parameters_capability: FirmwareDeviceCapability(capability),
            downstream_device_count,
            downstream_device_parameter_table,
        })
    }
}

/// Wrapper struct for PLDM timestamp representation
#[derive(Debug, Clone, Copy, PartialEq, FromBytes, IntoBytes)]
#[repr(C, packed)]
pub struct PldmTimeStamp([u8; 8]);

impl PldmTimeStamp {
    /// Ensure that only valid timestamps are created
    /// The timestamp string must be in the format "YYYYMMDD", where Y is year, M is month, D is day.
    /// These are ASCII characters in range 0x30 to 0x39, which are the ascii digits '0' to '9'.
    pub fn new(timestamp: &str) -> Result<Self, PldmError> {
        if timestamp.len() != 8 {
            return Err(PldmError::InvalidData);
        }

        let ts_bytes: [u8; 8] = timestamp
            .as_bytes()
            .try_into()
            .map_err(|_| PldmError::InvalidData)?;

        ts_bytes.iter().try_for_each(|&b| {
            if !(0x30..=0x39).contains(&b) {
                Err(PldmError::InvalidData)
            } else {
                Ok(())
            }
        })?;
        Ok(PldmTimeStamp(ts_bytes))
    }
}

#[derive(Debug, Clone, PartialEq, FromBytes, IntoBytes)]
#[repr(C, packed)]
/// # Warning
/// This struct is not a 1:1 representation of the PLDM spec,
/// since for simplicity we want to use PldmFirmwareString for the version strings.
/// This decision was made to prioritize usability over strict adherence to the spec.
/// For the original, see [DSP0267](https://www.dmtf.org/sites/default/files/standards/documents/DSP0267_1.2.0WIP99.pdf), Table 21.
///
/// It is up for discussion whether this is the right approach. For now we proceed with this.
pub struct DownstreamDeviceParameterTable {
    pub downstream_device_index: u16,
    pub active_component_comparison_stamp: u32,
    // pub active_component_version_string_type: u8, // See VersionStringType
    // pub active_component_version_string_length: u8,
    pub active_component_release_date: PldmTimeStamp,
    pub pending_component_comparison_stamp: u32,
    // pub pending_component_version_string_type: u8, // See VersionStringType
    // pub pending_component_version_string_length: u8,
    pub pending_component_release_date: PldmTimeStamp,
    pub component_activation_methods: ComponentActivationMethods,
    pub capabilities_during_update: CapabilitiesDuringUpdate,

    /// WARNING: when encoding/decoding this is variable in size.
    pub active_component_version_string: PldmFirmwareString,

    /// "If no pending firmware component exists, this field is zero bytes in length"
    ///
    /// **WARNING**: when encoding/decoding this is variable in size.
    pub pending_component_version_string: PldmFirmwareString,
}

#[allow(clippy::too_many_arguments)]
impl DownstreamDeviceParameterTable {
    pub fn new(
        downstream_device_index: DownstreamDeviceIndex,
        active_component_comparison_stamp: u32,
        active_component_version_string: PldmFirmwareString,
        pending_component_version_string: PldmFirmwareString,
        active_component_release_date: &str,
        pending_component_comparison_stamp: u32,
        component_activation_methods: ComponentActivationMethods,
        capabilities_during_update: CapabilitiesDuringUpdate,
        pending_component_release_date: &str,
    ) -> Result<Self, PldmError> {
        Ok(DownstreamDeviceParameterTable {
            downstream_device_index: downstream_device_index.0,
            active_component_comparison_stamp,
            active_component_release_date: PldmTimeStamp::new(active_component_release_date)?,
            pending_component_comparison_stamp,
            pending_component_release_date: PldmTimeStamp::new(pending_component_release_date)?,
            component_activation_methods,
            capabilities_during_update,
            active_component_version_string,
            pending_component_version_string,
        })
    }

    pub fn size(&self) -> usize {
        size_of::<u16>() // downstream_device_index
            + size_of::<u32>() // active_component_comparison_stamp
            + size_of::<u8>() // ActiveComponentVersionStringType
            + size_of::<u8>() // ActiveComponentVersionStringLength
            + size_of::<PldmTimeStamp>()  // active_component_release_date
            + size_of::<u32>() // pending_component_comparison_stamp
            + size_of::<u8>() // PendingComponentVersionStringType
            + size_of::<u8>() // PendingComponentVersionStringLength
            + size_of::<PldmTimeStamp>() // pending_component_release_date
            + size_of::<ComponentActivationMethods>() // component_activation_methods
            + size_of::<CapabilitiesDuringUpdate>() // capabilities_during_update
            // this is 6 although it should be 4
            + self.active_component_version_string.str_len as usize
            + self.pending_component_version_string.str_len as usize
    }
}

/// This is a custom implementation, since the struct contains variable-length fields.
/// [PldmFirmwareString] needs a custom codec as well, so we cannot derive it here.
impl PldmCodec for DownstreamDeviceParameterTable {
    fn encode(&self, buffer: &mut [u8]) -> Result<usize, crate::codec::PldmCodecError> {
        if buffer.len() < self.size() {
            return Err(crate::codec::PldmCodecError::BufferTooShort);
        }

        let mut offset = 0;
        buffer[offset..offset + size_of::<u16>()]
            .copy_from_slice(&self.downstream_device_index.to_le_bytes());
        offset += size_of::<u16>();

        buffer[offset..offset + size_of::<u32>()]
            .copy_from_slice(&self.active_component_comparison_stamp.to_le_bytes());
        offset += size_of::<u32>();

        buffer[offset] = self.active_component_version_string.str_type;
        offset += 1;

        buffer[offset] = self.active_component_version_string.str_len;
        offset += 1;

        buffer[offset..offset + size_of::<PldmTimeStamp>()]
            .copy_from_slice(&self.active_component_release_date.0);
        offset += size_of::<PldmTimeStamp>();

        buffer[offset..offset + size_of::<u32>()]
            .copy_from_slice(&self.pending_component_comparison_stamp.to_le_bytes());
        offset += size_of::<u32>();

        buffer[offset] = self.pending_component_version_string.str_type;
        offset += 1;

        buffer[offset] = self.pending_component_version_string.str_len;
        offset += 1;

        buffer[offset..offset + size_of::<PldmTimeStamp>()]
            .copy_from_slice(&self.pending_component_release_date.0);
        offset += size_of::<PldmTimeStamp>();

        buffer[offset..offset + size_of::<ComponentActivationMethods>()]
            .copy_from_slice(&self.component_activation_methods.0.to_le_bytes());
        offset += size_of::<ComponentActivationMethods>();

        buffer[offset..offset + size_of::<CapabilitiesDuringUpdate>()]
            .copy_from_slice(&self.capabilities_during_update.0.to_le_bytes());
        offset += size_of::<CapabilitiesDuringUpdate>();

        buffer[offset..offset + self.active_component_version_string.str_len as usize]
            .copy_from_slice(
                &self.active_component_version_string.str_data
                    [..self.active_component_version_string.str_len as usize],
            );
        offset += self.active_component_version_string.str_len as usize;

        buffer[offset..offset + self.pending_component_version_string.str_len as usize]
            .copy_from_slice(
                &self.pending_component_version_string.str_data
                    [..self.pending_component_version_string.str_len as usize],
            );
        Ok(offset + self.pending_component_version_string.str_len as usize)
    }

    fn decode(buffer: &[u8]) -> Result<Self, crate::codec::PldmCodecError> {
        // min size, assumed both version strings are 0 length
        let min_size = size_of::<u16>()
            + size_of::<u32>()
            + size_of::<PldmTimeStamp>()
            + size_of::<u32>()
            + size_of::<PldmTimeStamp>()
            + size_of::<ComponentActivationMethods>()
            + size_of::<CapabilitiesDuringUpdate>();

        if buffer.len() < min_size {
            return Err(crate::codec::PldmCodecError::BufferTooShort);
        }

        let mut offset = 0;
        let downstream_device_index = u16::from_le_bytes(
            buffer[offset..offset + size_of::<u16>()]
                .try_into()
                .map_err(|_| crate::codec::PldmCodecError::InvalidData)?,
        );
        offset += size_of::<u16>();

        let active_component_comparison_stamp = u32::from_le_bytes(
            buffer[offset..offset + size_of::<u32>()]
                .try_into()
                .map_err(|_| crate::codec::PldmCodecError::InvalidData)?,
        );
        offset += size_of::<u32>();

        // in the spec we now have to decode the string information one after another
        // See [DownstreamDeviceParameterTable] warning
        let active_component_version_string_type = buffer[offset];
        offset += size_of::<u8>();

        let active_component_version_string_length = buffer[offset];
        offset += size_of::<u8>();

        let active_component_release_date = PldmTimeStamp::try_read_from_prefix(
            &buffer[offset..offset + size_of::<PldmTimeStamp>()],
        )
        .map_err(|_| PldmCodecError::InvalidData)?
        .0;
        offset += size_of::<PldmTimeStamp>();

        let pending_component_comparison_stamp = u32::from_le_bytes(
            buffer[offset..offset + size_of::<u32>()]
                .try_into()
                .map_err(|_| crate::codec::PldmCodecError::InvalidData)?,
        );
        offset += size_of::<u32>();

        let pending_component_version_string_type = buffer[offset];
        offset += size_of::<u8>();

        let pending_component_version_string_length = buffer[offset];
        offset += size_of::<u8>();

        let pending_component_release_date = PldmTimeStamp::try_read_from_prefix(
            &buffer[offset..offset + size_of::<PldmTimeStamp>()],
        )
        .map_err(|_| PldmCodecError::InvalidData)?
        .0;
        offset += size_of::<PldmTimeStamp>();

        let component_activation_methods = ComponentActivationMethods::decode(&buffer[offset..])?;
        offset += size_of::<ComponentActivationMethods>();

        let capabilities_during_update = CapabilitiesDuringUpdate::decode(&buffer[offset..])?;
        offset += size_of::<CapabilitiesDuringUpdate>();

        let mut active_component_version_string = PldmFirmwareString {
            str_type: active_component_version_string_type,
            str_len: active_component_version_string_length,
            str_data: [0u8; 32],
        };
        active_component_version_string.str_data[..active_component_version_string_length as usize]
            .copy_from_slice(
                &buffer[offset..offset + active_component_version_string_length as usize],
            );
        offset += active_component_version_string_length as usize;

        let mut pending_component_version_string = PldmFirmwareString {
            str_type: pending_component_version_string_type,
            str_len: pending_component_version_string_length,
            str_data: [0u8; 32],
        };
        pending_component_version_string.str_data
            [..pending_component_version_string_length as usize]
            .copy_from_slice(
                &buffer[offset..offset + pending_component_version_string_length as usize],
            );

        Ok(DownstreamDeviceParameterTable {
            downstream_device_index,
            active_component_comparison_stamp,
            active_component_release_date,
            pending_component_comparison_stamp,
            pending_component_release_date,
            component_activation_methods,
            capabilities_during_update,
            active_component_version_string,
            pending_component_version_string,
        })
    }
}

impl GetDownstreamFirmwareParametersResponse {
    pub fn new(
        instance_id: InstanceId,
        completion_code: GetDownstreamFirmwareParametersResponseCode,
        next_data_transfer_handle: u32,
        transfer_flag: u8,
        portion: GetDownstreamFirmwareParametersPortion,
    ) -> Self {
        GetDownstreamFirmwareParametersResponse {
            hdr: PldmMsgHeader::new(
                instance_id,
                PldmMsgType::Response,
                PldmSupportedType::FwUpdate,
                FwUpdateCmd::GetDownstreamFirmwareParameters as u8,
            ),
            completion_code: completion_code.into(),
            next_data_transfer_handle,
            transfer_flag,
            portion,
        }
    }
}

impl PldmCodec for GetDownstreamFirmwareParametersResponse {
    fn encode(&self, buffer: &mut [u8]) -> Result<usize, crate::codec::PldmCodecError> {
        let size = size_of::<PldmMsgHeader<[u8; PLDM_MSG_HEADER_LEN]>>()
            + size_of::<u8>()
            + size_of::<u32>()
            + size_of::<u8>()
            + self.portion.size();

        if buffer.len() < size {
            return Err(crate::codec::PldmCodecError::BufferTooShort);
        }

        let mut offset = 0;
        self.hdr
            .write_to(&mut buffer[offset..offset + PLDM_MSG_HEADER_LEN])
            .map_err(|_| crate::codec::PldmCodecError::BufferTooShort)?;
        offset += PLDM_MSG_HEADER_LEN;

        buffer[offset] = self.completion_code;
        offset += size_of::<u8>();

        buffer[offset..offset + size_of::<u32>()]
            .copy_from_slice(&self.next_data_transfer_handle.to_le_bytes());
        offset += size_of::<u32>();

        buffer[offset] = self.transfer_flag;
        offset += size_of::<u8>();

        let bytes_written = self.portion.encode(&mut buffer[offset..])?;

        Ok(offset + bytes_written)
    }

    fn decode(buffer: &[u8]) -> Result<Self, crate::codec::PldmCodecError> {
        let min_size = size_of::<PldmMsgHeader<[u8; PLDM_MSG_HEADER_LEN]>>()
            + size_of::<u8>()
            + size_of::<u32>()
            + size_of::<u8>()
            + size_of::<FirmwareDeviceCapability>()
            + size_of::<u16>();

        if buffer.len() < min_size {
            return Err(crate::codec::PldmCodecError::BufferTooShort);
        }

        let mut offset = 0;
        let hdr = PldmMsgHeader::<[u8; PLDM_MSG_HEADER_LEN]>::read_from_bytes(
            &buffer[offset..offset + PLDM_MSG_HEADER_LEN],
        )
        .map_err(|_| crate::codec::PldmCodecError::InvalidData)?;
        offset += PLDM_MSG_HEADER_LEN;

        let completion_code = buffer[offset];
        offset += size_of::<u8>();

        let next_data_transfer_handle = u32::from_le_bytes(
            buffer[offset..offset + size_of::<u32>()]
                .try_into()
                .map_err(|_| crate::codec::PldmCodecError::InvalidData)?,
        );
        offset += size_of::<u32>();

        let transfer_flag = buffer[offset];
        offset += size_of::<u8>();

        let portion = GetDownstreamFirmwareParametersPortion::decode(&buffer[offset..])?;
        Ok(GetDownstreamFirmwareParametersResponse {
            hdr,
            completion_code,
            next_data_transfer_handle,
            transfer_flag,
            portion,
        })
    }
}

bitfield! {
    #[derive(Clone, Copy, FromBytes, IntoBytes, Immutable, PartialEq, Eq)]
    pub struct CapabilitiesDuringUpdate(u32);
    impl Debug;
    pub u32, reserved, _: 31, 5;
    pub u32, component_security_level_latest, set_component_security_level_latest: 4;
    pub u32, security_revision_number_updateable, set_security_revision_number_updateable: 3;
    pub u32, component_downgrade_capability, set_component_downgrade_capability: 2;
    pub u32, downstream_updateable, set_downstream_updateable: 1;
    pub u32, downstream_apply_state, set_downstream_apply_state: 0;
}
// DMTF0267 12.17
#[derive(Debug, Clone, FromBytes, IntoBytes, Immutable, PartialEq)]
#[repr(C, packed)]
pub struct RequestDownstreamDeviceUpdateRequest {
    pub hdr: PldmMsgHeader<[u8; PLDM_MSG_HEADER_LEN]>,
    // "This value shall be equal to or greater than firmware update baseline transfer size"
    // See section 7.8
    pub max_downstream_device_transfer_size: u32,
    pub max_outstanding_transfer_requests: u8,
    pub downstream_device_pkg_data_length: u16,
}

impl RequestDownstreamDeviceUpdateRequest {
    pub fn new(
        instance_id: InstanceId,
        max_downstream_device_transfer_size: u32,
        max_outstanding_transfer_requests: u8,
        downstream_device_pkg_data_length: u16,
    ) -> Self {
        RequestDownstreamDeviceUpdateRequest {
            hdr: PldmMsgHeader::new(
                instance_id,
                PldmMsgType::Request,
                PldmSupportedType::FwUpdate,
                FwUpdateCmd::RequestDownstreamDeviceUpdate as u8,
            ),
            max_downstream_device_transfer_size,
            max_outstanding_transfer_requests,
            downstream_device_pkg_data_length,
        }
    }
}

#[derive(Debug, Clone, Immutable, PartialEq)]
pub enum DDWillSendGetPackageDataCommand {
    FDPShouldObtainUALimited = 0x02,
    FDPShouldObtainLearn = 0x01,
    FDPNoSupport = 0x00,
}

impl TryFrom<u8> for DDWillSendGetPackageDataCommand {
    type Error = ();
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x02 => Ok(DDWillSendGetPackageDataCommand::FDPShouldObtainUALimited),
            0x01 => Ok(DDWillSendGetPackageDataCommand::FDPShouldObtainLearn),
            0x00 => Ok(DDWillSendGetPackageDataCommand::FDPNoSupport),
            _ => Err(()),
        }
    }
}

impl From<DDWillSendGetPackageDataCommand> for u8 {
    fn from(cmd: DDWillSendGetPackageDataCommand) -> Self {
        match cmd {
            DDWillSendGetPackageDataCommand::FDPShouldObtainUALimited => 0x02,
            DDWillSendGetPackageDataCommand::FDPShouldObtainLearn => 0x01,
            DDWillSendGetPackageDataCommand::FDPNoSupport => 0x00,
        }
    }
}

pldm_completion_code! {
    RequestDownstreamDeviceUpdateCode {
        AlreadyInUpdateMode,
        UnableToInitiateUpdate,
        RetryRequestUpdate
    }
}

#[derive(Debug, Clone, FromBytes, IntoBytes, Immutable, PartialEq)]
#[repr(C, packed)]
pub struct RequestDownstreamDeviceUpdateResponse {
    pub hdr: PldmMsgHeader<[u8; PLDM_MSG_HEADER_LEN]>,

    /// PLDM_BASE_CODES, ALREADY_IN_UPDATE_MODE, UNABLE_TO_INITIATE_UPDATE,
    /// RETRY_REQUEST_UPDATE
    ///
    /// See [RequestDownstreamDeviceUpdateCode].
    pub completion_code: u8,
    pub downstream_device_metadata_length: u16,
    pub pkg_data_command: u8,
    pub get_pkg_data_max_transfer_size: u16,
}

impl RequestDownstreamDeviceUpdateResponse {
    pub fn new(
        instance_id: InstanceId,
        completion_code: RequestDownstreamDeviceUpdateCode,
        downstream_device_metadata_length: u16,
        pkg_data_command: DDWillSendGetPackageDataCommand,
        get_pkg_data_max_transfer_size: u16,
    ) -> Self {
        RequestDownstreamDeviceUpdateResponse {
            hdr: PldmMsgHeader::new(
                instance_id,
                PldmMsgType::Response,
                PldmSupportedType::FwUpdate,
                FwUpdateCmd::RequestDownstreamDeviceUpdate as u8,
            ),
            completion_code: completion_code.into(),
            downstream_device_metadata_length,
            pkg_data_command: pkg_data_command.into(),
            get_pkg_data_max_transfer_size,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{codec::PldmCodec, protocol::firmware_update::DescriptorType};

    #[test]
    fn test_query_downstream_devices_request_codec() {
        let instance_id: InstanceId = 0x01;
        let req = QueryDownstreamDevicesRequest::new(instance_id);

        let mut buffer_encode = [0u8; core::mem::size_of::<QueryDownstreamDevicesRequest>()];
        req.encode(&mut buffer_encode).unwrap();

        let decoded = QueryDownstreamDevicesRequest::decode(&buffer_encode).unwrap();
        assert_eq!(req, decoded);
    }

    #[test]
    fn test_query_downstream_devices_response_codec() {
        let instance_id: InstanceId = 0x01;
        let resp = QueryDownstreamDeviceResponse::new(
            instance_id,
            PldmBaseCompletionCode::Success,
            1u8,
            1u16,
            1u16,
        );

        let mut buffer_encode = [0u8; core::mem::size_of::<QueryDownstreamDeviceResponse>()];
        resp.encode(&mut buffer_encode).unwrap();

        let decoded = QueryDownstreamDeviceResponse::decode(&buffer_encode).unwrap();
        assert_eq!(resp, decoded);
    }

    #[test]
    fn test_query_downstream_identifiers_request_codec() {
        let instance_id: InstanceId = 0x01;
        let downstream_data_device_handle = 0x12345678;
        let transfer_op_flag = TransferOperationFlag::GetFirstPart;

        let req = QueryDownstreamIdentifiersRequest::new(
            instance_id,
            downstream_data_device_handle,
            transfer_op_flag,
        );

        let mut buffer_encode = [0u8; core::mem::size_of::<QueryDownstreamIdentifiersRequest>()];
        req.encode(&mut buffer_encode).unwrap();

        let decoded = QueryDownstreamIdentifiersRequest::decode(&buffer_encode).unwrap();
        assert_eq!(req, decoded);
    }

    #[test]
    fn test_query_downstream_identifiers_response_codec() {
        let instance_id: InstanceId = 0x01;
        let resp = QueryDownstreamIdentifiersResponse::new(
            instance_id,
            QueryDownstreamIdentifiersResponseCode::BaseCodes(PldmBaseCompletionCode::Success),
            0x12345678,
            TransferOperationFlag::GetFirstPart as u8,
            &[0u8; 16],
        );

        let mut buffer_encode = [0u8; 128];
        let bytes_written = resp.encode(&mut buffer_encode).unwrap();
        let decoded =
            QueryDownstreamIdentifiersResponse::decode(&buffer_encode[..bytes_written]).unwrap();
        assert_eq!(resp, decoded);
    }

    #[test]
    fn test_get_downstream_firmware_parameters_request_codec() {
        let instance_id: InstanceId = 0x01;
        let req = GetDownstreamFirmwareParametersRequest::new(
            instance_id,
            0x12345678,
            TransferOperationFlag::GetFirstPart,
        );

        let mut buffer_encode =
            [0u8; core::mem::size_of::<GetDownstreamFirmwareParametersRequest>()];
        req.encode(&mut buffer_encode).unwrap();

        let decoded = GetDownstreamFirmwareParametersRequest::decode(&buffer_encode).unwrap();
        assert_eq!(req, decoded);
    }

    #[test]
    fn test_device_parameter_table_codec() {
        let downstream_device_index = DownstreamDeviceIndex::try_from(1).unwrap();
        let acvs = PldmFirmwareString::new("ascii", "test").unwrap();
        let pcvs = PldmFirmwareString::new("ascii", "test").unwrap();

        let table = DownstreamDeviceParameterTable::new(
            downstream_device_index,
            12345,
            acvs,
            pcvs,
            "20250101",
            12346,
            ComponentActivationMethods(0),
            CapabilitiesDuringUpdate(0),
            "20250202",
        )
        .unwrap();

        const STR_DATA_OFFSET: usize = size_of::<u16>()
                + size_of::<u32>()
                + 2 * size_of::<u8> ()// string meta data
                + size_of::<PldmTimeStamp>()
                + size_of::<u32>()
                + 2 * size_of::<u8>()// string meta data
                + size_of::<PldmTimeStamp>()
                + size_of::<ComponentActivationMethods>()
                + size_of::<CapabilitiesDuringUpdate>();

        let mut buffer = [0u8; STR_DATA_OFFSET + 4 + 4];
        let bytes_written = table.encode(&mut buffer).unwrap();

        // check if the strings were encoded correctly
        let mut offset = STR_DATA_OFFSET;
        assert_eq!(&buffer[offset..offset + 4], b"test");
        offset += 4;

        assert_eq!(&buffer[offset..offset + 4], b"test");

        let decoded = DownstreamDeviceParameterTable::decode(&buffer[..bytes_written]).unwrap();
        assert_eq!(table, decoded);
    }

    #[test]
    fn test_get_downstream_firmware_parameters_portion_codec() {
        let downstream_device_index = DownstreamDeviceIndex::try_from(1).unwrap();
        let acvs = PldmFirmwareString::new("ascii", "test").unwrap();
        let pcvs = PldmFirmwareString::new("ascii", "test").unwrap();

        let downstream_device_parameter_table = DownstreamDeviceParameterTable::new(
            downstream_device_index,
            12345,
            acvs,
            pcvs,
            "20250101",
            12346,
            ComponentActivationMethods(0),
            CapabilitiesDuringUpdate(0),
            "20250202",
        )
        .unwrap();

        let portion = GetDownstreamFirmwareParametersPortion {
            get_downstream_firmware_parameters_capability: FirmwareDeviceCapability(0u32),
            downstream_device_count: 1,
            downstream_device_parameter_table,
        };

        let mut buffer = [0u8; 0xff0];
        let bytes_written = portion.encode(&mut buffer).unwrap();

        let decoded =
            GetDownstreamFirmwareParametersPortion::decode(&buffer[..bytes_written]).unwrap();
        assert_eq!(portion, decoded);
    }

    #[test]
    fn test_get_downstream_firmware_parameters_response_codec() {
        let instance_id: InstanceId = 0x01;
        let downstream_device_index = DownstreamDeviceIndex::try_from(1).unwrap();
        let cap: FirmwareDeviceCapability = FirmwareDeviceCapability(0u32);

        let acvs = PldmFirmwareString::new("ascii", "test").unwrap();
        let pcvs = PldmFirmwareString::new("ascii", "test").unwrap();

        let downstream_device_parameter_table = DownstreamDeviceParameterTable::new(
            downstream_device_index,
            12345,
            acvs,
            pcvs,
            "20250101",
            12346,
            ComponentActivationMethods(0),
            CapabilitiesDuringUpdate(0),
            "20250202",
        )
        .unwrap();

        let portion = GetDownstreamFirmwareParametersPortion {
            get_downstream_firmware_parameters_capability: cap,
            downstream_device_count: 1,
            downstream_device_parameter_table,
        };

        let resp = GetDownstreamFirmwareParametersResponse::new(
            instance_id,
            GetDownstreamFirmwareParametersResponseCode::BaseCodes(PldmBaseCompletionCode::Success),
            0x12345678,
            TransferOperationFlag::GetFirstPart as u8,
            portion,
        );

        let mut buffer_encode = [0u8; 0xff0];
        let bytes_written = resp.encode(&mut buffer_encode).unwrap();

        let decoded =
            GetDownstreamFirmwareParametersResponse::decode(&buffer_encode[..bytes_written])
                .unwrap();
        assert_eq!(resp, decoded);
    }

    #[test]
    fn test_request_downstream_device_update_request_codec() {
        let instance_id: InstanceId = 0x01;
        let req = RequestDownstreamDeviceUpdateRequest::new(instance_id, 1024, 4, 512);

        let mut buffer_encode = [0u8; core::mem::size_of::<RequestDownstreamDeviceUpdateRequest>()];
        req.encode(&mut buffer_encode).unwrap();

        let decoded = RequestDownstreamDeviceUpdateRequest::decode(&buffer_encode).unwrap();
        assert_eq!(req, decoded);
    }

    #[test]
    fn test_request_downstream_device_update_response_codec() {
        let instance_id: InstanceId = 0x01;
        let resp = RequestDownstreamDeviceUpdateResponse::new(
            instance_id,
            RequestDownstreamDeviceUpdateCode::BaseCodes(PldmBaseCompletionCode::Success),
            256,
            DDWillSendGetPackageDataCommand::FDPShouldObtainLearn,
            512,
        );

        let mut buffer_encode =
            [0u8; core::mem::size_of::<RequestDownstreamDeviceUpdateResponse>()];
        resp.encode(&mut buffer_encode).unwrap();

        let decoded = RequestDownstreamDeviceUpdateResponse::decode(&buffer_encode).unwrap();
        assert_eq!(resp, decoded);
    }

    #[test]
    fn test_downstream_device_codec() {
        let descriptor = Descriptor {
            descriptor_type: 0xff,
            descriptor_length: 0x02,
            descriptor_data: [0u8; 64],
        };
        let descriptors: [Descriptor; 3] = [descriptor, descriptor, descriptor];

        const DESC_LEN: usize = size_of::<Descriptor>();
        let mut descriptor_bytes_max: [u8; DESC_LEN * 3] = [0u8; DESC_LEN * 3];
        let mut offset = 0;

        // encoding
        for desc in descriptors.iter() {
            let encoded = desc.encode(&mut descriptor_bytes_max[offset..]).unwrap();
            offset += encoded;
        }

        let downstream_device_index = DownstreamDeviceIndex::try_from(1).unwrap();
        let downstream_device =
            DownstreamDevice::new(downstream_device_index, &descriptor_bytes_max);

        let mut buffer = [0u8; 256];
        let bytes_written = downstream_device.encode(&mut buffer).unwrap();
        let decoded = DownstreamDevice::decode(&buffer[..bytes_written]).unwrap();
        assert_eq!(downstream_device, decoded);
    }

    #[test]
    fn test_iterator_query_downstream_identifiers_response() {
        const DSC_DATA_LEN: usize = 16;
        let instance_id: InstanceId = 0x01;
        let ph: PortionHeader = PortionHeader {
            downstream_devices_length: 1,
            number_of_downstream_devices: 1,
        };
        let dsdh: DownstreamDevicesHeader = DownstreamDevicesHeader {
            downstream_devices_index: 1,
            downstream_descriptor_count: 3,
        };
        let dsc_0: Descriptor = Descriptor {
            descriptor_type: DescriptorType::VendorDefined as u16,
            descriptor_length: DSC_DATA_LEN as u16,
            descriptor_data: [0u8; 64],
        };

        let mut dsc_1 = dsc_0;
        dsc_1.descriptor_data[0..16].clone_from_slice(&[1u8; 16]);

        let mut dsc_2 = dsc_0;
        dsc_2.descriptor_data[0..16].clone_from_slice(&[2u8; 16]);

        const LEN: usize = size_of::<PortionHeader>()
            + size_of::<DownstreamDevicesHeader>()
            + 3 * (2 * size_of::<u16>() + DSC_DATA_LEN); // type + length + data

        let mut offset = 0;
        let mut portion: [u8; LEN] = [0u8; LEN];

        portion[0..offset + size_of::<PortionHeader>()].copy_from_slice(ph.as_bytes());
        offset += size_of::<PortionHeader>();

        portion[offset..offset + size_of::<DownstreamDevicesHeader>()]
            .copy_from_slice(dsdh.as_bytes());
        offset += size_of::<DownstreamDevicesHeader>();

        for desc in [dsc_0, dsc_1, dsc_2].iter() {
            let size = &desc.encode(&mut portion[offset..]).unwrap();
            offset += size;
        }

        let mut qdir = QueryDownstreamIdentifiersResponse::new(
            instance_id,
            QueryDownstreamIdentifiersResponseCode::BaseCodes(PldmBaseCompletionCode::Success),
            0x12345678,
            0x00,
            &portion,
        );

        // test iterator implementation
        let mut qdir_iter = qdir.next();
        let mut dsd_iter = qdir_iter.as_mut().unwrap().next();
        assert!(dsd_iter.is_some());
        assert_eq!(dsc_0, dsd_iter.unwrap());

        dsd_iter = qdir_iter.as_mut().unwrap().next();
        assert!(dsd_iter.is_some());
        assert_eq!(dsc_1, dsd_iter.unwrap());

        dsd_iter = qdir_iter.as_mut().unwrap().next();
        assert!(dsd_iter.is_some());
        assert_eq!(dsc_2, dsd_iter.unwrap());

        qdir_iter = qdir.next();
        assert!(qdir_iter.is_none());

        // test try_at
        assert!(qdir.try_at(0).is_ok());
        assert!(qdir.try_at(1).is_err());
    }
}
