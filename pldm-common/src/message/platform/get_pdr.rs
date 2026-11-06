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

//! GetPDR, command code 0x51, DSP0248 clause 26.2, Table 69.

use crate::codec::{PldmCodec, PldmCodecError, PldmCodecWithLifetime};
use crate::error::PldmError;
use crate::protocol::base::{
    InstanceId, PldmBaseCompletionCode, PldmFailureResponse, PldmMsgHeader, PldmMsgType,
    PldmSupportedType, PLDM_FAILURE_RESP_LEN, PLDM_MSG_HEADER_LEN,
};
use crate::protocol::platform::{
    PlatformTransferFlag, PlatformTransferOperationFlag, PldmPlatformCmd,
};
use zerocopy::{FromBytes, Immutable, IntoBytes};

/// Encoded length of the request, DSP0248 Table 69.
///
/// Taken from the packed struct, whose layout is the wire layout. The test
/// below pins that layout to the byte count Table 69 prints.
pub const GET_PDR_REQUEST_LEN: usize = core::mem::size_of::<GetPdrRequest>();

/// Length of a success response before `recordData`, DSP0248 Table 69.
pub const GET_PDR_RESPONSE_FIXED_LEN: usize = core::mem::size_of::<GetPdrResponseFixed>();

/// `recordHandle` asking for the first PDR in the repository.
pub const RECORD_HANDLE_FIRST: u32 = 0x0000_0000;

/// `nextRecordHandle` meaning no PDR follows this one.
pub const RECORD_HANDLE_NONE: u32 = 0x0000_0000;

/// `dataTransferHandle` used with `GetFirstPart`, and returned when no record
/// data remains.
pub const DATA_TRANSFER_HANDLE_NONE: u32 = 0x0000_0000;

/// The largest `recordData` a response can carry in a buffer of `buffer_len`.
///
/// The `transferCRC` byte is reserved whether or not this part carries one,
/// because the part that completes a record is not known to be the last until
/// its length is chosen. DSP0248 Table 69 lets a responder return fewer bytes
/// than `requestCount` asks for, which is what makes this bound legal.
pub const fn max_record_data_len(buffer_len: usize) -> usize {
    buffer_len.saturating_sub(GET_PDR_RESPONSE_FIXED_LEN + core::mem::size_of::<u8>())
}

#[derive(Debug, Clone, FromBytes, IntoBytes, Immutable, PartialEq)]
#[repr(C, packed)]
pub struct GetPdrRequest {
    pub hdr: PldmMsgHeader<[u8; PLDM_MSG_HEADER_LEN]>,
    /// [`RECORD_HANDLE_FIRST`] asks for the first record.
    pub record_handle: u32,
    /// [`DATA_TRANSFER_HANDLE_NONE`] when `transfer_operation_flag` is
    /// `GetFirstPart`.
    pub data_transfer_handle: u32,
    /// One of [`PlatformTransferOperationFlag`]; GetPDR accepts the first two.
    pub transfer_operation_flag: u8,
    /// Maximum `recordData` bytes wanted back. The responder may return fewer.
    pub request_count: u16,
    /// Zero on `GetFirstPart`; the value from the first part on `GetNextPart`.
    pub record_change_number: u16,
}

impl GetPdrRequest {
    pub fn new(
        instance_id: InstanceId,
        message_type: PldmMsgType,
        record_handle: u32,
        data_transfer_handle: u32,
        transfer_operation_flag: PlatformTransferOperationFlag,
        request_count: u16,
        record_change_number: u16,
    ) -> Self {
        GetPdrRequest {
            hdr: PldmMsgHeader::new(
                instance_id,
                message_type,
                PldmSupportedType::Platform,
                PldmPlatformCmd::GetPdr as u8,
            ),
            record_handle,
            data_transfer_handle,
            transfer_operation_flag: transfer_operation_flag as u8,
            request_count,
            record_change_number,
        }
    }

    /// Converts the wire byte back to the named operation.
    ///
    /// `AcknowledgementOnly` decodes successfully here because it is a legal
    /// Table 16 value; GetPDR rejects it, see [`super::pdr_transfer`].
    pub fn transfer_operation_flag(&self) -> Result<PlatformTransferOperationFlag, PldmError> {
        PlatformTransferOperationFlag::try_from(self.transfer_operation_flag)
    }
}

