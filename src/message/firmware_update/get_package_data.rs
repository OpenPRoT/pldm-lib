// Licensed under the Apache-2.0 license

use crate::protocol::base::{
    InstanceId, PldmBaseCompletionCode, PldmMsgHeader, PldmMsgType, PldmSupportedType,
    TransferOperationFlag, PLDM_MSG_HEADER_LEN,
};

use crate::pldm_completion_code;

use crate::protocol::firmware_update::{FwUpdateCmd, FwUpdateCompletionCode};
use zerocopy::{FromBytes, Immutable, IntoBytes};

pub const GET_PACKAGE_DATA_PORTION_SIZE: usize = 1024;

#[derive(Debug, Clone, FromBytes, IntoBytes, Immutable, PartialEq)]
#[repr(C, packed)]
/// The FD sends this command to transfer optional data that shall be received prior to transferring
/// components during the firmware update process. This command is only used if the firmware update
/// package contained content within the [FirmwareDevicePackageData] field, the UA provided the length of
/// the package data in the RequestUpdate command, and the FD indicated that it would use this command
/// in the [FDWillSendGetPackageDataCommand] field.
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

#[derive(Debug, Clone, FromBytes, IntoBytes, Immutable, PartialEq)]
#[repr(C, packed)]
pub struct GetPackageDataResponse<'a> {
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
    pub portion_of_package_data: &'a [u8],
}

impl<'a> GetPackageDataResponse<'a> {
    pub fn new(
        instance_id: InstanceId,
        completion_code: GetPackageDataCode,
        next_data_transfer_handle: u32,
        transfer_flag: TransferOperationFlag,
        portion_of_package_data: &'a [u8],
    ) -> Self {
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
            portion_of_package_data,
        }
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

/// The FD sends this command to transfer the data that was originally obtained by the UA through the
/// [GetDeviceMetaData] command. This command shall only be used if the FD indicated in the
/// [RequestUpdate] response that it had device metadata that needed to be obtained by the UA. The FD can
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
    /// baseline transfer size and MaximumTransferSize from the [RequestUpdate] or
    /// [RequestDownstreamDeviceUpdate] command. When TransferFlag = End or StartAndEnd, the
    /// variable size of this field can also be less than the firmware update baseline transfer size.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol;

    use crate::codec::PldmCodec;

    #[test]
    fn test_get_package_data_request() {
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
    fn test_get_data_response() {
        let instance_id: InstanceId = 0x01;
        let next_data_transfer_handle: u32 = 0x12345678;
        let transfer_operation_flag = TransferOperationFlag::GetFirstPart;
        let portion = [0u8; 0xff];

        let _ = GetPackageDataResponse::new(
            instance_id,
            GetPackageDataCode::BaseCodes(PldmBaseCompletionCode::Success),
            next_data_transfer_handle,
            transfer_operation_flag,
            &portion,
        );

        //TODO: encoding for response does not work atm due to unknown sizes
        // let mut buffer = [0u8; core::mem::size_of::<GetPackageDataResponse>()];
        // request.encode(&mut buffer).unwrap();
        // let decoded = GetPackageDataResponse::decode(&buffer).unwrap();

        // assert_eq!(request, decoded);
    }

    #[test]
    fn test_get_device_metadata_request() {}

    #[test]
    fn test_get_device_metadata_reponse() {}
}
