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

//! GetPDRRepositoryInfo, command code 0x50, DSP0248 clause 26.1, Table 68.

use crate::error::PldmError;
use crate::protocol::base::{
    InstanceId, PldmMsgHeader, PldmMsgType, PldmSupportedType, PLDM_MSG_HEADER_LEN,
};
use crate::protocol::platform::{PdrRepositoryState, PldmPlatformCmd, Timestamp104};
use zerocopy::{FromBytes, Immutable, IntoBytes};

/// Encoded length of the response, DSP0248 Table 68.
///
/// Taken from the packed struct, whose layout is the wire layout. The test
/// below pins that layout to the byte count Table 68 prints.
pub const GET_PDR_REPOSITORY_INFO_RESPONSE_LEN: usize =
    core::mem::size_of::<GetPdrRepositoryInfoResponse>();

/// `dataTransferHandleTimeout` value meaning a handle never expires.
pub const DATA_TRANSFER_HANDLE_TIMEOUT_NONE: u8 = 0x00;

/// `dataTransferHandleTimeout` value meaning the MC1 default minimum.
pub const DATA_TRANSFER_HANDLE_TIMEOUT_DEFAULT: u8 = 0x01;

/// `dataTransferHandleTimeout` value meaning longer than 254 seconds.
pub const DATA_TRANSFER_HANDLE_TIMEOUT_OVER_254S: u8 = 0xFF;

#[derive(Debug, Clone, FromBytes, IntoBytes, Immutable, PartialEq)]
#[repr(C, packed)]
pub struct GetPdrRepositoryInfoRequest {
    pub hdr: PldmMsgHeader<[u8; PLDM_MSG_HEADER_LEN]>,
}

impl GetPdrRepositoryInfoRequest {
    pub fn new(instance_id: InstanceId, message_type: PldmMsgType) -> Self {
        GetPdrRepositoryInfoRequest {
            hdr: PldmMsgHeader::new(
                instance_id,
                message_type,
                PldmSupportedType::Platform,
                PldmPlatformCmd::GetPdrRepositoryInfo as u8,
            ),
        }
    }
}

/// The repository properties a PDR provider supplies for the response.
///
/// Grouping them keeps the response constructor to three arguments and gives
/// the provider side of the pldm-lib boundary one type to return.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PdrRepositoryInfo {
    pub repository_state: PdrRepositoryState,
    /// Last change to the standard PDRs.
    pub update_time: Timestamp104,
    /// Last change to the OEM PDRs.
    pub oem_update_time: Timestamp104,
    pub record_count: u32,
    /// Cumulative size of the record fields, excluding internal headers.
    pub repository_size: u32,
    pub largest_record_size: u32,
    pub data_transfer_handle_timeout: u8,
}

#[derive(Debug, Clone, FromBytes, IntoBytes, Immutable, PartialEq)]
#[repr(C, packed)]
pub struct GetPdrRepositoryInfoResponse {
    pub hdr: PldmMsgHeader<[u8; PLDM_MSG_HEADER_LEN]>,
    pub completion_code: u8,
    /// One of [`PdrRepositoryState`].
    pub repository_state: u8,
    pub update_time: Timestamp104,
    pub oem_update_time: Timestamp104,
    pub record_count: u32,
    pub repository_size: u32,
    pub largest_record_size: u32,
    /// Seconds a `dataTransferHandle` stays valid; see the constants above.
    pub data_transfer_handle_timeout: u8,
}

impl GetPdrRepositoryInfoResponse {
    pub fn new(instance_id: InstanceId, completion_code: u8, info: &PdrRepositoryInfo) -> Self {
        GetPdrRepositoryInfoResponse {
            hdr: PldmMsgHeader::new(
                instance_id,
                PldmMsgType::Response,
                PldmSupportedType::Platform,
                PldmPlatformCmd::GetPdrRepositoryInfo as u8,
            ),
            completion_code,
            repository_state: info.repository_state as u8,
            update_time: info.update_time,
            oem_update_time: info.oem_update_time,
            record_count: info.record_count,
            repository_size: info.repository_size,
            largest_record_size: info.largest_record_size,
            data_transfer_handle_timeout: info.data_transfer_handle_timeout,
        }
    }