/// The part of a success response that precedes `recordData`.
///
/// Splitting the fixed prefix into a packed struct is the shape
/// `get_fw_params.rs` uses for its variable-length message: zerocopy places
/// every field, so the hand-written codec below computes no field offsets of
/// its own and the wire length comes from the type rather than from a sum of
/// literals.
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Immutable, PartialEq)]
#[repr(C, packed)]
pub struct GetPdrResponseFixed {
    pub hdr: PldmMsgHeader<[u8; PLDM_MSG_HEADER_LEN]>,
    pub completion_code: u8,
    pub next_record_handle: u32,
    pub next_data_transfer_handle: u32,
    pub transfer_flag: u8,
    pub response_count: u16,
}

/// GetPDR response, whose `recordData` borrows the record it carries.
///
/// `transfer_crc` is present exactly when `transfer_flag` is `End`, so a
/// single-part transfer answering `StartAndEnd` carries no CRC byte. Encoding
/// a mismatch between the two fails rather than shifting every following byte
/// in the requester's decoder.
#[derive(Debug, PartialEq)]
pub struct GetPdrResponse<'a> {
    pub hdr: PldmMsgHeader<[u8; PLDM_MSG_HEADER_LEN]>,
    pub completion_code: u8,
    /// Repeated unchanged across every part of one multipart transfer.
    pub next_record_handle: u32,
    /// [`DATA_TRANSFER_HANDLE_NONE`] once no record data remains.
    pub next_data_transfer_handle: u32,
    /// One of [`PlatformTransferFlag`].
    pub transfer_flag: u8,
    pub response_count: u16,
    pub record_data: &'a [u8],
    /// CRC-8 over the whole PDR, not over this part.
    pub transfer_crc: Option<u8>,
}

impl<'a> GetPdrResponse<'a> {
    /// # Errors
    ///
    /// Returns `InvalidData` when `record_data` is longer than the
    /// `responseCount` field can count. A narrowing cast would report a
    /// shorter record than the response carries.
    pub fn new(
        instance_id: InstanceId,
        next_record_handle: u32,
        next_data_transfer_handle: u32,
        transfer_flag: PlatformTransferFlag,
        record_data: &'a [u8],
        transfer_crc: Option<u8>,
    ) -> Result<Self, PldmCodecError> {
        let response_count =
            u16::try_from(record_data.len()).map_err(|_| PldmCodecError::InvalidData)?;
        Ok(GetPdrResponse {
            hdr: Self::response_header(instance_id),
            completion_code: PldmBaseCompletionCode::Success as u8,
            next_record_handle,
            next_data_transfer_handle,
            transfer_flag: transfer_flag as u8,
            response_count,
            record_data,
            transfer_crc,
        })
    }

    /// Builds the response that carries a completion code alone.
    pub fn new_error(instance_id: InstanceId, completion_code: u8) -> Self {
        Self::from_failure(Self::response_header(instance_id), completion_code)
    }

    pub fn transfer_flag(&self) -> Result<PlatformTransferFlag, PldmError> {
        PlatformTransferFlag::try_from(self.transfer_flag)
    }

    fn response_header(instance_id: InstanceId) -> PldmMsgHeader<[u8; PLDM_MSG_HEADER_LEN]> {
        PldmMsgHeader::new(
            instance_id,
            PldmMsgType::Response,
            PldmSupportedType::Platform,
            PldmPlatformCmd::GetPdr as u8,
        )
    }

    fn from_failure(hdr: PldmMsgHeader<[u8; PLDM_MSG_HEADER_LEN]>, completion_code: u8) -> Self {
        GetPdrResponse {
            hdr,
            completion_code,
            next_record_handle: RECORD_HANDLE_NONE,
            next_data_transfer_handle: DATA_TRANSFER_HANDLE_NONE,
            transfer_flag: PlatformTransferFlag::Start as u8,
            response_count: 0,
            record_data: &[],
            transfer_crc: None,
        }
    }

    fn is_success(&self) -> bool {
        self.completion_code == PldmBaseCompletionCode::Success as u8
    }

    fn codec_size_in_bytes(&self) -> usize {
        if !self.is_success() {
            return PLDM_FAILURE_RESP_LEN;
        }
        GET_PDR_RESPONSE_FIXED_LEN
            + self.record_data.len()
            + usize::from(self.transfer_crc.is_some())
    }
}

