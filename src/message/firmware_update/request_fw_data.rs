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

use crate::codec::{PldmCodecError, PldmCodecWithLifetime};
use crate::protocol::base::{
    InstanceId, PLDM_MSG_HEADER_LEN, PldmMsgHeader, PldmMsgType, PldmSupportedType,
};
use crate::protocol::firmware_update::FwUpdateCmd;
use zerocopy::{FromBytes, Immutable, IntoBytes};

pub const MAX_TRANSFER_SIZE: usize = 512; // Define an appropriate size

#[derive(Debug, Clone, FromBytes, IntoBytes, Immutable, PartialEq)]
#[repr(C, packed)]
pub struct RequestFirmwareDataRequest {
    pub hdr: PldmMsgHeader<[u8; PLDM_MSG_HEADER_LEN]>,
    pub offset: u32,
    pub length: u32,
}

impl RequestFirmwareDataRequest {
    pub fn new(
        instance_id: InstanceId,
        msg_type: PldmMsgType,
        offset: u32,
        length: u32,
    ) -> RequestFirmwareDataRequest {
        let hdr = PldmMsgHeader::new(
            instance_id,
            msg_type,
            PldmSupportedType::FwUpdate,
            FwUpdateCmd::RequestFirmwareData as u8,
        );
        RequestFirmwareDataRequest {
            hdr,
            offset,
            length,
        }
    }
}

#[derive(Debug, Clone, FromBytes, IntoBytes, Immutable, PartialEq)]
#[repr(C, packed)]
// TODO: this is calitpra code, but imho this structure is too clunky.
pub struct RequestFirmwareDataResponseFixed {
    pub hdr: PldmMsgHeader<[u8; PLDM_MSG_HEADER_LEN]>,
    pub completion_code: u8,
}

#[derive(Debug, Clone, PartialEq)]
#[repr(C)]
pub struct RequestFirmwareDataResponse<'a> {
    pub fixed: RequestFirmwareDataResponseFixed,
    pub data: &'a [u8],
}

impl<'a> RequestFirmwareDataResponse<'a> {
    pub fn new(
        instance_id: InstanceId,
        completion_code: u8,
        data: &'_ [u8],
    ) -> RequestFirmwareDataResponse<'_> {
        let fixed = RequestFirmwareDataResponseFixed {
            hdr: PldmMsgHeader::new(
                instance_id,
                PldmMsgType::Response,
                PldmSupportedType::FwUpdate,
                FwUpdateCmd::RequestFirmwareData as u8,
            ),
            completion_code,
        };
        RequestFirmwareDataResponse { fixed, data }
    }

    pub fn codec_size_in_bytes(&self) -> usize {
        let mut bytes = core::mem::size_of::<RequestFirmwareDataResponseFixed>();
        bytes += self.data.len();
        bytes
    }
}

impl<'a> PldmCodecWithLifetime<'a> for RequestFirmwareDataResponse<'a> {
    fn encode(&self, buffer: &mut [u8]) -> Result<usize, PldmCodecError> {
        if buffer.len() < self.codec_size_in_bytes() {
            return Err(PldmCodecError::BufferTooShort);
        }

        let mut offset = 0;
        let bytes = core::mem::size_of::<RequestFirmwareDataResponseFixed>();
        self.fixed
            .write_to(&mut buffer[offset..offset + bytes])
            .unwrap();
        offset += bytes;

        let data_len = self.data.len();
        if data_len > MAX_TRANSFER_SIZE {
            return Err(PldmCodecError::BufferTooShort);
        }
        buffer[offset..offset + data_len].copy_from_slice(self.data);
        Ok(bytes + data_len)
    }

    // Decoding is implemented for this struct. The caller should use the `length` field in the request to read the image portion data from the buffer.
    fn decode(buffer: &'a [u8]) -> Result<Self, PldmCodecError> {
        let size = core::mem::size_of::<RequestFirmwareDataResponseFixed>();
        if buffer.len() < size {
            return Err(PldmCodecError::BufferTooShort);
        }

        let (fixed, _) =
            RequestFirmwareDataResponseFixed::read_from_prefix(&buffer[..size]).unwrap();
        Ok(RequestFirmwareDataResponse {
            fixed,
            data: &buffer[size..],
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::codec::PldmCodec;

    #[test]
    fn test_request_firmware_data_request_codec() {
        let request = RequestFirmwareDataRequest::new(1, PldmMsgType::Request, 0, 64);
        let mut buffer = [0u8; 1024];
        let bytes = request.encode(&mut buffer).unwrap();

        let decoded_request = RequestFirmwareDataRequest::decode(&buffer[..bytes]).unwrap();
        assert_eq!(request, decoded_request);
    }

    #[test]
    fn test_request_firmware_data_response_codec() {
        let data = [0u8; 512];
        let response = RequestFirmwareDataResponse::new(1, 0, &data);
        let mut buffer = [0u8; 1024];
        let bytes = response.encode(&mut buffer).unwrap();

        let decoded_response = RequestFirmwareDataResponse::decode(&buffer[..bytes]).unwrap();
        assert_eq!(response, decoded_response);
    }
}
