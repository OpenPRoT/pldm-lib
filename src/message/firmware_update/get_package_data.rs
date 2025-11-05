// Licensed under the Apache-2.0 license

use crate::protocol;
use crate::protocol::base::{
    InstanceId, PldmMsgHeader, PldmMsgType, PldmSupportedType, TransferOperationFlag,
    PLDM_MSG_HEADER_LEN,
};

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

pub enum GetPackageDataCodes {
    BaseCodes(protocol::base::PldmBaseCompletionCode),
    CommandNotExpected,
    NoPackageData,
    InvalidTransferHandle,
    InvalidTransferOperationFlag,
}

impl From<GetPackageDataCodes> for u8 {
    fn from(code: GetPackageDataCodes) -> Self {
        match code {
            GetPackageDataCodes::BaseCodes(code) => code as u8,
            GetPackageDataCodes::CommandNotExpected => {
                FwUpdateCompletionCode::CommandNotExpected as u8
            }
            GetPackageDataCodes::NoPackageData => FwUpdateCompletionCode::NoPackageData as u8,
            GetPackageDataCodes::InvalidTransferHandle => {
                FwUpdateCompletionCode::InvalidTransferHandle as u8
            }
            GetPackageDataCodes::InvalidTransferOperationFlag => {
                FwUpdateCompletionCode::InvalidTransferOperationFlag as u8
            }
        }
    }
}

pub struct GetPackageDataResponse<'a> {
    pub hdr: PldmMsgHeader<[u8; PLDM_MSG_HEADER_LEN]>,

    /// PLDM_BASE_CODES, COMMAND_NOT_EXPECTED, NO_PACKAGE_DATA,
    /// INVALID_TRANSFER_HANDLE, INVALID_TRANSFER_OPERATION_FLAG
    ///
    /// See [GetPackageDataCodes]
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
        completion_code: GetPackageDataCodes,
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

pub enum GetDeviceMetaDataCodes {
    BaseCodes(protocol::base::PldmBaseCompletionCode),
    InvalidStateForCommand,
    NoDeviceMetadata,
    InvalidTransferHandle,
    InvalidTransferOperationFlag,
    PackageDataError,
}

impl From<GetDeviceMetaDataCodes> for u8 {
    fn from(code: GetDeviceMetaDataCodes) -> Self {
        match code {
            GetDeviceMetaDataCodes::BaseCodes(code) => code as u8,
            GetDeviceMetaDataCodes::InvalidStateForCommand => {
                FwUpdateCompletionCode::InvalidStateForCommand as u8
            }
            GetDeviceMetaDataCodes::NoDeviceMetadata => {
                FwUpdateCompletionCode::NoDeviceMetadata as u8
            }
            GetDeviceMetaDataCodes::InvalidTransferHandle => {
                FwUpdateCompletionCode::InvalidTransferHandle as u8
            }
            GetDeviceMetaDataCodes::InvalidTransferOperationFlag => {
                FwUpdateCompletionCode::InvalidTransferOperationFlag as u8
            }
            GetDeviceMetaDataCodes::PackageDataError => {
                FwUpdateCompletionCode::PackageDataError as u8
            }
        }
    }
}

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

#[cfg(test)]
mod tests {
    use crate::protocol;

    use super::*;

    //TODO: Add a test that behaves like the example in DSP0267, Fig. 11
    //TODO: take ensure that the creation of these packages fits to the
    // description of Table 28, PortionOfPackageData Field
    #[test]
    fn test_get_package_data() {
        let instance_id: InstanceId = 0x01;
        //TODO: use FwUpdateCompletionCode
        let completion_code =
            GetPackageDataCodes::BaseCodes(protocol::base::PldmBaseCompletionCode::Success);
        let next_data_transfer_handle: u32 = 0x00000010;
        let transfer_flag = TransferOperationFlag::GetFirstPart;
        let portion_of_package_data: &[u8] = &[0xAA; GET_PACKAGE_DATA_PORTION_SIZE];

        let response = GetPackageDataResponse::new(
            instance_id,
            completion_code,
            next_data_transfer_handle,
            transfer_flag,
            portion_of_package_data,
        );
        assert_eq!(response.hdr.rq(), PldmMsgType::Response as u8);
        assert_eq!(
            response.completion_code,
            GetPackageDataCodes::BaseCodes(protocol::base::PldmBaseCompletionCode::Success).into()
        );
        assert_eq!(
            response.next_data_transfer_handle,
            next_data_transfer_handle
        );
        assert_eq!(response.transfer_flag, transfer_flag as u8);
        assert_eq!(response.portion_of_package_data, portion_of_package_data);
    }

    #[test]
    fn test_get_device_metadata() {
        let instance_id: InstanceId = 0x01;
        let completion_code =
            GetDeviceMetaDataCodes::BaseCodes(protocol::base::PldmBaseCompletionCode::Success);
        let next_data_transfer_handle: u32 = 0x00000020;
        let transfer_flag = TransferOperationFlag::GetFirstPart;
        let portion_of_device_metadata: &[u8] = &[0xBB; 0xff];
        let response = GetDeviceMetaDataResponse::new(
            instance_id,
            completion_code,
            next_data_transfer_handle,
            transfer_flag,
            portion_of_device_metadata,
        );

        assert_eq!(response.hdr.rq(), PldmMsgType::Response as u8);
        assert_eq!(
            response.completion_code,
            GetDeviceMetaDataCodes::BaseCodes(protocol::base::PldmBaseCompletionCode::Success)
                .into()
        );
        assert_eq!(
            response.next_data_transfer_handle,
            next_data_transfer_handle
        );
        assert_eq!(response.transfer_flag, transfer_flag as u8);
        assert_eq!(
            response.portion_of_device_metadata,
            portion_of_device_metadata
        );
    }
}
