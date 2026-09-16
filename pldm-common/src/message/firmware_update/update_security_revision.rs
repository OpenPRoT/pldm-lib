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
    InstanceId, PldmMsgHeader, PldmMsgType, PldmSupportedType, PLDM_MSG_HEADER_LEN,
};
use crate::protocol::firmware_update::{ComponentClassification, FwUpdateCmd};
use zerocopy::{FromBytes, Immutable, IntoBytes};

/// Value for `comp_classification_index` that targets one downstream device.
/// Only meaningful when the classification is
/// [`ComponentClassification::DownstreamDevice`].
pub const UPDATE_SECURITY_REVISION_SINGLE_DEVICE: u8 = 0x00;

/// Value for `comp_classification_index` that targets every downstream device
/// sharing the descriptors of the one named by `comp_identifier`. Only
/// meaningful when the classification is
/// [`ComponentClassification::DownstreamDevice`].
pub const UPDATE_SECURITY_REVISION_ALL_DEVICES: u8 = 0xFF;

/// UpdateSecurityRevision request, DSP0267 1.3.0 section 12.19.
///
/// The UA sends this to commit the security revision of a component image it
/// transferred with the Security Revision Number Delayed Update option set.
/// Until it arrives the image runs at the old revision floor and a downgrade
/// is still allowed, so the UA can test the image first. The FD only accepts
/// the command in the IDLE state, and it acts on the active running image,
/// not a pending one.
#[derive(Debug, Clone, FromBytes, IntoBytes, Immutable, PartialEq)]
#[repr(C, packed)]
pub struct UpdateSecurityRevisionRequest {
    pub hdr: PldmMsgHeader<[u8; PLDM_MSG_HEADER_LEN]>,
    pub comp_classification: u16,
    /// Component identifier, or the downstream device index when
    /// `comp_classification` is `0xFFFF`.
    pub comp_identifier: u16,
    /// Distinguishes identical components sharing a classification and
    /// identifier. When `comp_classification` is `0xFFFF` it instead selects
    /// how many devices the command covers, one of
    /// [`UPDATE_SECURITY_REVISION_SINGLE_DEVICE`] or
    /// [`UPDATE_SECURITY_REVISION_ALL_DEVICES`].
    pub comp_classification_index: u8,
}

impl UpdateSecurityRevisionRequest {
    pub fn new(
        instance_id: InstanceId,
        msg_type: PldmMsgType,
        comp_classification: ComponentClassification,
        comp_identifier: u16,
        comp_classification_index: u8,
    ) -> UpdateSecurityRevisionRequest {
        UpdateSecurityRevisionRequest {
            hdr: PldmMsgHeader::new(
                instance_id,
                msg_type,
                PldmSupportedType::FwUpdate,
                FwUpdateCmd::UpdateSecurityRevision as u8,
            ),
            comp_classification: comp_classification as u16,
            comp_identifier,
            comp_classification_index,
        }
    }
}

/// UpdateSecurityRevision response, DSP0267 1.3.0 section 12.19.
///
/// Beyond the PLDM base codes the command returns `INVALID_STATE_FOR_COMMAND`
/// when the FD is not IDLE, and `UPDATE_SECURITY_REVISION_NOT_PERMITTED` when
/// it does not support committing the revision of that component image.
#[derive(Debug, Clone, FromBytes, IntoBytes, Immutable, PartialEq)]
#[repr(C, packed)]
pub struct UpdateSecurityRevisionResponse {
    pub hdr: PldmMsgHeader<[u8; PLDM_MSG_HEADER_LEN]>,
    pub completion_code: u8,
}

impl UpdateSecurityRevisionResponse {
    pub fn new(instance_id: InstanceId, completion_code: u8) -> UpdateSecurityRevisionResponse {
        UpdateSecurityRevisionResponse {
            hdr: PldmMsgHeader::new(
                instance_id,
                PldmMsgType::Response,
                PldmSupportedType::FwUpdate,
                FwUpdateCmd::UpdateSecurityRevision as u8,
            ),
            completion_code,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::codec::{PldmCodec, PldmCodecError};

    #[test]
    fn test_update_security_revision_request() {
        let request = UpdateSecurityRevisionRequest::new(
            1,
            PldmMsgType::Request,
            ComponentClassification::Firmware,
            0x0002,
            0,
        );

        let mut buffer = [0u8; core::mem::size_of::<UpdateSecurityRevisionRequest>()];
        request.encode(&mut buffer).unwrap();

        let decoded_request = UpdateSecurityRevisionRequest::decode(&buffer).unwrap();
        assert_eq!(request, decoded_request);
    }

    #[test]
    fn test_update_security_revision_request_for_all_downstream_devices() {
        let request = UpdateSecurityRevisionRequest::new(
            1,
            PldmMsgType::Request,
            ComponentClassification::DownstreamDevice,
            0x0003,
            UPDATE_SECURITY_REVISION_ALL_DEVICES,
        );

        let mut buffer = [0u8; core::mem::size_of::<UpdateSecurityRevisionRequest>()];
        request.encode(&mut buffer).unwrap();

        let decoded_request = UpdateSecurityRevisionRequest::decode(&buffer).unwrap();
        assert_eq!(request, decoded_request);
        let classification = decoded_request.comp_classification;
        assert_eq!(
            classification,
            ComponentClassification::DownstreamDevice as u16
        );
    }

    #[test]
    fn test_update_security_revision_request_rejects_a_short_buffer() {
        let request = UpdateSecurityRevisionRequest::new(
            1,
            PldmMsgType::Request,
            ComponentClassification::Firmware,
            0x0002,
            0,
        );

        let mut buffer = [0u8; core::mem::size_of::<UpdateSecurityRevisionRequest>() - 1];
        assert_eq!(
            request.encode(&mut buffer),
            Err(PldmCodecError::BufferTooShort)
        );
        assert_eq!(
            UpdateSecurityRevisionRequest::decode(&buffer),
            Err(PldmCodecError::BufferTooShort)
        );
    }

    #[test]
    fn test_update_security_revision_response() {
        let response = UpdateSecurityRevisionResponse::new(1, 0);

        let mut buffer = [0u8; core::mem::size_of::<UpdateSecurityRevisionResponse>()];
        response.encode(&mut buffer).unwrap();

        let decoded_response = UpdateSecurityRevisionResponse::decode(&buffer).unwrap();
        assert_eq!(response, decoded_response);
    }
}
