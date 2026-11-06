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

//! Version-string length fields are `u8` values taken off the wire (`0..=255`),
//! but they index a fixed `PLDM_FWUP_IMAGE_SET_VER_STR_MAX_LEN` (32) byte
//! destination. A peer that reports a length above that bound must be rejected
//! with `InvalidData` rather than panicking the decoder.

use pldm_common::codec::{PldmCodec, PldmCodecError};
use pldm_common::message::firmware_update::get_fw_params::{
    FirmwareParamFixed, FirmwareParameters,
};
use pldm_common::message::firmware_update::pass_component::{
    PassComponentTableRequest, PassComponentTableRequestFixed,
};
use pldm_common::message::firmware_update::query_downstream::{
    CapabilitiesDuringUpdate, DownstreamDeviceParameterTable, PldmTimeStamp,
};
use pldm_common::message::firmware_update::request_update::{
    RequestUpdateRequest, RequestUpdateRequestFixed,
};
use pldm_common::message::firmware_update::update_component::{
    UpdateComponentRequest, UpdateComponentRequestFixed,
};
use pldm_common::protocol::base::PldmMsgType;
use pldm_common::protocol::firmware_update::{
    ComponentActivationMethods, ComponentParameterEntry, ComponentParameterEntryFixed, Descriptor,
    PldmFirmwareString, DESCRIPTOR_DATA_MAX_LEN, MAX_COMPONENT_COUNT,
    PLDM_FWUP_IMAGE_SET_VER_STR_MAX_LEN,
};
use zerocopy::{FromZeros, Immutable, IntoBytes};

/// Longest string the 32-byte destination can hold; the boundary that must
/// still decode successfully.
const MAX_LEN: usize = PLDM_FWUP_IMAGE_SET_VER_STR_MAX_LEN;

/// Serializes `fixed` and pads it with `tail` zero bytes, so the payload is
/// long enough that only the destination bound can reject a length.
fn wire<T: IntoBytes + Immutable>(fixed: &T, tail: usize) -> Vec<u8> {
    let mut buf = fixed.as_bytes().to_vec();
    buf.resize(buf.len() + tail, 0);
    buf
}

/// Site 1: `RequestUpdateRequest::decode` reads `comp_image_set_ver_str_len`
/// off the wire and copies that many bytes into a 32-byte array.
#[test]
fn request_update_rejects_over_long_ver_str() {
    for len in [MAX_LEN + 1, 64, 255] {
        let mut fixed = RequestUpdateRequestFixed::new_zeroed();
        fixed.comp_image_set_ver_str_len = len as u8;
        let buf = wire(&fixed, 256);

        assert_eq!(
            RequestUpdateRequest::decode(&buf),
            Err(PldmCodecError::InvalidData),
            "wire length {len} must be rejected, not panic"
        );
    }
}

/// A length that fits the destination but overruns the buffer is still a
/// short-buffer error, not `InvalidData`.
#[test]
fn request_update_short_buffer_still_reports_buffer_too_short() {
    let mut fixed = RequestUpdateRequestFixed::new_zeroed();
    fixed.comp_image_set_ver_str_len = MAX_LEN as u8;
    let buf = wire(&fixed, 4);

    assert_eq!(
        RequestUpdateRequest::decode(&buf),
        Err(PldmCodecError::BufferTooShort)
    );
}

/// Site 2: `get_comp_image_set_ver_str` used `copy_from_slice` on the whole
/// 32-byte destination, which panics for every length that is not exactly 32.
#[test]
fn request_update_getter_handles_every_length() {
    for len in 0..=MAX_LEN {
        let text = "a".repeat(len);
        let fw_str = PldmFirmwareString::new("ASCII", &text).unwrap();
        let req = RequestUpdateRequest::new(0, PldmMsgType::Request, 512, 1, 1, 0, &fw_str);

        let got = req.get_comp_image_set_ver_str();
        assert_eq!(got.str_len as usize, len);
        assert_eq!(&got.str_data[..len], text.as_bytes());
    }
}

