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

use crate::protocol::base::{
    InstanceId, PLDM_MSG_HEADER_LEN, PldmBaseCompletionCode, PldmMsgHeader, PldmMsgType,
    PldmSupportedType, TransferOperationFlag,
};

use crate::pldm_completion_code;

use crate::codec::{PldmCodec, PldmCodecError, PldmCodecWithLifetime};
use crate::protocol::firmware_update::{FwUpdateCmd, FwUpdateCompletionCode};
use zerocopy::{FromBytes, Immutable, IntoBytes};

pub const GET_PACKAGE_DATA_PORTION_SIZE: usize = 1024;

#[derive(Debug, Clone, FromBytes, IntoBytes, Immutable, PartialEq)]
#[repr(C, packed)]
/// The FD sends this command to transfer optional data that shall be received prior to transferring
/// components during the firmware update process. This command is only used if the firmware update
/// package contained content within the FirmwareDevicePackageData field, the UA provided the length of
/// the package data in the RequestUpdate command, and the FD indicated that it would use this command
/// in the FDWillSendGetPackageDataCommand field.
pub struct GetPackageDataRequest {
    pub hdr: PldmMsgHeader<[u8; PLDM_MSG_HEADER_LEN]>,
    pub data_transfer_handle: u32,
    pub transfer_operation_flag: u8,
}

impl GetPackageDataRequest {
    pub fn new(
        instance_id: InstanceId,
        data_transfer_handle: u32,
        transfer_operation_flag: TransferOperationFlag,
    ) -> Self {
        GetPackageDataRequest {
            hdr: PldmMsgHeader::new(
                instance_id,
                PldmMsgType::Request,
                PldmSupportedType::FwUpdate,
                FwUpdateCmd::GetPackageData as u8,
            ),
            data_transfer_handle,
            transfer_operation_flag: transfer_operation_flag as u8,
        }
    }
}

pldm_completion_code! {
    GetPackageDataCode {
        CommandNotExpected,
        NoPackageData,
        InvalidTransferHandle,
        InvalidTransferOperationFlag
    }
}

const MAX_PORTION_DATA_SIZE: usize = 0xff;
#[derive(Debug, Clone, PartialEq, FromBytes)]
#[repr(C)]
/// GetPackageDataResponse is parameterized over the portion package data size.
pub struct GetPackageDataResponse {
    pub hdr: PldmMsgHeader<[u8; PLDM_MSG_HEADER_LEN]>,

    /// PLDM_BASE_CODES, COMMAND_NOT_EXPECTED, NO_PACKAGE_DATA,
    /// INVALID_TRANSFER_HANDLE, INVALID_TRANSFER_OPERATION_FLAG
    ///
    /// See [GetPackageDataCode]
    pub completion_code: u8,
    pub next_data_transfer_handle: u32,
    pub transfer_flag: u8,

    /// If the FD provided a value in the GetPackageDataMaximumTransferSize field, then the UA should
    /// select the amount of data to return such that the byte length for this field, except when TransferFlag
    /// = End or StartAndEnd, is equal to or less than that value.
    pub portion_of_package_data: [u8; MAX_PORTION_DATA_SIZE],

    // Non-spec field. Since the spec tells us that this can vary in unknown size,
    // we save the max and keep track of the actual size in this var.
    // When encoding this into byte form, we have to omit this field and only
    // encode as many bytes of [portion_of_package] as [portion_of_package_data_len]
    // tells us to.
    portion_of_package_data_len: usize,
}

