// Licensed under the Apache-2.0 license

use crate::protocol::base::{
    InstanceId, PldmBaseCompletionCode, PldmMsgHeader, PldmMsgType, PldmSupportedType,
    TransferOperationFlag, PLDM_MSG_HEADER_LEN,
};

use crate::pldm_completion_code;

use crate::protocol::firmware_update::{
    ComponentActivationMethods, Descriptor, FwUpdateCmd, FwUpdateCompletionCode,
};
use bitfield::bitfield;
use zerocopy::{FromBytes, Immutable, IntoBytes};

/// QueryDownstreamDevices is used by the UA to obtain the firmware identifiers
/// for the downstream devices supported by the FDP. The entire list of all
/// attached downstream devices is provided by the response to
/// [QueryDownstreamIdentifiers] command. The FDP shall provide a response
/// message to this command in all states, including IDLE.
#[derive(Debug, Clone, FromBytes, IntoBytes, Immutable, PartialEq)]
#[repr(C, packed)]
pub struct QueryDownstreamDevicesRequest {
    pub hdr: PldmMsgHeader<[u8; PLDM_MSG_HEADER_LEN]>,
}

impl QueryDownstreamDevicesRequest {
    pub fn new(instance_id: InstanceId) -> Self {
        QueryDownstreamDevicesRequest {
            hdr: PldmMsgHeader::new(
                instance_id,
                PldmMsgType::Request,
                PldmSupportedType::FwUpdate,
                FwUpdateCmd::QueryDownstreamDevices as u8,
            ),
        }
    }
}

bitfield! {
    #[derive(Clone, Copy, FromBytes, IntoBytes, Immutable, PartialEq, Eq)]
    pub struct QueryDownstreamDevicesCapability(u32);
    impl Debug;
    pub u32, reserved, _: 31, 3;
    pub u32, update_simultaneous, set_update_simultaneous: 2;
    pub u32, dynamic_remove, set_dynamic_remove: 1;
    pub u32, dynamic_attach, set_dynamic_attach: 0;
}

#[derive(Debug, Clone, FromBytes, Immutable, PartialEq)]
pub struct QueryDownstreamDeviceResponse {
    pub hdr: PldmMsgHeader<[u8; PLDM_MSG_HEADER_LEN]>,

    /// PLDM_BASE_CODES
    pub completion_code: u8,
    pub downstream_device_update_supported: u8,
    pub number_of_downstream_devices: u16,
    pub max_number_of_downstream_devices: u16,
    pub capabilities: QueryDownstreamDevicesCapability,
}

impl QueryDownstreamDeviceResponse {
    pub fn new(
        instance_id: InstanceId,
        completion_code: PldmBaseCompletionCode,
        downstream_device_update_supported: u8,
        number_of_downstream_devices: u16,
        maximum_number_of_downstream_devices: u16,
    ) -> Self {
        QueryDownstreamDeviceResponse {
            hdr: PldmMsgHeader::new(
                instance_id,
                PldmMsgType::Response,
                PldmSupportedType::FwUpdate,
                FwUpdateCmd::QueryDownstreamDevices as u8,
            ),
            completion_code: completion_code as u8,
            downstream_device_update_supported,
            number_of_downstream_devices,
            max_number_of_downstream_devices: maximum_number_of_downstream_devices,
            capabilities: QueryDownstreamDevicesCapability(0),
        }
    }
}

#[derive(Debug, Clone, FromBytes, IntoBytes, Immutable, PartialEq)]
#[repr(C, packed)]
pub struct QueryDownstreamIdentifiersRequest {
    pub hdr: PldmMsgHeader<[u8; PLDM_MSG_HEADER_LEN]>,
    pub downstream_data_device_handle: u32,
    pub transfer_op_flag: u8,
}

