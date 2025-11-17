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
use crate::protocol::base::{
    InstanceId, PLDM_MSG_HEADER_LEN, PldmMsgHeader, PldmMsgType, PldmSupportedType,
};
use crate::protocol::firmware_update::FwUpdateCmd;
use zerocopy::{FromBytes, Immutable, IntoBytes};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum VerifyResult {
    VerifySuccess = 0x00,
    VerifyErrorVerificationFailure = 0x01,
    VerifyErrorVersionMismatch = 0x02,
    VerifyFailedFdSecurityChecks = 0x03,
    VerifyErrorImageIncomplete = 0x04,
    VerifyTimeOut = 0x09,
    #[default]
    VerifyGenericError = 0x0a,
    VendorDefined,
}

impl TryFrom<u8> for VerifyResult {
    type Error = PldmError;

    fn try_from(value: u8) -> Result<Self, PldmError> {
        match value {
            0x00 => Ok(VerifyResult::VerifySuccess),
            0x01 => Ok(VerifyResult::VerifyErrorVerificationFailure),
            0x02 => Ok(VerifyResult::VerifyErrorVersionMismatch),
            0x03 => Ok(VerifyResult::VerifyFailedFdSecurityChecks),
            0x04 => Ok(VerifyResult::VerifyErrorImageIncomplete),
            0x09 => Ok(VerifyResult::VerifyTimeOut),
            0x0a => Ok(VerifyResult::VerifyGenericError),
            0x90..=0xaf => Ok(VerifyResult::VendorDefined),
            _ => Err(PldmError::InvalidVerifyResult),
        }
    }
}

#[derive(Debug, Clone, FromBytes, IntoBytes, Immutable, PartialEq)]
#[repr(C, packed)]
pub struct VerifyCompleteRequest {
    pub hdr: PldmMsgHeader<[u8; PLDM_MSG_HEADER_LEN]>,
    pub verify_result: u8,
}

impl VerifyCompleteRequest {
    pub fn new(
        instance_id: InstanceId,
        msg_type: PldmMsgType,
        verify_result: VerifyResult,
    ) -> Self {
        VerifyCompleteRequest {
            hdr: PldmMsgHeader::new(
                instance_id,
                msg_type,
                PldmSupportedType::FwUpdate,
                FwUpdateCmd::VerifyComplete as u8,
            ),
            verify_result: verify_result as u8,
        }
    }
}

#[derive(Debug, Clone, FromBytes, IntoBytes, Immutable, PartialEq)]
#[repr(C, packed)]
pub struct VerifyCompleteResponse {
    pub hdr: PldmMsgHeader<[u8; PLDM_MSG_HEADER_LEN]>,
    pub completion_code: u8,
}

impl VerifyCompleteResponse {
    pub fn new(instance_id: InstanceId, completion_code: u8) -> VerifyCompleteResponse {
        VerifyCompleteResponse {
            hdr: PldmMsgHeader::new(
                instance_id,
                PldmMsgType::Response,
                PldmSupportedType::FwUpdate,
                FwUpdateCmd::VerifyComplete as u8,
            ),
            completion_code,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::codec::PldmCodec;

    #[test]
    fn test_verify_complete_request_codec() {
        let request =
            VerifyCompleteRequest::new(0x01, PldmMsgType::Request, VerifyResult::VerifySuccess);
        let mut buffer = [0u8; core::mem::size_of::<VerifyCompleteRequest>()];
        request.encode(&mut buffer).unwrap();
        let decoded_request = VerifyCompleteRequest::decode(&buffer).unwrap();
        assert_eq!(request, decoded_request);
    }

    #[test]
    fn test_verify_complete_response_codec() {
        let response = VerifyCompleteResponse::new(0x01, 0x00);
        let mut buffer = [0u8; core::mem::size_of::<VerifyCompleteResponse>()];
        response.encode(&mut buffer).unwrap();
        let decoded_response = VerifyCompleteResponse::decode(&buffer).unwrap();
        assert_eq!(response, decoded_response);
    }
}