impl GetPackageDataResponse {
    pub fn new(
        instance_id: InstanceId,
        completion_code: GetPackageDataCode,
        next_data_transfer_handle: u32,
        transfer_flag: TransferOperationFlag,
        portion_of_package_data: &[u8],
    ) -> Self {
        let mut pdata: [u8; MAX_PORTION_DATA_SIZE] = [0x00; MAX_PORTION_DATA_SIZE];
        pdata[0..portion_of_package_data.len()].copy_from_slice(portion_of_package_data);
        GetPackageDataResponse {
            hdr: PldmMsgHeader::new(
                instance_id,
                PldmMsgType::Response,
                PldmSupportedType::FwUpdate,
                FwUpdateCmd::GetPackageData as u8,
            ),
            completion_code: completion_code.into(),
            next_data_transfer_handle,
            transfer_flag: transfer_flag as u8,
            portion_of_package_data: pdata,
            portion_of_package_data_len: portion_of_package_data.len(),
        }
    }
}

// See: src/message/firmware_update/get_fw_params.rs for manual decode etc
impl PldmCodec for GetPackageDataResponse {
    fn encode(&self, buffer: &mut [u8]) -> Result<usize, crate::codec::PldmCodecError> {
        if buffer.len()
            < core::mem::size_of::<GetPackageDataResponse>()
                - MAX_PORTION_DATA_SIZE
                - core::mem::size_of_val(&self.portion_of_package_data_len)
                + self.portion_of_package_data_len
        {
            return Err(PldmCodecError::BufferTooShort);
        }

        let mut offset = 0;
        buffer[offset..offset + size_of_val(&self.hdr.0)].copy_from_slice(&self.hdr.0);
        offset += size_of_val(&self.hdr.0);

        buffer[offset] = self.completion_code;
        offset += 1;

        buffer[offset..offset + size_of_val(&self.next_data_transfer_handle)]
            .copy_from_slice(self.next_data_transfer_handle.as_bytes());
        offset += size_of_val(&self.next_data_transfer_handle);

        buffer[offset] = self.transfer_flag;
        offset += 1;

        buffer[offset..offset + self.portion_of_package_data_len].copy_from_slice(
            self.portion_of_package_data[0..self.portion_of_package_data_len].as_bytes(),
        );

        Ok(offset + self.portion_of_package_data_len)
    }

    fn decode(buffer: &[u8]) -> Result<Self, crate::codec::PldmCodecError> {
        const MIN_LEN: usize = core::mem::size_of::<GetPackageDataResponse>()
            - core::mem::size_of::<usize>()
            - core::mem::size_of::<u8>() * MAX_PORTION_DATA_SIZE;

        if buffer.len() < MIN_LEN {
            return Err(PldmCodecError::BufferTooShort);
        }

        let mut offset = 0;

        let mut hdr_bytes = [0u8; PLDM_MSG_HEADER_LEN];
        hdr_bytes.copy_from_slice(&buffer[offset..offset + PLDM_MSG_HEADER_LEN]);

        let hdr = PldmMsgHeader(hdr_bytes);
        offset += PLDM_MSG_HEADER_LEN;

        let completion_code = GetPackageDataCode::try_from(buffer[offset])
            .map_err(|_| PldmCodecError::Unsupported)?;
        offset += 1;

        let next_data_transfer_handle = u32::from_le_bytes([
            buffer[offset],
            buffer[offset + 1],
            buffer[offset + 2],
            buffer[offset + 3],
        ]);
        offset += 4;

        let transfer_flag = TransferOperationFlag::try_from(buffer[offset])
            .map_err(|_| PldmCodecError::Unsupported)?;
        offset += 1;

        let portion_len = buffer.len() - offset;
        if portion_len > MAX_PORTION_DATA_SIZE {
            return Err(PldmCodecError::BufferTooShort);
        }

        let portion_data = &buffer[offset..];
        Ok(Self::new(
            hdr.instance_id(),
            completion_code,
            next_data_transfer_handle,
            transfer_flag,
            portion_data,
        ))
    }
}