/// The getter must also stay in bounds when the struct carries a length that
/// never passed through the decoder's validation.
#[test]
fn request_update_getter_clamps_unvalidated_length() {
    let fw_str = PldmFirmwareString::new("ASCII", "mcu-1.0.0").unwrap();
    let mut req = RequestUpdateRequest::new(0, PldmMsgType::Request, 512, 1, 1, 0, &fw_str);
    req.fixed.comp_image_set_ver_str_len = 255;

    let got = req.get_comp_image_set_ver_str();
    assert_eq!(got.str_len as usize, MAX_LEN);
    assert_eq!(&got.str_data[..9], b"mcu-1.0.0");

    // The clamped string must also encode without panicking.
    let mut buf = [0u8; 64];
    assert!(got.encode(&mut buf).is_ok());
}

/// Site 6: `PassComponentTableRequest::decode`, same shape as site 1.
#[test]
fn pass_component_rejects_over_long_ver_str() {
    for len in [MAX_LEN + 1, 64, 255] {
        let mut fixed = PassComponentTableRequestFixed::new_zeroed();
        fixed.comp_ver_str_len = len as u8;
        let buf = wire(&fixed, 256);

        assert_eq!(
            PassComponentTableRequest::decode(&buf),
            Err(PldmCodecError::InvalidData),
            "wire length {len} must be rejected, not panic"
        );
    }
}

/// Site 5: `PldmFirmwareString::decode` reads a `u8` length into the same
/// 32-byte destination.
#[test]
fn firmware_string_rejects_over_long_ver_str() {
    for len in [MAX_LEN + 1, 64, 255] {
        let mut buf = vec![0u8; 512];
        buf[0] = 1; // str_type
        buf[1] = len as u8; // str_len

        assert_eq!(
            PldmFirmwareString::decode(&buf),
            Err(PldmCodecError::InvalidData),
            "wire length {len} must be rejected, not panic"
        );
    }
}

/// Every length the destination can hold must still round-trip unchanged.
#[test]
fn firmware_string_round_trips_every_valid_length() {
    for len in 0..=MAX_LEN {
        let text = "v".repeat(len);
        let original = PldmFirmwareString::new("ASCII", &text).unwrap();

        let mut buf = [0u8; 128];
        let n = original.encode(&mut buf).unwrap();
        let decoded = PldmFirmwareString::decode(&buf[..n]).unwrap();

        assert_eq!(decoded.str_len as usize, len);
        assert_eq!(&decoded.str_data[..len], text.as_bytes());
    }
}

/// Site 3: `FirmwareParameters::decode` copies
/// `active_comp_image_set_ver_str_len` bytes into a 32-byte array.
#[test]
fn fw_params_rejects_over_long_active_ver_str() {
    for len in [MAX_LEN + 1, 64, 255] {
        let mut fixed = FirmwareParamFixed::new_zeroed();
        fixed.active_comp_image_set_ver_str_len = len as u8;
        let buf = wire(&fixed, 512);

        assert_eq!(
            FirmwareParameters::decode(&buf),
            Err(PldmCodecError::InvalidData),
            "active wire length {len} must be rejected, not panic"
        );
    }
}

/// Site 4: the same field for the pending version string.
#[test]
fn fw_params_rejects_over_long_pending_ver_str() {
    for len in [MAX_LEN + 1, 64, 255] {
        let mut fixed = FirmwareParamFixed::new_zeroed();
        fixed.active_comp_image_set_ver_str_len = 8;
        fixed.pending_comp_image_set_ver_str_len = len as u8;
        let buf = wire(&fixed, 512);

        assert_eq!(
            FirmwareParameters::decode(&buf),
            Err(PldmCodecError::InvalidData),
            "pending wire length {len} must be rejected, not panic"
        );
    }
}

/// A valid request must still decode after the bounds check is added.
#[test]
fn request_update_round_trip_still_works() {
    let request = RequestUpdateRequest::new(
        0,
        PldmMsgType::Request,
        512,
        2,
        1,
        256,
        &PldmFirmwareString::new("ASCII", "mcu-1.0.0").unwrap(),
    );

    let mut buffer = [0u8; 512];
    let n = request.encode(&mut buffer).unwrap();
    assert_eq!(RequestUpdateRequest::decode(&buffer[..n]).unwrap(), request);
}

