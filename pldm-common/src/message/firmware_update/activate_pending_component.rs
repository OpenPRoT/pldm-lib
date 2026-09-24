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
    InstanceId, PldmBaseCompletionCode, PldmMsgHeader, PldmMsgType, PldmSupportedType,
    PLDM_MSG_HEADER_LEN,
};
use crate::protocol::firmware_update::{
    ComponentClassification, FwUpdateCmd, FwUpdateCompletionCode,
};
use zerocopy::{FromBytes, Immutable, IntoBytes};

/// Component named by an `ActivatePendingComponentImage` request.
///
/// Carries only the three request fields. `classification` stays a raw `u16`:
/// it is passed through as received, and `ComponentClassification` does not
/// name every value that can arrive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingComponent {
    pub classification: u16,
    pub identifier: u16,
    /// With a downstream device, 0x00 selects one device and 0xFF selects every
    /// device whose descriptors match.
    pub classification_index: u8,
}

impl PendingComponent {
    pub fn new(classification: u16, identifier: u16, classification_index: u8) -> Self {
        Self {
            classification,
            identifier,
            classification_index,
        }
    }

    /// The downstream device index, if the classification is 0xFFFF. In that
    /// case `identifier` is a device index, not a component identifier.
    pub fn downstream_device_index(&self) -> Option<u16> {
        (self.classification == ComponentClassification::DownstreamDevice as u16)
            .then_some(self.identifier)
    }
}

/// Outcome of activating a pending component image, DSP0267 Table 44.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingComponentResult {
    /// Activation started. The value is the estimated time in seconds.
    Activated(u16),
    /// The component has no pending image.
    ActivationNotRequired,
    /// The FD does not activate pending images.
    NotPermitted,
}

impl PendingComponentResult {
    /// Completion code and estimated time for the response. The time is zero
    /// unless activation started.
    pub fn to_response_fields(self) -> (u8, u16) {
        match self {
            PendingComponentResult::Activated(estimated_time) => {
                (PldmBaseCompletionCode::Success as u8, estimated_time)
            }
            PendingComponentResult::ActivationNotRequired => {
                (FwUpdateCompletionCode::ActivationNotRequired as u8, 0)
            }
            PendingComponentResult::NotPermitted => (
                FwUpdateCompletionCode::ActivatePendingImageNotPermitted as u8,
                0,
            ),
        }
    }
}

#[derive(Debug, Clone, FromBytes, IntoBytes, Immutable, PartialEq)]
#[repr(C, packed)]
pub struct ActivatePendingComponentRequest {
    pub hdr: PldmMsgHeader<[u8; PLDM_MSG_HEADER_LEN]>,
    pub comp_classification: u16,
    pub comp_identifier: u16,
    pub comp_classification_index: u8,
}

impl ActivatePendingComponentRequest {
    pub fn new(
        instance_id: InstanceId,
        msg_type: PldmMsgType,
        comp_classification: ComponentClassification,
        comp_identifier: u16,
        comp_classification_index: u8,
    ) -> ActivatePendingComponentRequest {
        ActivatePendingComponentRequest {
            hdr: PldmMsgHeader::new(
                instance_id,
                msg_type,
                PldmSupportedType::FwUpdate,
                FwUpdateCmd::ActivatePendingComponentImage as u8,
            ),
            comp_classification: comp_classification as u16,
            comp_identifier,
            comp_classification_index,
        }
    }
}

#[derive(Debug, Clone, FromBytes, IntoBytes, Immutable, PartialEq)]
#[repr(C, packed)]
pub struct ActivatePendingComponentResponse {
    pub hdr: PldmMsgHeader<[u8; PLDM_MSG_HEADER_LEN]>,
    pub completion_code: u8,
    pub estimated_time_activation: u16,
}

impl ActivatePendingComponentResponse {
    pub fn new(
        instance_id: InstanceId,
        completion_code: u8,
        estimated_time_activation: u16,
    ) -> ActivatePendingComponentResponse {
        ActivatePendingComponentResponse {
            hdr: PldmMsgHeader::new(
                instance_id,
                PldmMsgType::Response,
                PldmSupportedType::FwUpdate,
                FwUpdateCmd::ActivatePendingComponentImage as u8,
            ),
            completion_code,
            estimated_time_activation,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::PldmCodec;

    #[test]
    fn test_activate_pending_component() {
        let request = ActivatePendingComponentRequest::new(
            1,
            PldmMsgType::Request,
            ComponentClassification::Firmware,
            2,
            3,
        );

        let mut buffer = [0u8; 64];
        let bytes_written = request.encode(&mut buffer).unwrap();
        assert_eq!(
            bytes_written,
            core::mem::size_of::<ActivatePendingComponentRequest>()
        );
        let decoded_request =
            ActivatePendingComponentRequest::decode(&buffer[..bytes_written]).unwrap();
        assert_eq!(request, decoded_request);
    }

    #[test]
    fn test_pending_component_downstream_device_index() {
        let firmware = PendingComponent::new(ComponentClassification::Firmware as u16, 7, 0);
        assert_eq!(firmware.downstream_device_index(), None);

        let downstream =
            PendingComponent::new(ComponentClassification::DownstreamDevice as u16, 7, 0xFF);
        assert_eq!(downstream.downstream_device_index(), Some(7));
    }

    #[test]
    fn test_pending_component_result_response_fields() {
        assert_eq!(
            PendingComponentResult::Activated(42).to_response_fields(),
            (PldmBaseCompletionCode::Success as u8, 42)
        );
        assert_eq!(
            PendingComponentResult::ActivationNotRequired.to_response_fields(),
            (FwUpdateCompletionCode::ActivationNotRequired as u8, 0)
        );
        assert_eq!(
            PendingComponentResult::NotPermitted.to_response_fields(),
            (
                FwUpdateCompletionCode::ActivatePendingImageNotPermitted as u8,
                0
            )
        );
    }

    #[test]
    fn test_activate_pending_component_response() {
        let response = ActivatePendingComponentResponse::new(1, 0, 0);
        let mut buffer = [0u8; 64];
        let bytes_written = response.encode(&mut buffer).unwrap();
        assert_eq!(
            bytes_written,
            core::mem::size_of::<ActivatePendingComponentResponse>()
        );
        let decoded_response =
            ActivatePendingComponentResponse::decode(&buffer[..bytes_written]).unwrap();
        assert_eq!(response, decoded_response);
    }
}