/// The UA sends this command to acquire optional data that the FD shall transfer to the UA prior to
/// beginning the transfer of component images. This command is only used if the FD has indicated in the
/// RequestUpdate command response that it has data that shall be retrieved and restored by the UA. The
/// firmware device metadata retrieved by this command will be sent back to the FD through the
/// GetMetaData command after all component images have been transferred.
///
#[derive(Debug, Clone, FromBytes, IntoBytes, Immutable, PartialEq)]
#[repr(C, packed)]
pub struct GetDeviceMetaDataRequest {
    pub hdr: PldmMsgHeader<[u8; PLDM_MSG_HEADER_LEN]>,
    pub data_transfer_handle: u32,
    pub transfer_operation_flag: u8,
}

impl GetDeviceMetaDataRequest {
    pub fn new(
        instance_id: InstanceId,
        data_transfer_handle: u32,
        transfer_operation_flag: TransferOperationFlag,
    ) -> Self {
        GetDeviceMetaDataRequest {
            hdr: PldmMsgHeader::new(
                instance_id,
                PldmMsgType::Request,
                PldmSupportedType::FwUpdate,
                FwUpdateCmd::GetDeviceMetaData as u8,
            ),
            data_transfer_handle,
            transfer_operation_flag: transfer_operation_flag as u8,
        }
    }
}

pldm_completion_code! {
    GetDeviceMetaDataCodes {
    InvalidStateForCommand,
    NoDeviceMetadata,
    InvalidTransferHandle,
    InvalidTransferOperationFlag,
    PackageDataError,
}}

#[derive(Debug, Clone, FromBytes, IntoBytes, Immutable, PartialEq)]
#[repr(C, packed)]
pub struct GetDeviceMetaDataResponse<'a> {
    pub hdr: PldmMsgHeader<[u8; PLDM_MSG_HEADER_LEN]>,

    /// PLDM_BASE_CODES, INVALID_STATE_FOR_COMMAND, NO_DEVICE_METADATA,
    /// INVALID_TRANSFER_HANDLE, INVALID_TRANSFER_OPERATION_FLAG, PACKAGE_DATA_ERROR
    ///
    /// See [GetDeviceMetaDataCodes]
    pub completion_code: u8,
    pub next_data_transfer_handle: u32,
    pub transfer_flag: u8,

    /// The FD should select the amount of data to return such that the byte length for this field, except
    /// when TransferFlag = End or StartAndEnd, is equal to or between the values of the firmware update
    /// baseline transfer size and MaximumTransferSize from the RequestUpdate or
    /// RequestDownstreamDeviceUpdate command. When TransferFlag = End or StartAndEnd, the
    /// variable size of this field can also be less than the firmware update baseline transfer size.
    pub portion_of_device_metadata: &'a [u8],
}

impl<'a> GetDeviceMetaDataResponse<'a> {
    pub fn new(
        instance_id: InstanceId,
        completion_code: GetDeviceMetaDataCodes,
        next_data_transfer_handle: u32,
        transfer_flag: TransferOperationFlag,
        portion_of_device_metadata: &'a [u8],
    ) -> Self {
        GetDeviceMetaDataResponse {
            hdr: PldmMsgHeader::new(
                instance_id,
                PldmMsgType::Response,
                PldmSupportedType::FwUpdate,
                FwUpdateCmd::GetDeviceMetaData as u8,
            ),
            completion_code: completion_code.into(),
            next_data_transfer_handle,
            transfer_flag: transfer_flag as u8,
            portion_of_device_metadata,
        }
    }
}