    /// Converts the wire byte back to the named state.
    ///
    /// Errors on a value outside Table 68, which a conforming responder never
    /// emits but a peer might.
    pub fn repository_state(&self) -> Result<PdrRepositoryState, PldmError> {
        PdrRepositoryState::try_from(self.repository_state)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::codec::{PldmCodec, PldmCodecError};

    fn sample_info() -> PdrRepositoryInfo {
        let mut update_time = Timestamp104::unknown();
        update_time.year = 2026;
        update_time.month = 8;
        update_time.day = 12;
        PdrRepositoryInfo {
            repository_state: PdrRepositoryState::Available,
            update_time,
            oem_update_time: Timestamp104::unknown(),
            record_count: 4,
            repository_size: 96,
            largest_record_size: 32,
            data_transfer_handle_timeout: DATA_TRANSFER_HANDLE_TIMEOUT_DEFAULT,
        }
    }

    #[test]
    fn test_request_carries_platform_type_and_opcode() {
        let request = GetPdrRepositoryInfoRequest::new(3, PldmMsgType::Request);
        assert_eq!(request.hdr.pldm_type(), PldmSupportedType::Platform as u8);
        assert_eq!(
            request.hdr.cmd_code(),
            PldmPlatformCmd::GetPdrRepositoryInfo as u8
        );
        assert!(request.hdr.is_request());
    }

    #[test]
    fn test_request_roundtrip() {
        let request = GetPdrRepositoryInfoRequest::new(1, PldmMsgType::Request);
        let mut buffer = [0u8; PLDM_MSG_HEADER_LEN];
        assert_eq!(request.encode(&mut buffer).unwrap(), PLDM_MSG_HEADER_LEN);
        assert_eq!(
            GetPdrRepositoryInfoRequest::decode(&buffer).unwrap(),
            request
        );
    }

    #[test]
    fn test_response_roundtrip() {
        let info = sample_info();
        let response = GetPdrRepositoryInfoResponse::new(1, 0, &info);

        let mut buffer = [0u8; GET_PDR_REPOSITORY_INFO_RESPONSE_LEN];
        assert_eq!(
            response.encode(&mut buffer).unwrap(),
            GET_PDR_REPOSITORY_INFO_RESPONSE_LEN
        );

        let decoded = GetPdrRepositoryInfoResponse::decode(&buffer).unwrap();
        assert_eq!(decoded, response);
        assert_eq!(
            decoded.repository_state().unwrap(),
            PdrRepositoryState::Available
        );
        // Copied out first: a reference into a packed struct is undefined
        // behaviour, and `assert_eq!` takes one.
        let record_count = decoded.record_count;
        assert_eq!(record_count, 4);
    }

    #[test]
    fn test_response_is_44_bytes_on_the_wire() {
        // Table 68: 3 header + 1 completion + 1 state + 13 + 13 timestamps
        // + 4 + 4 + 4 counts + 1 timeout.
        assert_eq!(GET_PDR_REPOSITORY_INFO_RESPONSE_LEN, 44);
    }

    #[test]
    fn test_response_rejects_short_buffer() {
        let response = GetPdrRepositoryInfoResponse::new(1, 0, &sample_info());
        let mut buffer = [0u8; GET_PDR_REPOSITORY_INFO_RESPONSE_LEN - 1];
        assert_eq!(
            response.encode(&mut buffer),
            Err(PldmCodecError::BufferTooShort)
        );
    }

    #[test]
    fn test_response_rejects_illegal_repository_state() {
        let mut response = GetPdrRepositoryInfoResponse::new(1, 0, &sample_info());
        // 3 is one past `failed`, the last value Table 68 defines.
        response.repository_state = 3;
        assert!(response.repository_state().is_err());
    }
}