impl<'a> PldmCodecWithLifetime<'a> for GetPdrResponse<'a> {
    fn encode(&self, buffer: &mut [u8]) -> Result<usize, PldmCodecError> {
        if buffer.len() < self.codec_size_in_bytes() {
            return Err(PldmCodecError::BufferTooShort);
        }

        if !self.is_success() {
            let failure = PldmFailureResponse {
                hdr: self.hdr,
                completion_code: self.completion_code,
            };
            return failure.encode(buffer);
        }

        let carries_crc = self
            .transfer_flag()
            .map_err(|_| PldmCodecError::Unsupported)?
            .carries_transfer_crc();
        if carries_crc != self.transfer_crc.is_some() {
            return Err(PldmCodecError::InvalidData);
        }
        if usize::from(self.response_count) != self.record_data.len() {
            return Err(PldmCodecError::InvalidData);
        }

        let fixed = GetPdrResponseFixed {
            hdr: self.hdr,
            completion_code: self.completion_code,
            next_record_handle: self.next_record_handle,
            next_data_transfer_handle: self.next_data_transfer_handle,
            transfer_flag: self.transfer_flag,
            response_count: self.response_count,
        };
        let mut offset = fixed.encode(buffer)?;
        buffer[offset..offset + self.record_data.len()].copy_from_slice(self.record_data);
        offset += self.record_data.len();
        if let Some(crc) = self.transfer_crc {
            buffer[offset] = crc;
            offset += 1;
        }
        Ok(offset)
    }

    fn decode(buffer: &'a [u8]) -> Result<Self, PldmCodecError> {
        // The header and completion code are the whole of a failed response
        // and the start of a successful one, so decoding the failure shape
        // first answers which of the two this is.
        let failure = PldmFailureResponse::decode(buffer)?;
        if failure.completion_code != PldmBaseCompletionCode::Success as u8 {
            return Ok(Self::from_failure(failure.hdr, failure.completion_code));
        }

        let fixed = GetPdrResponseFixed::decode(buffer)?;
        let response_count = fixed.response_count;
        let record_data = buffer
            .get(
                GET_PDR_RESPONSE_FIXED_LEN
                    ..GET_PDR_RESPONSE_FIXED_LEN + usize::from(response_count),
            )
            .ok_or(PldmCodecError::BufferTooShort)?;

        let transfer_flag = fixed.transfer_flag;
        let transfer_crc = if PlatformTransferFlag::try_from(transfer_flag)
            .map_err(|_| PldmCodecError::Unsupported)?
            .carries_transfer_crc()
        {
            Some(
                *buffer
                    .get(GET_PDR_RESPONSE_FIXED_LEN + record_data.len())
                    .ok_or(PldmCodecError::BufferTooShort)?,
            )
        } else {
            None
        };

        Ok(GetPdrResponse {
            hdr: fixed.hdr,
            completion_code: fixed.completion_code,
            next_record_handle: fixed.next_record_handle,
            next_data_transfer_handle: fixed.next_data_transfer_handle,
            transfer_flag,
            response_count,
            record_data,
            transfer_crc,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::protocol::platform::GetPdrCompletionCode;

    const RECORD: [u8; 8] = [0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88];
    /// Length of the tail a multipart transfer would deliver last.
    const LAST_PART_LEN: usize = RECORD.len() / 2;
    const TRANSFER_CRC: u8 = 0x5A;

    /// Offset of `nextRecordHandle` within a success response, in Table 69
    /// field order. Only a test addresses a single field by position.
    const NEXT_RECORD_HANDLE_OFFSET: usize = PLDM_MSG_HEADER_LEN + core::mem::size_of::<u8>();

    /// Offset of `transferFlag`, two handles further along.
    const TRANSFER_FLAG_OFFSET: usize = NEXT_RECORD_HANDLE_OFFSET + 2 * core::mem::size_of::<u32>();

    fn single_part_response() -> GetPdrResponse<'static> {
        GetPdrResponse::new(
            1,
            RECORD_HANDLE_NONE,
            DATA_TRANSFER_HANDLE_NONE,
            PlatformTransferFlag::StartAndEnd,
            &RECORD,
            None,
        )
        .unwrap()
    }

    #[test]
    fn test_request_roundtrip() {
        let request = GetPdrRequest::new(
            2,
            PldmMsgType::Request,
            0x0000_0007,
            DATA_TRANSFER_HANDLE_NONE,
            PlatformTransferOperationFlag::GetFirstPart,
            64,
            0,
        );

        let mut buffer = [0u8; GET_PDR_REQUEST_LEN];
        assert_eq!(request.encode(&mut buffer).unwrap(), GET_PDR_REQUEST_LEN);

        let decoded = GetPdrRequest::decode(&buffer).unwrap();
        assert_eq!(decoded, request);
        assert_eq!(
            decoded.transfer_operation_flag().unwrap(),
            PlatformTransferOperationFlag::GetFirstPart
        );
        assert_eq!(decoded.hdr.cmd_code(), PldmPlatformCmd::GetPdr as u8);
    }

    #[test]
    fn test_wire_lengths_match_the_specification() {
        // Table 69 request: 3 header + 4 recordHandle + 4 dataTransferHandle
        // + 1 transferOperationFlag + 2 requestCount + 2 recordChangeNumber.
        assert_eq!(GET_PDR_REQUEST_LEN, 16);
        // Table 69 response before recordData: 3 header + 1 completionCode
        // + 4 nextRecordHandle + 4 nextDataTransferHandle + 1 transferFlag
        // + 2 responseCount.
        assert_eq!(GET_PDR_RESPONSE_FIXED_LEN, 15);
        assert_eq!(TRANSFER_FLAG_OFFSET, 12);
    }

    #[test]
    fn test_multibyte_response_fields_are_little_endian() {
        // The codec places fields by struct layout rather than by explicit
        // byte order, so this pins the layout to what DSP0240 clause 6
        // requires: least significant byte first.
        let response = GetPdrResponse::new(
            1,
            0x0403_0201,
            DATA_TRANSFER_HANDLE_NONE,
            PlatformTransferFlag::StartAndEnd,
            &RECORD,
            None,
        )
        .unwrap();

        let mut buffer = [0u8; GET_PDR_RESPONSE_FIXED_LEN + RECORD.len()];
        response.encode(&mut buffer).unwrap();
        assert_eq!(
            &buffer[NEXT_RECORD_HANDLE_OFFSET..NEXT_RECORD_HANDLE_OFFSET + 4],
            &[0x01, 0x02, 0x03, 0x04]
        );
    }

    #[test]
    fn test_request_rejects_illegal_transfer_operation_flag() {
        let mut request = GetPdrRequest::new(
            0,
            PldmMsgType::Request,
            1,
            0,
            PlatformTransferOperationFlag::GetFirstPart,
            8,
            0,
        );
        // 3 is one past acknowledgementOnly, the last Table 16 value.
        request.transfer_operation_flag = 3;
        assert!(request.transfer_operation_flag().is_err());
    }

    #[test]
    fn test_single_part_response_carries_no_crc() {
        let response = single_part_response();

        let mut buffer = [0u8; GET_PDR_RESPONSE_FIXED_LEN + RECORD.len()];
        let written = response.encode(&mut buffer).unwrap();
        // No CRC byte trails a StartAndEnd transfer, so the encoded length is
        // exactly the fixed part plus the record.
        assert_eq!(written, GET_PDR_RESPONSE_FIXED_LEN + RECORD.len());

        let decoded = GetPdrResponse::decode(&buffer[..written]).unwrap();
        assert_eq!(decoded, response);
        assert_eq!(decoded.record_data, &RECORD);
        assert_eq!(decoded.transfer_crc, None);
    }

    #[test]
    fn test_end_part_response_carries_crc() {
        let response = GetPdrResponse::new(
            1,
            0x0000_0002,
            DATA_TRANSFER_HANDLE_NONE,
            PlatformTransferFlag::End,
            &RECORD[RECORD.len() - LAST_PART_LEN..],
            Some(TRANSFER_CRC),
        )
        .unwrap();

        let mut buffer = [0u8; GET_PDR_RESPONSE_FIXED_LEN + LAST_PART_LEN + 1];
        let written = response.encode(&mut buffer).unwrap();
        // The CRC byte adds one to the fixed part plus the record bytes.
        assert_eq!(written, buffer.len());
        assert_eq!(buffer[written - 1], TRANSFER_CRC);

        let decoded = GetPdrResponse::decode(&buffer[..written]).unwrap();
        assert_eq!(decoded, response);
        assert_eq!(decoded.transfer_crc, Some(TRANSFER_CRC));
    }

    #[test]
    fn test_encode_rejects_crc_on_a_flag_that_does_not_carry_one() {
        let mut response = single_part_response();
        response.transfer_crc = Some(TRANSFER_CRC);
        let mut buffer = [0u8; GET_PDR_RESPONSE_FIXED_LEN + RECORD.len() + 1];
        assert_eq!(
            response.encode(&mut buffer),
            Err(PldmCodecError::InvalidData)
        );
    }

    #[test]
    fn test_encode_rejects_end_flag_without_crc() {
        let mut response = single_part_response();
        response.transfer_flag = PlatformTransferFlag::End as u8;
        let mut buffer = [0u8; GET_PDR_RESPONSE_FIXED_LEN + RECORD.len() + 1];
        assert_eq!(
            response.encode(&mut buffer),
            Err(PldmCodecError::InvalidData)
        );
    }

    #[test]
    fn test_encode_rejects_short_buffer() {
        let response = single_part_response();
        let mut buffer = [0u8; GET_PDR_RESPONSE_FIXED_LEN + RECORD.len() - 1];
        assert_eq!(
            response.encode(&mut buffer),
            Err(PldmCodecError::BufferTooShort)
        );
    }

    #[test]
    fn test_zero_length_record_data_encodes_the_fixed_part_only() {
        // requestCount 0 is legal and answers responseCount 0 with no
        // recordData field at all, see Table 69.
        let response = GetPdrResponse::new(
            1,
            RECORD_HANDLE_NONE,
            0x0000_0001,
            PlatformTransferFlag::Start,
            &[],
            None,
        )
        .unwrap();
        let mut buffer = [0u8; GET_PDR_RESPONSE_FIXED_LEN];
        let written = response.encode(&mut buffer).unwrap();
        assert_eq!(written, GET_PDR_RESPONSE_FIXED_LEN);

        let decoded = GetPdrResponse::decode(&buffer[..written]).unwrap();
        assert_eq!(decoded.response_count, 0);
        assert!(decoded.record_data.is_empty());
    }

    #[test]
    fn test_record_data_too_long_to_count_is_rejected() {
        // One byte past what responseCount can express. A narrowing cast
        // would wrap this to zero and report an empty record. A static
        // rather than a const, so the 64 KiB lives in one place instead of
        // being materialised at each use.
        static OVERSIZE: [u8; u16::MAX as usize + 1] = [0; u16::MAX as usize + 1];

        assert_eq!(
            GetPdrResponse::new(
                1,
                RECORD_HANDLE_NONE,
                DATA_TRANSFER_HANDLE_NONE,
                PlatformTransferFlag::StartAndEnd,
                &OVERSIZE,
                None,
            ),
            Err(PldmCodecError::InvalidData)
        );
    }

    #[test]
    fn test_error_response_stops_after_the_completion_code() {
        let response =
            GetPdrResponse::new_error(1, GetPdrCompletionCode::InvalidRecordHandle as u8);

        let mut buffer = [0u8; GET_PDR_RESPONSE_FIXED_LEN];
        let written = response.encode(&mut buffer).unwrap();
        // The requester must not read the handles a success response carries.
        assert_eq!(written, PLDM_FAILURE_RESP_LEN);

        let decoded = GetPdrResponse::decode(&buffer[..written]).unwrap();
        assert_eq!(decoded, response);
        assert_eq!(
            decoded.completion_code,
            GetPdrCompletionCode::InvalidRecordHandle as u8
        );
    }

    #[test]
    fn test_decode_rejects_truncated_record_data() {
        let response = single_part_response();
        let mut buffer = [0u8; GET_PDR_RESPONSE_FIXED_LEN + RECORD.len()];
        let written = response.encode(&mut buffer).unwrap();
        assert_eq!(
            GetPdrResponse::decode(&buffer[..written - 1]),
            Err(PldmCodecError::BufferTooShort)
        );
    }

    #[test]
    fn test_decode_rejects_illegal_transfer_flag() {
        let response = single_part_response();
        let mut buffer = [0u8; GET_PDR_RESPONSE_FIXED_LEN + RECORD.len()];
        let written = response.encode(&mut buffer).unwrap();
        // 0x02 is the DSP0267 Middle value, which DSP0248 does not define.
        buffer[TRANSFER_FLAG_OFFSET] = 0x02;
        assert_eq!(
            GetPdrResponse::decode(&buffer[..written]),
            Err(PldmCodecError::Unsupported)
        );
    }
}
