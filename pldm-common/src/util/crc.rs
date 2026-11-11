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

//! CRC-8 used by the PLDM Platform Monitoring and Control multipart transfers.

use crc::{Crc, NoTable, CRC_8_SMBUS};

/// CRC-8 over the polynomial x^8 + x^2 + x^1 + 1, computed most-significant
/// bit first with a zero initial value and no output reflection.
///
/// DSP0248 specifies this algorithm for the `transferCRC` field of the GetPDR
/// response (clause 26.2.1, Table 69) and of the ReadPLDMEventLog response
/// (clause 23.6, Table 62). `NoTable` keeps the 256-byte lookup table out of
/// the image; the transfers this covers are a few hundred bytes at most.
const PLDM_CRC8: Crc<u8, NoTable> = Crc::<u8, NoTable>::new(&CRC_8_SMBUS);

/// Computes the `transferCRC` for a complete multipart payload.
///
/// The caller passes the entire reassembled PDR or event log entry, not the
/// individual chunk: DSP0248 defines the CRC over the whole record.
pub fn transfer_crc(data: &[u8]) -> u8 {
    PLDM_CRC8.checksum(data)
}

#[cfg(test)]
mod test {
    use super::*;

    /// Reference implementation straight from the DSP0248 wording: process
    /// bytes in receive order, most-significant bit first. Kept independent of
    /// the `crc` crate so the test fails if the algorithm selection ever drifts.
    fn transfer_crc_reference(data: &[u8]) -> u8 {
        let mut crc: u8 = 0x00;
        for byte in data {
            crc ^= byte;
            for _ in 0..8 {
                crc = if crc & 0x80 != 0 {
                    (crc << 1) ^ 0x07
                } else {
                    crc << 1
                };
            }
        }
        crc
    }

    #[test]
    fn test_transfer_crc_matches_catalog_check_value() {
        // CRC_8_SMBUS carries check = 0xF4 for the string "123456789", the
        // standard catalogue vector. Proves the algorithm selection is right.
        assert_eq!(transfer_crc(b"123456789"), 0xF4);
    }

    #[test]
    fn test_transfer_crc_matches_spec_reference() {
        for case in [
            b"".as_slice(),
            b"\x00".as_slice(),
            b"\xff".as_slice(),
            b"123456789".as_slice(),
            // A plausible Terminus Locator PDR body: 10-byte common header
            // followed by the 8-byte MCTP_EID locator value.
            b"\x01\x00\x00\x00\x01\x01\x00\x00\x08\x00\x01\x00\x01\x01\x00\x00\x01\x01".as_slice(),
        ] {
            assert_eq!(transfer_crc(case), transfer_crc_reference(case));
        }
    }

    #[test]
    fn test_transfer_crc_of_empty_input_is_zero() {
        // The initial value is zero and no bytes are fed, so nothing shifts in.
        assert_eq!(transfer_crc(b""), 0x00);
    }

    #[test]
    fn test_transfer_crc_detects_single_bit_flip() {
        let good = b"\x01\x02\x03\x04";
        let bad = b"\x01\x02\x03\x05";
        assert_ne!(transfer_crc(good), transfer_crc(bad));
    }
}
