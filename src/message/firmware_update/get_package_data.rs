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

    /// PLDM_BASE_CODES, COMMAND_NOT_EXPECTED, NO_PACKAGE_DATA, INVALID_TRANSFER_HANDLE, INVALID_TRANSFER_OPERATION_FLAG
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

#[cfg(test)]
mod tests {
    use crate::protocol;

    use super::*;

    //TODO: Add a test that behaves like the example in DSP0267, Fig. 11
    //TODO: take ensure that the creation of these packages fits to the
    // description of Table 28, PortionOfPackageData Field
    #[test]
    fn test_package_data_flow() {
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
}