impl QueryDownstreamIdentifiersRequest {
    pub fn new(
        instance_id: InstanceId,
        downstream_data_device_handle: u32,
        transfer_op_flag: TransferOperationFlag,
    ) -> Self {
        QueryDownstreamIdentifiersRequest {
            hdr: PldmMsgHeader::new(
                instance_id,
                PldmMsgType::Request,
                PldmSupportedType::FwUpdate,
                FwUpdateCmd::QueryDownstreamIdentifiers as u8,
            ),
            downstream_data_device_handle,
            transfer_op_flag: transfer_op_flag as u8,
        }
    }
}

// instead of using heapless::Vec, let's just allocate a fixed-size slices for now
pub const DOWNSTREAM_DEVICE_PORTION_COUNT: usize = 4;
pub const DOWNSTREAM_DEVICE_COUNT: usize = 8;
pub const DOWNSTREAM_DESCRIPTOR_COUNT: usize = 4;

pldm_completion_code! {
    QueryDownstreamIdentifiersResponseCode {
        InvalidTransferHandle,
        InvalidTransferOperationFlag,
        DownstreamDeviceListChanged,
    }
}

#[derive(Debug, Clone, PartialEq)]
#[repr(C)]
pub struct QueryDownstreamIdentifiersResponse {
    pub hdr: PldmMsgHeader<[u8; PLDM_MSG_HEADER_LEN]>,

    /// PLDM_BASE_CODES, INVALID_TRANSFER_HANDLE, INVALID_TRANSFER_OPERATION_FLAG,
    /// DOWNSTREAM_DEVICE_LIST_CHANGED
    ///
    /// See [QueryDownstreamIdentifiersResponseCode].
    pub completion_code: u8,
    pub next_data_transfer_handle: u32,
    pub transfer_flag: u8,

    /// QueryDownstreamIdentifiersResponsePortion
    ///
    /// If the FDP has negotiated a PartSize as defined by DSP0240 and its NegotiateTransferParameters
    /// command, then the maximum size for this field shall be equal to or less than that negotiated value.
    /// Otherwise the FDP can determine the size for this field.
    // TODO: check for PartSize and make this dynamic, for now make it static
    pub portions: Option<[QueryDownstreamIdentifiersPortion; DOWNSTREAM_DEVICE_PORTION_COUNT]>,
}

impl QueryDownstreamIdentifiersResponse {
    pub fn new(
        instance_id: InstanceId,
        completion_code: QueryDownstreamIdentifiersResponseCode,
        next_data_transfer_handle: u32,
        transfer_flag: u8,
        portions: Option<&[QueryDownstreamIdentifiersPortion; DOWNSTREAM_DEVICE_PORTION_COUNT]>,
    ) -> Self {
        let portions = portions.cloned();
        QueryDownstreamIdentifiersResponse {
            hdr: PldmMsgHeader::new(
                instance_id,
                PldmMsgType::Response,
                PldmSupportedType::FwUpdate,
                FwUpdateCmd::QueryDownstreamIdentifiers as u8,
            ),
            completion_code: completion_code.into(),
            next_data_transfer_handle,
            transfer_flag,
            portions,
        }
    }
}

// #[derive(Debug, Clone, FromBytes, IntoBytes, Immutable, PartialEq)]
#[derive(Debug, Clone, PartialEq)]
#[repr(C)]
pub struct QueryDownstreamIdentifiersPortion {
    pub downstream_devices_length: u32,
    pub number_of_downstream_devices: u16,
    // pub first_downstream_device_index: DownstreamDeviceIndex,
    pub first_downstream_device_index: u16,
    pub downstream_devices: Option<[DownstreamDevices; DOWNSTREAM_DEVICE_COUNT]>,
}

impl QueryDownstreamIdentifiersPortion {
    pub fn new(
        downstream_devices_length: u32,
        number_of_downstream_devices: u16,
        // first_downstream_device_index: DownstreamDeviceIndex,
        first_downstream_device_index: u16,
        downstream_devices: Option<[DownstreamDevices; DOWNSTREAM_DEVICE_COUNT]>,
    ) -> Self {
        QueryDownstreamIdentifiersPortion {
            downstream_devices_length,
            number_of_downstream_devices,
            first_downstream_device_index,
            downstream_devices,
        }
    }
}