/// Site 7: `UpdateComponentRequest::decode`, same shape as site 1. It also
/// indexed the source without `.get()`, so a short payload panicked too.
#[test]
fn update_component_rejects_over_long_ver_str() {
    for len in [MAX_LEN + 1, 64, 255] {
        let mut fixed = UpdateComponentRequestFixed::new_zeroed();
        fixed.comp_ver_str_len = len as u8;
        let buf = wire(&fixed, 256);

        assert_eq!(
            UpdateComponentRequest::decode(&buf),
            Err(PldmCodecError::InvalidData),
            "wire length {len} must be rejected, not panic"
        );
    }
}

#[test]
fn update_component_short_buffer_reports_buffer_too_short() {
    let mut fixed = UpdateComponentRequestFixed::new_zeroed();
    fixed.comp_ver_str_len = MAX_LEN as u8;
    let buf = wire(&fixed, 4);

    assert_eq!(
        UpdateComponentRequest::decode(&buf),
        Err(PldmCodecError::BufferTooShort)
    );
}

/// Sites 8 and 9: `ComponentParameterEntry::decode`, active and pending
/// component version strings.
#[test]
fn component_entry_rejects_over_long_active_ver_str() {
    for len in [MAX_LEN + 1, 64, 255] {
        let mut fixed = ComponentParameterEntryFixed::new_zeroed();
        fixed.active_comp_ver_str_len = len as u8;
        let buf = wire(&fixed, 512);

        assert_eq!(
            ComponentParameterEntry::decode(&buf),
            Err(PldmCodecError::InvalidData),
            "active wire length {len} must be rejected, not panic"
        );
    }
}

#[test]
fn component_entry_rejects_over_long_pending_ver_str() {
    for len in [MAX_LEN + 1, 64, 255] {
        let mut fixed = ComponentParameterEntryFixed::new_zeroed();
        fixed.active_comp_ver_str_len = 8;
        fixed.pending_comp_ver_str_len = len as u8;
        let buf = wire(&fixed, 512);

        assert_eq!(
            ComponentParameterEntry::decode(&buf),
            Err(PldmCodecError::InvalidData),
            "pending wire length {len} must be rejected, not panic"
        );
    }
}

/// Site 10: a response that claims a component and then stops short must be
/// a short-buffer error, not a panic on `unwrap`.
#[test]
fn fw_params_truncated_component_reports_buffer_too_short() {
    let mut fixed = FirmwareParamFixed::new_zeroed();
    fixed.comp_count = 1;
    let buf = wire(&fixed, 4);

    assert_eq!(
        FirmwareParameters::decode(&buf),
        Err(PldmCodecError::BufferTooShort)
    );
}

/// Site 11: `comp_count` is a `u16` off the wire, but the table holds
/// `MAX_COMPONENT_COUNT` entries. Anything above that must be rejected, not
/// silently truncated into a struct that panics when it is encoded again.
#[test]
fn fw_params_rejects_too_many_components() {
    let entry_sz = core::mem::size_of::<ComponentParameterEntryFixed>();
    for count in [MAX_COMPONENT_COUNT + 1, u16::MAX as usize] {
        let mut fixed = FirmwareParamFixed::new_zeroed();
        fixed.comp_count = count as u16;
        let buf = wire(&fixed, entry_sz * (MAX_COMPONENT_COUNT + 1));

        assert_eq!(
            FirmwareParameters::decode(&buf),
            Err(PldmCodecError::InvalidData),
            "comp_count {count} must be rejected"
        );
    }
}

/// The full table must still decode and encode again unchanged.
#[test]
fn fw_params_max_components_round_trip() {
    let entry_sz = core::mem::size_of::<ComponentParameterEntryFixed>();
    let mut fixed = FirmwareParamFixed::new_zeroed();
    fixed.comp_count = MAX_COMPONENT_COUNT as u16;
    let buf = wire(&fixed, entry_sz * MAX_COMPONENT_COUNT);

    let params = FirmwareParameters::decode(&buf).unwrap();
    let mut out = vec![0u8; buf.len()];
    let n = params.encode(&mut out).unwrap();
    assert_eq!(&out[..n], &buf[..]);
}

