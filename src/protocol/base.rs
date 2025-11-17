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

use crate::error::PldmError;
use bitfield::bitfield;
use core::convert::TryFrom;
use zerocopy::{FromBytes, Immutable, IntoBytes};

pub const PLDM_MSG_HEADER_LEN: usize = 3;
pub const PLDM_FAILURE_RESP_LEN: usize = 4;
pub type InstanceId = u8;

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum PldmSupportedType {
    Base = 0x00,
    Platform = 0x02,
    Bios = 0x03,
    Fru = 0x04,
    FwUpdate = 0x05,
    Oem = 0x3F,
}

impl TryFrom<u8> for PldmSupportedType {
    type Error = PldmError;

    fn try_from(value: u8) -> Result<Self, PldmError> {
        match value {
            0x00 => Ok(PldmSupportedType::Base),
            0x02 => Ok(PldmSupportedType::Platform),
            0x03 => Ok(PldmSupportedType::Bios),
            0x04 => Ok(PldmSupportedType::Fru),
            0x05 => Ok(PldmSupportedType::FwUpdate),
            0x3F => Ok(PldmSupportedType::Oem),
            _ => Err(PldmError::UnsupportedPldmType),
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum PldmControlCmd {
    SetTid = 0x1,
    GetTid = 0x2,
    GetPldmVersion = 0x3,
    GetPldmTypes = 0x4,
    GetPldmCommands = 0x5,
}

impl TryFrom<u8> for PldmControlCmd {
    type Error = PldmError;

    fn try_from(value: u8) -> Result<Self, PldmError> {
        match value {
            0x1 => Ok(PldmControlCmd::SetTid),
            0x2 => Ok(PldmControlCmd::GetTid),
            0x3 => Ok(PldmControlCmd::GetPldmVersion),
            0x4 => Ok(PldmControlCmd::GetPldmTypes),
            0x5 => Ok(PldmControlCmd::GetPldmCommands),
            _ => Err(PldmError::UnsupportedCmd),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum PldmMsgType {
    Response = 0x00,
    Reserved = 0x01,
    Request = 0x02,
    AsyncRequestNotify = 0x03,
}

impl TryFrom<u8> for PldmMsgType {
    type Error = PldmError;

    fn try_from(value: u8) -> Result<Self, PldmError> {
        match value {
            0x00 => Ok(PldmMsgType::Response),
            0x01 => Ok(PldmMsgType::Reserved),
            0x02 => Ok(PldmMsgType::Request),
            0x03 => Ok(PldmMsgType::AsyncRequestNotify),
            _ => Err(PldmError::InvalidMsgType),
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum PldmHeaderVersion {
    Version0 = 0x00,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum PldmBaseCompletionCode {
    Success = 0x00,
    Error = 0x01,
    InvalidData = 0x02,
    InvalidLength = 0x03,
    NotReady = 0x04,
    UnsupportedPldmCmd = 0x05,
    InvalidPldmType = 0x20,
}

impl TryFrom<u8> for PldmBaseCompletionCode {
    type Error = PldmError;

    fn try_from(value: u8) -> Result<Self, PldmError> {
        match value {
            0x00 => Ok(PldmBaseCompletionCode::Success),
            0x01 => Ok(PldmBaseCompletionCode::Error),
            0x02 => Ok(PldmBaseCompletionCode::InvalidData),
            0x03 => Ok(PldmBaseCompletionCode::InvalidLength),
            0x04 => Ok(PldmBaseCompletionCode::NotReady),
            0x05 => Ok(PldmBaseCompletionCode::UnsupportedPldmCmd),
            0x20 => Ok(PldmBaseCompletionCode::InvalidPldmType),
            _ => Err(PldmError::InvalidCompletionCode),
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum PldmControlCompletionCode {
    InvalidDataTransferHandle = 0x80,
    InvalidTransferOperationFlag = 0x81,
    InvalidPldmTypeInRequestData = 0x83,
    InvalidPldmVersionInRequestData = 0x84,
}

impl TryFrom<u8> for PldmControlCompletionCode {
    type Error = PldmError;

    fn try_from(value: u8) -> Result<Self, PldmError> {
        match value {
            0x80 => Ok(PldmControlCompletionCode::InvalidDataTransferHandle),
            0x81 => Ok(PldmControlCompletionCode::InvalidTransferOperationFlag),
            0x83 => Ok(PldmControlCompletionCode::InvalidPldmTypeInRequestData),
            0x84 => Ok(PldmControlCompletionCode::InvalidPldmVersionInRequestData),
            _ => Err(PldmError::InvalidCompletionCode),
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum TransferOperationFlag {
    GetNextPart = 0,
    GetFirstPart = 1,
}

impl TryFrom<u8> for TransferOperationFlag {
    type Error = PldmError;

    fn try_from(value: u8) -> Result<Self, PldmError> {
        match value {
            0 => Ok(TransferOperationFlag::GetNextPart),
            1 => Ok(TransferOperationFlag::GetFirstPart),
            _ => Err(PldmError::InvalidTransferOpFlag),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum TransferRespFlag {
    Start = 0x01,
    Middle = 0x02,
    End = 0x04,
    StartAndEnd = 0x05,
}

impl TryFrom<u8> for TransferRespFlag {
    type Error = PldmError;

    fn try_from(value: u8) -> Result<Self, PldmError> {
        match value {
            0x01 => Ok(TransferRespFlag::Start),
            0x02 => Ok(TransferRespFlag::Middle),
            0x04 => Ok(TransferRespFlag::End),
            0x05 => Ok(TransferRespFlag::StartAndEnd),
            _ => Err(PldmError::InvalidTransferRespFlag),
        }
    }
}

bitfield! {
    #[repr(C)]
    #[derive(Copy, Clone, FromBytes, IntoBytes, Immutable, PartialEq, Default)]
    pub struct PldmMsgHeader([u8]);
    impl Debug;
    pub u8, instance_id, set_instance_id: 4, 0;
    pub u8, reserved, _: 5, 5;
    pub u8, datagram, set_datagram: 6, 6;
    pub u8, rq, set_rq: 7, 7;
    pub u8, pldm_type, set_pldm_type: 13, 8;
    pub u8, hdr_ver, set_hdr_ver: 15, 14;
    pub u8, cmd_code, set_command_code: 23, 16;
}

impl PldmMsgHeader<[u8; PLDM_MSG_HEADER_LEN]> {
    const DATAGRAM_MASK: u8 = 0x01;
    const REQUEST_MASK: u8 = 0x01 << 1;

    pub fn new(
        instance_id: InstanceId,
        message_type: PldmMsgType,
        pldm_type: PldmSupportedType,
        cmd_code: u8,
    ) -> Self {
        let mut header = PldmMsgHeader([0; PLDM_MSG_HEADER_LEN]);
        header.set_instance_id(instance_id);
        header.set_datagram(message_type as u8 & Self::DATAGRAM_MASK);
        header.set_rq((message_type as u8 & Self::REQUEST_MASK) >> 1);
        header.set_pldm_type(pldm_type as u8);
        header.set_hdr_ver(PldmHeaderVersion::Version0 as u8);
        header.set_command_code(cmd_code);
        header
    }

    pub fn is_request(&self) -> bool {
        self.rq() == (PldmMsgType::Request as u8 >> 0x01)
    }

    pub fn is_hdr_ver_valid(&self) -> bool {
        self.hdr_ver() == PldmHeaderVersion::Version0 as u8
    }

    // switch the message type to response
    pub fn into_response(&self) -> Self {
        let mut header = *self;
        header.set_rq(PldmMsgType::Response as u8);
        header
    }
}

#[derive(Debug, FromBytes, IntoBytes, Immutable, PartialEq)]
#[repr(C, packed)]
pub struct PldmFailureResponse {
    pub hdr: PldmMsgHeader<[u8; PLDM_MSG_HEADER_LEN]>,
    pub completion_code: u8,
}

impl PldmFailureResponse {
    pub fn new(
        instance_id: InstanceId,
        pldm_type: PldmSupportedType,
        cmd_code: u8,
        completion_code: u8,
    ) -> Self {
        let hdr = PldmMsgHeader::new(instance_id, PldmMsgType::Response, pldm_type, cmd_code);
        PldmFailureResponse {
            hdr,
            completion_code,
        }
    }
}

/// Macro to define PLDM completion code enum with base codes and custom codes.
/// By default, the entire [PldmBaseCompletionCode] is included as a
/// variant named `BaseCodes`. The additional custom completion codes can be
/// specified as variants of [FwUpdateCompletionCode].
///
/// This macro is useful for the completion codes for the command responses, to ensure
/// type safety while still allowing the use of base completion codes and an
/// arbitrary number of custom completion codes specific to the command.
#[macro_export]
macro_rules! pldm_completion_code {
    (
        $enum_name:ident {
            $($variant:ident),* $(,)?
        }
    ) => {
        #[derive(PartialEq, Debug)]
        pub enum $enum_name {
            BaseCodes(PldmBaseCompletionCode),
            $($variant),*
        }

        impl From<$enum_name> for u8 {
            fn from(code: $enum_name) -> Self {
                match code {
                    $enum_name::BaseCodes(code) => code as u8,
                    $(
                        $enum_name::$variant => {
                            FwUpdateCompletionCode::$variant as u8
                        }
                    )*
                }
            }
        }

        impl TryFrom<u8> for $enum_name {
            type Error = $crate::error::PldmError;

            fn try_from(code: u8) -> Result<Self, Self::Error> {
                if let Ok(base_code) = PldmBaseCompletionCode::try_from(code) {
                    return Ok($enum_name::BaseCodes(base_code));
                }

                match FwUpdateCompletionCode::try_from(code) {
                    $(
                        Ok(FwUpdateCompletionCode::$variant) => {
                            Ok($enum_name::$variant)
                        }
                    )*
                    Ok(_) => Err($crate::error::PldmError::InvalidCompletionCode),
                    Err(e) => Err(e),
                }
        }
    }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::PldmCodec;
    use crate::protocol::firmware_update::FwUpdateCompletionCode;

    #[test]
    fn test_pldm_msg_header() {
        let header = PldmMsgHeader::new(
            0x01,
            PldmMsgType::Request,
            PldmSupportedType::Base,
            PldmControlCmd::GetTid as u8,
        );
        assert_eq!(header.0, [0x81, 0x00, 0x02]);
        assert!(header.is_request());
        let response = header.into_response();
        assert_eq!(response.0, [0x01, 0x00, 0x02]);
        assert_eq!(response.rq(), PldmMsgType::Response as u8);

        let mut buffer = [0; PLDM_MSG_HEADER_LEN];
        let size = header.encode(&mut buffer).unwrap();
        assert_eq!(size, PLDM_MSG_HEADER_LEN);

        let decoded_header = PldmMsgHeader::decode(&buffer).unwrap();
        assert_eq!(header, decoded_header);
    }

    #[test]
    fn test_pldm_failure_resp() {
        let resp = PldmFailureResponse::new(
            0x01,
            PldmSupportedType::Base,
            PldmControlCmd::GetTid as u8,
            PldmBaseCompletionCode::Success as u8,
        );

        let mut buffer = [0; PLDM_FAILURE_RESP_LEN];
        let size = resp.encode(&mut buffer).unwrap();
        assert_eq!(size, PLDM_FAILURE_RESP_LEN);

        let decoded_resp = PldmFailureResponse::decode(&buffer).unwrap();
        assert_eq!(resp, decoded_resp);
    }

    #[test]
    fn test_pldm_completion_code_macro() {
        pldm_completion_code! {
            TestEnum {
                InvalidStateForCommand,
                NoDeviceMetadata,
                InvalidTransferHandle,
                InvalidTransferOperationFlag,
                PackageDataError,
            }
        }

        let base = TestEnum::BaseCodes(PldmBaseCompletionCode::Success);
        if let TestEnum::BaseCodes(i) = base {
            assert_eq!(i, PldmBaseCompletionCode::Success);
        }
    }
}