// #[derive(Debug, Clone, FromBytes, IntoBytes, Immutable, PartialEq)]
// pub enum DownstreamDeviceIndex {
//     Index(u16), // 0x0000 – 0x0FFF = Downstream index number
//     Reserved,   // 0x1000 – 0xFFFF = Reserved
// }

// impl From<u16> for DownstreamDeviceIndex {
//     fn from(value: u16) -> Self {
//         if value <= 0x0FFF {
//             DownstreamDeviceIndex::Index(value)
//         } else {
//             DownstreamDeviceIndex::Reserved
//         }
//     }
// }

// #[derive(Debug, Clone, FromBytes, IntoBytes, Immutable, PartialEq)]
#[derive(Debug, Clone, PartialEq)]
#[repr(C)]
pub struct DownstreamDevices {
    pub downstream_device_index: u16,
    pub downstream_descriptor_count: u8,
    pub downstream_descriptors: Option<[DownstreamDescriptor; DOWNSTREAM_DESCRIPTOR_COUNT]>,
}

impl DownstreamDevices {
    pub fn new(
        downstream_device_index: u16,
        downstream_descriptor_count: u8,
        downstream_descriptors: Option<[DownstreamDescriptor; DOWNSTREAM_DESCRIPTOR_COUNT]>,
    ) -> Self {
        DownstreamDevices {
            downstream_device_index,
            downstream_descriptor_count,
            downstream_descriptors: downstream_descriptors.clone(),
        }
    }
}

// #[derive(Debug, Clone, FromBytes, IntoBytes, PartialEq)]
#[derive(Debug, Clone, PartialEq)]
#[repr(C, packed)]
pub struct DownstreamDescriptor {
    pub descriptor_type: u8,
    pub descriptor_length: u8,
    pub descriptor_data: Descriptor,
}

impl DownstreamDescriptor {
    pub fn new(descriptor_type: u8, descriptor_length: u8, descriptor_data: Descriptor) -> Self {
        DownstreamDescriptor {
            descriptor_type,
            descriptor_length,
            descriptor_data,
        }
    }
}

// #[derive(Debug, Clone, FromBytes, IntoBytes, Immutable, PartialEq)]
#[derive(Debug, Clone, PartialEq)]
#[repr(C, packed)]
pub struct GetDownstreamFirmwareParametersRequest {
    pub hdr: PldmMsgHeader<[u8; PLDM_MSG_HEADER_LEN]>,
    pub data_transfer_handle: u32,
    pub transfer_op_flag: u8,
}

impl GetDownstreamFirmwareParametersRequest {
    pub fn new(
        instance_id: InstanceId,
        data_transfer_handle: u32,
        transfer_op_flag: TransferOperationFlag,
    ) -> Self {
        GetDownstreamFirmwareParametersRequest {
            hdr: PldmMsgHeader::new(
                instance_id,
                PldmMsgType::Request,
                PldmSupportedType::FwUpdate,
                FwUpdateCmd::GetDownstreamFirmwareParameters as u8,
            ),
            data_transfer_handle,
            transfer_op_flag: transfer_op_flag as u8,
        }
    }
}

bitfield! {
    #[derive(Clone, Copy, FromBytes, IntoBytes, Immutable, PartialEq, Eq)]
    pub struct GetDownstreamFirmwareParametersCapability(u32);
    impl Debug;
    pub u32, reserved_0, _: 31, 10;
    pub u32, secuirty_revision_number, set_secuirty_revision_number: 9;
    pub u32, fdp_downgrade_restrictions, set_fdp_downgrade_restrictions: 8;
    pub u32, fdp_update_mode_restrictions, set_fdp_update_mode_restrictions: 7, 4;
    pub u32, reserved_1, _: 3;
    pub u32, downstream_device_host_functionality, set_downstream_device_host_functionality: 2;
    pub u32, component_update_failure_retry, set_component_update_failure_retry: 1;
    pub u32, downstream_device_component_update_failure, set_downstream_device_component_update_failure: 0;
}

