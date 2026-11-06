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

//! Every hand-written decoder takes bytes from a peer, so it must return an
//! error on bad input instead of panicking. This feeds each one a fixed set of
//! buffers: all zeros, all 0xFF, pseudo-random bytes, and zeros with one
//! large byte at each offset, so that every length field gets a turn at
//! being too big. The set is deterministic, so a failure always reproduces.
//!
//! Types that decode through the zerocopy blanket impl are not listed; they
//! cannot index past their input. Add any new hand-written decoder here.

use pldm_common::codec::{PldmCodec, PldmCodecWithLifetime};
use pldm_common::message::firmware_update::{
    get_fw_params, get_package_data, pass_component, query_devid, query_downstream,
    request_fw_data, request_update, update_component,
};
use pldm_common::protocol::firmware_update;

const LENGTHS: [usize; 18] = [
    0, 1, 2, 3, 4, 5, 8, 12, 16, 24, 32, 48, 64, 100, 128, 256, 600, 1200,
];

fn inputs() -> Vec<Vec<u8>> {
    let mut seed: u64 = 0x1234_5678;
    let mut next = move || {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (seed >> 33) as u8
    };

    let mut all = Vec::new();
    for len in LENGTHS {
        all.push(vec![0u8; len]);
        all.push(vec![0xFFu8; len]);
        for _ in 0..20 {
            all.push((0..len).map(|_| next()).collect());
        }
        for i in 0..len.min(96) {
            for byte in [0xFF, 0x80, 65, 33, 9] {
                let mut buf = vec![0u8; len];
                buf[i] = byte;
                all.push(buf);
            }
        }
    }
    all
}

fn check(name: &str, decode: impl Fn(&[u8])) {
    for input in inputs() {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| decode(&input)));
        assert!(
            result.is_ok(),
            "{name}::decode panicked on {} bytes starting {:02x?}",
            input.len(),
            &input[..input.len().min(16)]
        );
    }
}

macro_rules! no_panic {
    ($test:ident, $ty:ty) => {
        #[test]
        fn $test() {
            check(stringify!($ty), |b| {
                let _ = <$ty as PldmCodec>::decode(b);
            });
        }
    };
    ($test:ident, $ty:ty, lifetime) => {
        #[test]
        fn $test() {
            check(stringify!($ty), |b| {
                let _ = <$ty as PldmCodecWithLifetime>::decode(b);
            });
        }
    };
}

no_panic!(firmware_string, firmware_update::PldmFirmwareString);
no_panic!(descriptor, firmware_update::Descriptor);
no_panic!(
    component_parameter_entry,
    firmware_update::ComponentParameterEntry
);
no_panic!(firmware_parameters, get_fw_params::FirmwareParameters);
no_panic!(
    get_fw_params_response,
    get_fw_params::GetFirmwareParametersResponse
);
no_panic!(
    get_package_data_response,
    get_package_data::GetPackageDataResponse
);
no_panic!(
    pass_component_request,
    pass_component::PassComponentTableRequest
);
no_panic!(
    query_devid_response,
    query_devid::QueryDeviceIdentifiersResponse
);
no_panic!(
    downstream_parameter_table,
    query_downstream::DownstreamDeviceParameterTable
);
no_panic!(
    downstream_fw_params_portion,
    query_downstream::GetDownstreamFirmwareParametersPortion
);
no_panic!(
    downstream_fw_params_response,
    query_downstream::GetDownstreamFirmwareParametersResponse
);
no_panic!(request_update_request, request_update::RequestUpdateRequest);
no_panic!(
    request_update_response,
    request_update::RequestUpdateResponse
);
no_panic!(
    update_component_request,
    update_component::UpdateComponentRequest
);
no_panic!(
    update_component_response,
    update_component::UpdateComponentResponse
);
no_panic!(
    request_fw_data_response,
    request_fw_data::RequestFirmwareDataResponse,
    lifetime
);
no_panic!(
    device_metadata_response,
    get_package_data::GetDeviceMetaDataResponse,
    lifetime
);
no_panic!(
    metadata_response,
    get_package_data::GetMetaDataResponse,
    lifetime
);
no_panic!(
    downstream_identifiers_response,
    query_downstream::QueryDownstreamIdentifiersResponse,
    lifetime
);
no_panic!(
    downstream_device,
    query_downstream::DownstreamDevice,
    lifetime
);