impl<'a> PldmCodecWithLifetime<'a> for GetDeviceMetaDataResponse<'a> {
    fn encode(&self, buffer: &mut [u8]) -> Result<usize, PldmCodecError> {
        let size = core::mem::size_of::<Self>() - core::mem::size_of::<&'a [u8]>();
        if buffer.len() < size + self.portion_of_device_metadata.len() {
            return Err(PldmCodecError::BufferTooShort);
        }

        let mut offset = 0;
        self.hdr
            .write_to_prefix(&mut buffer[offset..])
            .map_err(|_| PldmCodecError::BufferTooShort)?;
        offset += PLDM_MSG_HEADER_LEN;

        buffer[offset] = self.completion_code;
        offset += 1;

        buffer[offset..offset + size_of::<u32>()]
            .copy_from_slice(&self.next_data_transfer_handle.to_le_bytes());
        offset += size_of::<u32>();

        buffer[offset] = self.transfer_flag;
        offset += size_of::<u8>();

        buffer[offset..offset + self.portion_of_device_metadata.len()]
            .copy_from_slice(self.portion_of_device_metadata);

        Ok(offset + self.portion_of_device_metadata.len())
    }

    fn decode(buffer: &'a [u8]) -> Result<Self, PldmCodecError> {
        let size = core::mem::size_of::<Self>() - core::mem::size_of::<&'a [u8]>();
        if buffer.len() < size {
            return Err(PldmCodecError::BufferTooShort);
        }

        let mut offset = 0;
        let hdr = PldmMsgHeader::read_from_prefix(&buffer[offset..])
            .map_err(|_| PldmCodecError::BufferTooShort)?
            .0;
        offset += PLDM_MSG_HEADER_LEN;

        let completion_code = buffer[offset];
        offset += size_of::<u8>();

        let next_data_transfer_handle = u32::from_le_bytes(
            buffer[offset..offset + 4]
                .try_into()
                .map_err(|_| PldmCodecError::BufferTooShort)?,
        );
        offset += size_of::<u32>();

        let transfer_flag = buffer[offset];
        offset += size_of::<u8>();

        let portion_of_device_metadata = &buffer[offset..];

        Ok(Self {
            hdr,
            completion_code,
            next_data_transfer_handle,
            transfer_flag,
            portion_of_device_metadata,
        })
    }
}

/// The FD sends this command to transfer the data that was originally obtained by the UA through the
/// [GetDeviceMetaDataRequest] command. This command shall only be used if the FD indicated in the
/// RequestUpdate response that it had device metadata that needed to be obtained by the UA. The FD can
/// send this command when it is in any state, except the IDLE and LEARN COMPONENTS state.
#[derive(Debug, Clone, FromBytes, IntoBytes, Immutable, PartialEq)]
#[repr(C, packed)]
pub struct GetMetaDataRequest {
    pub hdr: PldmMsgHeader<[u8; PLDM_MSG_HEADER_LEN]>,
    pub data_transfer_handle: u32,
    pub transfer_operation_flag: u8,
}

impl GetMetaDataRequest {
    pub fn new(
        instance_id: InstanceId,
        data_transfer_handle: u32,
        transfer_operation_flag: TransferOperationFlag,
    ) -> Self {
        GetMetaDataRequest {
            hdr: PldmMsgHeader::new(
                instance_id,
                PldmMsgType::Request,
                PldmSupportedType::FwUpdate,
                FwUpdateCmd::GetMetaData as u8,
            ),
            data_transfer_handle,
            transfer_operation_flag: transfer_operation_flag as u8,
        }
    }
}

pldm_completion_code! {
    GetMetaDataCode {
        CommandNotExpected,
        InvalidTransferHandle,
        InvalidTransferOperationFlag,
    }
}

#[derive(Debug, Clone, FromBytes, IntoBytes, Immutable, PartialEq)]
#[repr(C, packed)]
pub struct GetMetaDataResponse<'a> {
    pub hdr: PldmMsgHeader<[u8; PLDM_MSG_HEADER_LEN]>,

    /// PLDM_BASE_CODES, COMMAND_NOT_EXPECTED, INVALID_TRANSFER_HANDLE,
    /// INVALID_TRANSFER_OPERATION_FLAG
    ///
    /// See [GetMetaDataCode]
    pub completion_code: u8,
    pub next_data_transfer_handle: u32,
    pub transfer_flag: u8,

    /// The UA should select the amount of data to return such that the byte length for this field, except
    /// when TransferFlag = End or StartAndEnd, is equal to or between the values of the firmware update
    /// baseline transfer size and MaximumTransferSize from the RequestUpdate or
    /// [crate::message::firmware_update::query_downstream::RequestDownstreamDeviceUpdateRequest] command.
    ///  When TransferFlag = End or StartAndEnd, the variable size of this field can also be less than
    /// the firmware update baseline transfer size.
    pub portion_of_device_metadata: &'a [u8],
}

