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
use pldm_common::message::firmware_update::request_update::{
    RequestUpdateRequest, RequestUpdateRequestFixed,
};
use pldm_common::protocol::base::PldmMsgType;
use pldm_common::protocol::firmware_update::{
    PldmFirmwareString, PLDM_FWUP_IMAGE_SET_VER_STR_MAX_LEN,
};

/// Longest string the 32-byte destination can hold; the boundary that must
/// still decode successfully.
const MAX_LEN: usize = PLDM_FWUP_IMAGE_SET_VER_STR_MAX_LEN;

/// Site 1: `RequestUpdateRequest::decode` reads `comp_image_set_ver_str_len`
/// off the wire and copies that many bytes into a 32-byte array.
#[test]
fn request_update_rejects_over_long_ver_str() {
    let fixed_sz = core::mem::size_of::<RequestUpdateRequestFixed>();

    for len in [MAX_LEN + 1, 64, 255] {
        // Payload is long enough that the *source* check passes; only the
        // destination bound can reject this.
        let mut buf = vec![0u8; fixed_sz + 256];
        buf[fixed_sz - 1] = len as u8; // comp_image_set_ver_str_len

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
    let fixed_sz = core::mem::size_of::<RequestUpdateRequestFixed>();
    let mut buf = vec![0u8; fixed_sz + 4];
    buf[fixed_sz - 1] = MAX_LEN as u8;

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
    assert_eq!(got.str_data.len(), MAX_LEN);
}

/// Site 6: `PassComponentTableRequest::decode`, same shape as site 1.
#[test]
fn pass_component_rejects_over_long_ver_str() {
    let fixed_sz = core::mem::size_of::<PassComponentTableRequestFixed>();

    for len in [MAX_LEN + 1, 64, 255] {
        let mut buf = vec![0u8; fixed_sz + 256];
        buf[fixed_sz - 1] = len as u8; // comp_ver_str_len

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
    let fixed_sz = core::mem::size_of::<FirmwareParamFixed>();

    for len in [MAX_LEN + 1, 64, 255] {
        let mut buf = vec![0u8; fixed_sz + 512];
        // active len is the 4th field; pending len is the last byte of the struct.
        buf[fixed_sz - 3] = len as u8;

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
    let fixed_sz = core::mem::size_of::<FirmwareParamFixed>();

    for len in [MAX_LEN + 1, 64, 255] {
        let mut buf = vec![0u8; fixed_sz + 512];
        buf[fixed_sz - 3] = 8; // a valid active length
        buf[fixed_sz - 1] = len as u8; // pending len

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