/// Site 12: `Descriptor::decode` reads a `u16` length into a
/// `DESCRIPTOR_DATA_MAX_LEN` (64) byte array.
fn descriptor_wire(len: u16, tail: usize) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(&0xFFFFu16.to_le_bytes()); // descriptor_type
    buf.extend_from_slice(&len.to_le_bytes()); // descriptor_length
    buf.resize(buf.len() + tail, 0);
    buf
}

#[test]
fn descriptor_rejects_over_long_data() {
    for len in [DESCRIPTOR_DATA_MAX_LEN + 1, 255, u16::MAX as usize] {
        assert_eq!(
            Descriptor::decode(&descriptor_wire(len as u16, 1024)),
            Err(PldmCodecError::InvalidData),
            "descriptor length {len} must be rejected, not panic"
        );
    }
}

#[test]
fn descriptor_accepts_max_len_data() {
    let buf = descriptor_wire(DESCRIPTOR_DATA_MAX_LEN as u16, DESCRIPTOR_DATA_MAX_LEN);
    let desc = Descriptor::decode(&buf).unwrap();
    assert_eq!(desc.descriptor_length as usize, DESCRIPTOR_DATA_MAX_LEN);
}

/// Sites 13 and 14: `DownstreamDeviceParameterTable::decode`, active and
/// pending component version strings. The table has no zerocopy struct, so
/// the fields are written in wire order here.
fn downstream_table_wire(active_len: u8, pending_len: u8, tail: usize) -> Vec<u8> {
    let date = [0u8; core::mem::size_of::<PldmTimeStamp>()];
    let mut buf = Vec::new();
    buf.extend_from_slice(&0u16.to_le_bytes()); // downstream_device_index
    buf.extend_from_slice(&0u32.to_le_bytes()); // active comparison stamp
    buf.push(1); // active version string type
    buf.push(active_len); // active version string length
    buf.extend_from_slice(&date); // active release date
    buf.extend_from_slice(&0u32.to_le_bytes()); // pending comparison stamp
    buf.push(1); // pending version string type
    buf.push(pending_len); // pending version string length
    buf.extend_from_slice(&date); // pending release date
    buf.extend_from_slice(&[0u8; core::mem::size_of::<ComponentActivationMethods>()]);
    buf.extend_from_slice(&[0u8; core::mem::size_of::<CapabilitiesDuringUpdate>()]);
    buf.resize(buf.len() + tail, 0);
    buf
}

#[test]
fn downstream_table_rejects_over_long_active_ver_str() {
    for len in [MAX_LEN + 1, 64, 255] {
        assert_eq!(
            DownstreamDeviceParameterTable::decode(&downstream_table_wire(len as u8, 0, 512)),
            Err(PldmCodecError::InvalidData),
            "active wire length {len} must be rejected, not panic"
        );
    }
}

#[test]
fn downstream_table_rejects_over_long_pending_ver_str() {
    for len in [MAX_LEN + 1, 64, 255] {
        assert_eq!(
            DownstreamDeviceParameterTable::decode(&downstream_table_wire(8, len as u8, 512)),
            Err(PldmCodecError::InvalidData),
            "pending wire length {len} must be rejected, not panic"
        );
    }
}

#[test]
fn downstream_table_short_strings_report_buffer_too_short() {
    // The lengths fit the destination, but the strings are not all there.
    let buf = downstream_table_wire(MAX_LEN as u8, MAX_LEN as u8, MAX_LEN + 4);
    assert_eq!(
        DownstreamDeviceParameterTable::decode(&buf),
        Err(PldmCodecError::BufferTooShort)
    );
}

#[test]
fn downstream_table_accepts_max_len_strings() {
    let buf = downstream_table_wire(MAX_LEN as u8, MAX_LEN as u8, 2 * MAX_LEN);
    let table = DownstreamDeviceParameterTable::decode(&buf).unwrap();
    assert_eq!(
        table.active_component_version_string.str_len as usize,
        MAX_LEN
    );
    assert_eq!(
        table.pending_component_version_string.str_len as usize,
        MAX_LEN
    );
}