impl<'a> GetMetaDataResponse<'a> {
    pub fn new(
        instance_id: InstanceId,
        completion_code: GetMetaDataCode,
        next_data_transfer_handle: u32,
        transfer_flag: TransferOperationFlag,
        portion_of_device_metadata: &'a [u8],
    ) -> Self {
        GetMetaDataResponse {
            hdr: PldmMsgHeader::new(
                instance_id,
                PldmMsgType::Response,
                PldmSupportedType::FwUpdate,
                FwUpdateCmd::GetMetaData as u8,
            ),
            completion_code: completion_code.into(),
            next_data_transfer_handle,
            transfer_flag: transfer_flag as u8,
            portion_of_device_metadata,
        }
    }
}

impl<'a> PldmCodecWithLifetime<'a> for GetMetaDataResponse<'a> {
    fn encode(&self, buffer: &mut [u8]) -> Result<usize, PldmCodecError> {
        let size = core::mem::size_of::<Self>() - core::mem::size_of::<&'a [u8]>();
        if buffer.len() < size + self.portion_of_device_metadata.len() {
            return Err(PldmCodecError::BufferTooShort);
        }

        let mut offset = 0;
        self.hdr
            .write_to_prefix(&mut buffer[offset..])
            .map_err(|_| PldmCodecError::BufferTooShort)?;
        offset += PLDM_MSG_HEADER_LEN;

        buffer[offset] = self.completion_code;
        offset += 1;

        buffer[offset..offset + 4].copy_from_slice(&self.next_data_transfer_handle.to_le_bytes());
        offset += 4;

        buffer[offset] = self.transfer_flag;
        offset += 1;

        buffer[offset..offset + self.portion_of_device_metadata.len()]
            .copy_from_slice(self.portion_of_device_metadata);

        Ok(offset + self.portion_of_device_metadata.len())
    }

    fn decode(buffer: &'a [u8]) -> Result<Self, PldmCodecError> {
        let size = core::mem::size_of::<Self>() - core::mem::size_of::<&'a [u8]>();
        if buffer.len() < size {
            return Err(PldmCodecError::BufferTooShort);
        }

        let mut offset = 0;

        let hdr = PldmMsgHeader::read_from_prefix(&buffer[offset..])
            .map_err(|_| PldmCodecError::BufferTooShort)?
            .0;
        offset += PLDM_MSG_HEADER_LEN;

        let completion_code = buffer[offset];
        offset += 1;

        let next_data_transfer_handle = u32::from_le_bytes(
            buffer[offset..offset + 4]
                .try_into()
                .map_err(|_| PldmCodecError::BufferTooShort)?,
        );
        offset += 4;

        let transfer_flag = buffer[offset];
        offset += 1;

        let portion_of_device_metadata = &buffer[offset..];

        Ok(Self {
            hdr,
            completion_code,
            next_data_transfer_handle,
            transfer_flag,
            portion_of_device_metadata,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::PldmCodec;

    #[test]
    fn test_get_package_data_request_codec() {
        let instance_id: InstanceId = 0x01;
        let data_transfer_handle: u32 = 0x12345678;
        let transfer_operation_flag = TransferOperationFlag::GetFirstPart;

        let request =
            GetPackageDataRequest::new(instance_id, data_transfer_handle, transfer_operation_flag);

        let mut buffer = [0u8; core::mem::size_of::<GetPackageDataRequest>()];
        request.encode(&mut buffer).unwrap();
        let decoded = GetPackageDataRequest::decode(&buffer).unwrap();

        assert_eq!(request, decoded);
    }

    #[test]
    fn test_get_package_data_response_codec() {
        const PORTION_LEN: usize = 10;

        let instance_id: InstanceId = 0x01;
        let next_data_transfer_handle: u32 = 0x12345678;
        let transfer_operation_flag = TransferOperationFlag::GetFirstPart;
        let portion = [22u8; PORTION_LEN];

        let resp = GetPackageDataResponse::new(
            instance_id,
            GetPackageDataCode::BaseCodes(PldmBaseCompletionCode::Success),
            next_data_transfer_handle,
            transfer_operation_flag,
            &portion,
        );

        let mut buffer_fitted = [0u8; core::mem::size_of::<GetPackageDataResponse>()
            - MAX_PORTION_DATA_SIZE
            - core::mem::size_of::<usize>()
            + PORTION_LEN];

        resp.encode(&mut buffer_fitted).unwrap();
        let decoded = GetPackageDataResponse::decode(&buffer_fitted).unwrap();
        assert_eq!(resp, decoded);
    }

    #[test]
    fn test_get_metadata_request_codec() {
        let instance_id: InstanceId = 0x01;
        let data_transfer_handle = 0x12345678;
        let req = GetMetaDataRequest::new(
            instance_id,
            data_transfer_handle,
            TransferOperationFlag::GetFirstPart,
        );

        let mut buffer = [0u8; core::mem::size_of::<GetMetaDataRequest>()];
        req.encode(&mut buffer).unwrap();

        let decoded = GetMetaDataRequest::decode(&buffer).unwrap();
        assert_eq!(req, decoded);
    }

    #[test]
    fn test_get_meta_data_response_codec() {
        let instance_id: InstanceId = 0x01;
        let data_transfer_handle = 0x12345678;
        let payload = [11u8; 20];

        let resp = GetMetaDataResponse::new(
            instance_id,
            GetMetaDataCode::BaseCodes(PldmBaseCompletionCode::Success),
            data_transfer_handle,
            TransferOperationFlag::GetFirstPart,
            &payload,
        );

        let mut buffer =
            [0u8; core::mem::size_of::<GetMetaDataResponse>() - core::mem::size_of::<&[u8]>() + 20];
        resp.encode(&mut buffer).unwrap();

        let decoded = GetMetaDataResponse::decode(&buffer).unwrap();
        assert_eq!(resp, decoded);
    }

    #[test]
    fn test_get_device_meta_data_request_codec() {
        let instance_id: InstanceId = 0x01;
        let data_transfer_handle = 0x12345678;
        let req = GetDeviceMetaDataRequest::new(
            instance_id,
            data_transfer_handle,
            TransferOperationFlag::GetFirstPart,
        );

        let mut buffer = [0u8; core::mem::size_of::<GetDeviceMetaDataRequest>()];
        req.encode(&mut buffer).unwrap();

        let decoded = GetDeviceMetaDataRequest::decode(&buffer).unwrap();
        assert_eq!(req, decoded);
    }

    #[test]
    fn test_get_device_meta_data_response_codec() {
        let instance_id: InstanceId = 0x01;
        let data_transfer_handle = 0x12345678;
        const TEST_PAYLOAD_LEN: usize = 20;
        let payload = [11u8; TEST_PAYLOAD_LEN];

        let resp = GetDeviceMetaDataResponse::new(
            instance_id,
            GetDeviceMetaDataCodes::BaseCodes(PldmBaseCompletionCode::Success),
            data_transfer_handle,
            TransferOperationFlag::GetFirstPart,
            &payload,
        );

        let mut buffer = [0u8; core::mem::size_of::<GetDeviceMetaDataResponse>()
            - core::mem::size_of::<&[u8]>()
            + TEST_PAYLOAD_LEN];
        resp.encode(&mut buffer).unwrap();

        let decoded = GetDeviceMetaDataResponse::decode(&buffer).unwrap();
        assert_eq!(resp, decoded);
    }
}