pldm_completion_code! {
    GetDownstreamFirmwareParametersResponseCode {
        InvalidTransferHandle,
        InvalidTransferOperationFlag,
        DownstreamDeviceListChanged
    }
}

#[derive(Debug, Clone, PartialEq)]
#[repr(C)]
pub struct GetDownstreamFirmwareParametersResponse {
    pub hdr: PldmMsgHeader<[u8; PLDM_MSG_HEADER_LEN]>,

    /// PLDM_BASE_CODES, INVALID_TRANSFER_HANDLE, INVALID_TRANSFER_OPERATION_FLAG,
    /// DOWNSTREAM_DEVICE_LIST_CHANGED
    ///
    /// See [GetDownstreamFirmwareParametersResponseCode].
    pub completion_code: u8,
    pub next_data_transfer_handle: u32,
    pub transfer_flag: u8,

    /// GetDownstreamFirmwareParametersPortion
    ///
    /// If the FDP has negotiated a PartSize as defined by DSP0240 and its NegotiateTransferParameters
    /// command, then the maximum size for this field shall be equal to or less than that negotiated value.
    /// Otherwise the FDP can determine the size for this field.
    // TODO: check for PartSize and make this dynamic
    pub portions: Option<[GetDownstreamFirmwareParametersPortion; DOWNSTREAM_DEVICE_PORTION_COUNT]>,
}

// #[derive(Debug, Clone, FromBytes, IntoBytes, Immutable, PartialEq)]
#[derive(Debug, Clone, PartialEq)]
#[repr(C)]
pub struct GetDownstreamFirmwareParametersPortion {
    pub get_downstream_firmware_parameters_capability: GetDownstreamFirmwareParametersCapability,
    pub downstream_device_count: u16,
    pub downstream_device_parameter_table: Option<[DownstreamDeviceParameterTable; 8]>,
}

#[derive(Debug, Clone, PartialEq)]
#[repr(C)]
pub struct DownstreamDeviceParameterTable {
    pub downstream_device_index: u16,
    pub active_component_comparison_stamp: u32,
    pub active_component_version_string_type: u8, // See VersionStringType
    pub active_component_version_string_length: u8,
    pub active_component_release_date: [u8; 8], // YYYYMMDD
    pub pending_component_comparison_stamp: u32,
    pub pending_component_version_string_type: u8, // See VersionStringType
    pub pending_component_version_string_length: u8,
    pub pending_component_release_date: [u8; 8], // YYYYMMDD
    pub component_activation_methods: ComponentActivationMethods,

    // TODO: make this variable in length
    pub active_component_version_string: Option<[u8; 0xff]>,

    // TODO: make this variable in length
    // "If no pending firmware component exists, this field is zero bytes in length"
    pub pending_component_version_string: Option<[u8; 0xff]>,
}

impl GetDownstreamFirmwareParametersResponse {
    pub fn new(
        instance_id: InstanceId,
        completion_code: GetDownstreamFirmwareParametersResponseCode,
        next_data_transfer_handle: u32,
        transfer_flag: u8,
        portions: Option<[GetDownstreamFirmwareParametersPortion; DOWNSTREAM_DEVICE_PORTION_COUNT]>,
    ) -> Self {
        GetDownstreamFirmwareParametersResponse {
            hdr: PldmMsgHeader::new(
                instance_id,
                PldmMsgType::Response,
                PldmSupportedType::FwUpdate,
                FwUpdateCmd::GetDownstreamFirmwareParameters as u8,
            ),
            completion_code: completion_code.into(),
            next_data_transfer_handle,
            transfer_flag,
            portions,
        }
    }
}

bitfield! {
    pub struct CapabilitiesDuringUpdate(u32);
    impl Debug;
    pub u32, reserved, _: 31, 5;
    pub u32, component_security_level_latest, set_component_security_level_latest: 4;
    pub u32, security_revision_number_updateable, set_security_revision_number_updateable: 3;
    pub u32, component_downgrade_capability, set_component_downgrade_capability: 2;
    pub u32, downstream_updateable, set_downstream_updateable: 1;
    pub u32, downstream_apply_state, set_downstream_apply_state: 0;
}
// DMTF0267 12.17
#[derive(Debug, Clone, FromBytes, Immutable, PartialEq)]
pub struct RequestDownstreamDeviceUpdateRequest {
    pub hdr: PldmMsgHeader<[u8; PLDM_MSG_HEADER_LEN]>,
    // "This value shall be equal to or greater than firmware update baseline transfer size"
    // See section 7.8
    pub max_downstream_device_transfer_size: u32,
    pub max_outstanding_transfer_requests: u8,
    pub downstream_device_pkg_data_length: u16,
}

impl RequestDownstreamDeviceUpdateRequest {
    pub fn new(
        instance_id: InstanceId,
        max_downstream_device_transfer_size: u32,
        max_outstanding_transfer_requests: u8,
        downstream_device_pkg_data_length: u16,
    ) -> Self {
        RequestDownstreamDeviceUpdateRequest {
            hdr: PldmMsgHeader::new(
                instance_id,
                PldmMsgType::Request,
                PldmSupportedType::FwUpdate,
                FwUpdateCmd::RequestDownstreamDeviceUpdate as u8,
            ),
            max_downstream_device_transfer_size,
            max_outstanding_transfer_requests,
            downstream_device_pkg_data_length,
        }
    }
}

pub enum DDWillSendGetPackageDataCommand {
    FDPShouldObtainUALimited = 0x02,
    FDPShouldObtainLearn = 0x01,
    FDPNoSupport = 0x00,
}

impl TryFrom<u8> for DDWillSendGetPackageDataCommand {
    type Error = ();
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x02 => Ok(DDWillSendGetPackageDataCommand::FDPShouldObtainUALimited),
            0x01 => Ok(DDWillSendGetPackageDataCommand::FDPShouldObtainLearn),
            0x00 => Ok(DDWillSendGetPackageDataCommand::FDPNoSupport),
            _ => Err(()),
        }
    }
}

impl From<DDWillSendGetPackageDataCommand> for u8 {
    fn from(cmd: DDWillSendGetPackageDataCommand) -> Self {
        match cmd {
            DDWillSendGetPackageDataCommand::FDPShouldObtainUALimited => 0x02,
            DDWillSendGetPackageDataCommand::FDPShouldObtainLearn => 0x01,
            DDWillSendGetPackageDataCommand::FDPNoSupport => 0x00,
        }
    }
}

pldm_completion_code! {
    RequestDownstreamDeviceUpdateCode {
        AlreadyInUpdateMode,
        UnableToInitiateUpdate,
        RetryRequestUpdate
    }
}

pub struct RequestDownstreamDeviceUpdateResponse {
    pub hdr: PldmMsgHeader<[u8; PLDM_MSG_HEADER_LEN]>,

    /// PLDM_BASE_CODES, ALREADY_IN_UPDATE_MODE, UNABLE_TO_INITIATE_UPDATE,
    /// RETRY_REQUEST_UPDATE
    ///
    /// See [RequestDownstreamDeviceUpdateCode].
    pub completion_code: u8,
    pub downstream_device_metadata_length: u16,
    pub pkg_data_command: u8,
    pub get_pkg_data_max_transfer_size: u16,
}

impl RequestDownstreamDeviceUpdateResponse {
    pub fn new(
        instance_id: InstanceId,
        completion_code: RequestDownstreamDeviceUpdateCode,
        downstream_device_metadata_length: u16,
        pkg_data_command: DDWillSendGetPackageDataCommand,
        get_pkg_data_max_transfer_size: u16,
    ) -> Self {
        RequestDownstreamDeviceUpdateResponse {
            hdr: PldmMsgHeader::new(
                instance_id,
                PldmMsgType::Response,
                PldmSupportedType::FwUpdate,
                FwUpdateCmd::RequestDownstreamDeviceUpdate as u8,
            ),
            completion_code: completion_code.into(),
            downstream_device_metadata_length,
            pkg_data_command: pkg_data_command.into(),
            get_pkg_data_max_transfer_size,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_query_device_identifier() {
        let req = QueryDownstreamDevicesRequest::new(1);
        assert_eq!(req.hdr.instance_id(), 1);
    }

    #[test]
    fn test_query_downstream_device_response_new() {
        let resp =
            QueryDownstreamDeviceResponse::new(2, PldmBaseCompletionCode::Success, 0x01, 5, 10);
        assert_eq!(resp.hdr.instance_id(), 2);
        assert_eq!(resp.completion_code, 0x00);
        assert_eq!(resp.downstream_device_update_supported, 0x01);
        assert_eq!(resp.number_of_downstream_devices, 5);
        assert_eq!(resp.max_number_of_downstream_devices, 10);
        // capabilities default to zeroed fields
        assert_eq!(resp.capabilities.update_simultaneous(), false);
        assert_eq!(resp.capabilities.dynamic_remove(), false);
        assert_eq!(resp.capabilities.dynamic_attach(), false);
    }

    #[test]
    fn test_query_downstream_identifiers_portion_and_response() {
        let portion = QueryDownstreamIdentifiersPortion::new(100u32, 2u16, 1u16, None);
        let portions_arr = [
            portion.clone(),
            portion.clone(),
            portion.clone(),
            portion.clone(),
        ];
        let resp = QueryDownstreamIdentifiersResponse::new(
            4,
            QueryDownstreamIdentifiersResponseCode::BaseCodes(PldmBaseCompletionCode::Success),
            0x1234,
            1,
            Some(&portions_arr),
        );

        assert_eq!(resp.hdr.instance_id(), 4);
        assert_eq!(
            QueryDownstreamIdentifiersResponseCode::BaseCodes(
                PldmBaseCompletionCode::try_from(resp.completion_code).unwrap()
            ),
            QueryDownstreamIdentifiersResponseCode::BaseCodes(PldmBaseCompletionCode::Success),
        );
        assert_eq!(resp.next_data_transfer_handle, 0x1234);
        assert_eq!(resp.transfer_flag, 1);
        assert!(resp.portions.is_some());
        let p = resp.portions.unwrap();
        assert_eq!(p[0].downstream_devices_length, 100);
        assert_eq!(p[0].number_of_downstream_devices, 2);
        assert_eq!(p[0].first_downstream_device_index, 1);
        assert!(p[0].downstream_devices.is_none());
    }

    #[test]
    fn test_downstream_devices_new() {
        let ds = DownstreamDevices::new(10u16, 0u8, None);
        assert_eq!(ds.downstream_device_index, 10);
        assert_eq!(ds.downstream_descriptor_count, 0);
        assert!(ds.downstream_descriptors.is_none());
    }

    #[test]
    fn test_get_downstream_firmware_parameters_request_manual() {
        let req = GetDownstreamFirmwareParametersRequest {
            hdr: PldmMsgHeader::new(
                6,
                PldmMsgType::Request,
                PldmSupportedType::FwUpdate,
                FwUpdateCmd::GetDownstreamFirmwareParameters as u8,
            ),
            data_transfer_handle: 0xAABBCCDD,
            transfer_op_flag: 0,
        };
        assert_eq!(req.hdr.instance_id(), 6);
        let handle = req.data_transfer_handle;
        assert_eq!(handle, 0xAABBCCDD);
        assert_eq!(req.transfer_op_flag, 0);
    }

    #[test]
    fn test_query_downstream_identifiers_request_manual() {
        let req = QueryDownstreamIdentifiersRequest {
            hdr: PldmMsgHeader::new(
                5,
                PldmMsgType::Request,
                PldmSupportedType::FwUpdate,
                FwUpdateCmd::QueryDownstreamIdentifiers as u8,
            ),
            downstream_data_device_handle: 0xDEADBEEF,
            transfer_op_flag: 0,
        };
        assert_eq!(req.hdr.instance_id(), 5);
        let handle = req.downstream_data_device_handle;
        assert_eq!(handle, 0xDEADBEEF);
        assert_eq!(req.transfer_op_flag, 0);
    }
}
