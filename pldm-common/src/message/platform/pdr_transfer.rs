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

//! Multipart transfer state for the GetPDR command, DSP0248 clause 26.2.7.
//!
//! The state machine holds no record data of its own. The caller looks the
//! requested record up in whatever repository it owns and passes it in, which
//! keeps this crate free of any storage or platform dependency.

use crate::message::platform::get_pdr::{
    GetPdrRequest, DATA_TRANSFER_HANDLE_NONE, RECORD_HANDLE_NONE,
};
use crate::protocol::base::PldmBaseCompletionCode;
use crate::protocol::platform::{
    GetPdrCompletionCode, PdrRepositoryState, PlatformTransferFlag, PlatformTransferOperationFlag,
};
use crate::util::crc::transfer_crc;

/// First handle a transfer hands out. Any non-zero value works, because
/// DSP0248 leaves the handle opaque to the requester; zero is reserved for
/// [`DATA_TRANSFER_HANDLE_NONE`].
const FIRST_DATA_TRANSFER_HANDLE: u32 = 1;

/// One record as the repository hands it over.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PdrRecord<'a> {
    pub record_handle: u32,
    /// [`RECORD_HANDLE_NONE`] on the last record of the repository.
    pub next_record_handle: u32,
    /// The value inside the record's own common header, DSP0248 clause 28.1.
    pub record_change_number: u16,
    /// The complete record, every part of a multipart transfer included.
    pub data: &'a [u8],
}

/// One response part, ready for [`super::get_pdr::GetPdrResponse::new`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PdrTransferPart<'a> {
    pub next_record_handle: u32,
    pub next_data_transfer_handle: u32,
    pub transfer_flag: PlatformTransferFlag,
    pub record_data: &'a [u8],
    pub transfer_crc: Option<u8>,
}

/// Why a GetPDR request cannot be answered with record data.
///
/// Table 69 lists both the shared base codes and six codes of its own, so a
/// single enumeration cannot carry them; the caller writes
/// [`GetPdrError::completion_code`] into a failure response.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GetPdrError {
    Base(PldmBaseCompletionCode),
    Command(GetPdrCompletionCode),
}

impl GetPdrError {
    pub fn completion_code(&self) -> u8 {
        match self {
            GetPdrError::Base(code) => *code as u8,
            GetPdrError::Command(code) => *code as u8,
        }
    }
}

/// Position of the multipart GetPDR transfer in progress, if any.
///
/// One transfer at a time, which DSP0248 clause 26.2.9 permits a responder to
/// impose. Starting a new `GetFirstPart` abandons whatever was in flight.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PdrTransferState {
    /// The handle the requester sent to start the transfer, which clause
    /// 26.2.7 has it repeat on every later part. A walk starting at
    /// `RECORD_HANDLE_FIRST` repeats that value rather than the handle the
    /// repository resolved it to.
    requested_handle: u32,
    /// The handle the repository resolved the request to, which pins the
    /// transfer to one record even as the requester repeats its own value.
    record_handle: u32,
    /// Bytes of that record already delivered.
    offset: usize,
    /// Handle the requester must present next; [`DATA_TRANSFER_HANDLE_NONE`]
    /// while idle.
    data_transfer_handle: u32,
    /// Change number pinned when the first part went out, clause 26.2.5.
    record_change_number: u16,
    /// Source of the next handle value. Never zero, so an active transfer is
    /// always distinguishable from an idle one.
    handle_seed: u32,
}

impl Default for PdrTransferState {
    fn default() -> Self {
        Self::new()
    }
}

impl PdrTransferState {
    pub const fn new() -> Self {
        PdrTransferState {
            requested_handle: RECORD_HANDLE_NONE,
            record_handle: RECORD_HANDLE_NONE,
            offset: 0,
            data_transfer_handle: DATA_TRANSFER_HANDLE_NONE,
            record_change_number: 0,
            handle_seed: FIRST_DATA_TRANSFER_HANDLE,
        }
    }

    /// Whether a multipart transfer is waiting for a `GetNextPart`.
    pub fn is_active(&self) -> bool {
        self.data_transfer_handle != DATA_TRANSFER_HANDLE_NONE
    }

    /// Abandons any transfer in progress, for a transfer timeout or a
    /// repository update.
    pub fn reset(&mut self) {
        self.requested_handle = RECORD_HANDLE_NONE;
        self.record_handle = RECORD_HANDLE_NONE;
        self.offset = 0;
        self.data_transfer_handle = DATA_TRANSFER_HANDLE_NONE;
        self.record_change_number = 0;
    }

    /// Produces the next response part for `request`.
    ///
    /// `record` is the repository's answer for the request's `recordHandle`,
    /// and `None` means no such record. The state machine validates the
    /// request against the transfer it is holding and advances only on
    /// success, so a rejected request leaves a transfer in progress intact.
    ///
    /// `max_chunk` caps the part at what the caller's response buffer can
    /// carry, which Table 69 permits by letting a responder return fewer
    /// bytes than `requestCount` asks for. Without the cap a requester
    /// asking for more than the buffer holds would get no response at all.
    /// See [`super::get_pdr::max_record_data_len`].
    pub fn next_part<'a>(
        &mut self,
        request: &GetPdrRequest,
        repository_state: PdrRepositoryState,
        record: Option<&PdrRecord<'a>>,
        max_chunk: usize,
    ) -> Result<PdrTransferPart<'a>, GetPdrError> {
        match repository_state {
            PdrRepositoryState::Available => {}
            PdrRepositoryState::UpdateInProgress => {
                return Err(GetPdrError::Command(
                    GetPdrCompletionCode::RepositoryUpdateInProgress,
                ))
            }
            // Table 68 defines no command-specific code for a failed
            // repository, so the generic error applies.
            PdrRepositoryState::Failed => {
                return Err(GetPdrError::Base(PldmBaseCompletionCode::Error))
            }
        }

        let operation = request.transfer_operation_flag().map_err(|_| {
            GetPdrError::Command(GetPdrCompletionCode::InvalidTransferOperationFlag)
        })?;
        let record = record.ok_or(GetPdrError::Command(
            GetPdrCompletionCode::InvalidRecordHandle,
        ))?;

        match operation {
            PlatformTransferOperationFlag::GetFirstPart => {
                Ok(self.first_part(request, record, max_chunk))
            }
            PlatformTransferOperationFlag::GetNextPart => {
                self.next_chunk(request, record, max_chunk)
            }
            // Table 69 admits the first two operations only; Table 16 defines
            // acknowledgementOnly for PollForPlatformEventMessage instead.
            PlatformTransferOperationFlag::AcknowledgementOnly => Err(GetPdrError::Command(
                GetPdrCompletionCode::InvalidTransferOperationFlag,
            )),
        }
    }

    fn first_part<'a>(
        &mut self,
        request: &GetPdrRequest,
        record: &PdrRecord<'a>,
        max_chunk: usize,
    ) -> PdrTransferPart<'a> {
        self.reset();
        let wanted = usize::from(request.request_count).min(max_chunk);
        if record.data.len() <= wanted {
            return PdrTransferPart {
                next_record_handle: record.next_record_handle,
                next_data_transfer_handle: DATA_TRANSFER_HANDLE_NONE,
                transfer_flag: PlatformTransferFlag::StartAndEnd,
                record_data: record.data,
                transfer_crc: None,
            };
        }

        self.requested_handle = request.record_handle;
        self.record_handle = record.record_handle;
        self.record_change_number = record.record_change_number;
        self.offset = wanted;
        PdrTransferPart {
            next_record_handle: record.next_record_handle,
            next_data_transfer_handle: self.allocate_handle(),
            transfer_flag: PlatformTransferFlag::Start,
            record_data: &record.data[..wanted],
            transfer_crc: None,
        }
    }

    fn next_chunk<'a>(
        &mut self,
        request: &GetPdrRequest,
        record: &PdrRecord<'a>,
        max_chunk: usize,
    ) -> Result<PdrTransferPart<'a>, GetPdrError> {
        if !self.is_active() || request.data_transfer_handle != self.data_transfer_handle {
            return Err(GetPdrError::Command(
                GetPdrCompletionCode::InvalidDataTransferHandle,
            ));
        }
        // Clause 26.2.7 has the requester repeat the handle it opened the
        // transfer with, which is `RECORD_HANDLE_FIRST` for a walk starting
        // at the first record; comparing against the resolved handle instead
        // would refuse every continuation of such a walk.
        if request.record_handle != self.requested_handle
            || record.record_handle != self.record_handle
        {
            return Err(GetPdrError::Command(
                GetPdrCompletionCode::InvalidRecordHandle,
            ));
        }
        // Clause 26.2.5: a changed record invalidates the parts already sent,
        // whether the requester noticed or not.
        if request.record_change_number != self.record_change_number
            || record.record_change_number != self.record_change_number
        {
            self.reset();
            return Err(GetPdrError::Command(
                GetPdrCompletionCode::InvalidRecordChangeNumber,
            ));
        }

        let remaining = record.data.len().saturating_sub(self.offset);
        let taken = remaining
            .min(usize::from(request.request_count))
            .min(max_chunk);
        // A record that shrank below the delivered offset keeps its change
        // number here, so the slice is what catches it; indexing directly
        // would panic on a repository that misreports its own record.
        let chunk = match record.data.get(self.offset..self.offset + taken) {
            Some(chunk) => chunk,
            None => {
                self.reset();
                return Err(GetPdrError::Command(
                    GetPdrCompletionCode::InvalidRecordChangeNumber,
                ));
            }
        };
        self.offset += taken;

        if taken == remaining {
            self.reset();
            return Ok(PdrTransferPart {
                next_record_handle: record.next_record_handle,
                next_data_transfer_handle: DATA_TRANSFER_HANDLE_NONE,
                transfer_flag: PlatformTransferFlag::End,
                record_data: chunk,
                // Table 69 computes the CRC over the whole PDR, not over the
                // part that carries it.
                transfer_crc: Some(transfer_crc(record.data)),
            });
        }

        Ok(PdrTransferPart {
            next_record_handle: record.next_record_handle,
            next_data_transfer_handle: self.allocate_handle(),
            transfer_flag: PlatformTransferFlag::Middle,
            record_data: chunk,
            transfer_crc: None,
        })
    }

    fn allocate_handle(&mut self) -> u32 {
        let handle = self.handle_seed;
        self.handle_seed = match self.handle_seed.wrapping_add(1) {
            DATA_TRANSFER_HANDLE_NONE => 1,
            next => next,
        };
        self.data_transfer_handle = handle;
        handle
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::message::platform::get_pdr::RECORD_HANDLE_FIRST;
    use crate::protocol::base::PldmMsgType;

    const RECORD_DATA: [u8; 10] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    /// Leaves `requestCount` as the only bound, which is what the tests
    /// below are about; the buffer bound has its own tests.
    const UNBOUNDED_CHUNK: usize = usize::MAX;
    const RECORD_HANDLE: u32 = 0x0000_0005;
    const NEXT_RECORD_HANDLE: u32 = 0x0000_0006;
    const CHANGE_NUMBER: u16 = 0x0042;

    /// Reference CRC-8, polynomial x^8 + x^2 + x^1 + 1, most-significant bit
    /// first. Recomputed here so the assertion encodes the specification
    /// rather than whatever the implementation happens to return.
    fn crc8_reference(data: &[u8]) -> u8 {
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

    fn record() -> PdrRecord<'static> {
        PdrRecord {
            record_handle: RECORD_HANDLE,
            next_record_handle: NEXT_RECORD_HANDLE,
            record_change_number: CHANGE_NUMBER,
            data: &RECORD_DATA,
        }
    }

    fn request(
        operation: PlatformTransferOperationFlag,
        data_transfer_handle: u32,
        request_count: u16,
        record_change_number: u16,
    ) -> GetPdrRequest {
        GetPdrRequest::new(
            0,
            PldmMsgType::Request,
            RECORD_HANDLE,
            data_transfer_handle,
            operation,
            request_count,
            record_change_number,
        )
    }

    #[test]
    fn test_record_fitting_the_request_count_transfers_in_one_part() {
        let mut state = PdrTransferState::new();
        let part = state
            .next_part(
                &request(PlatformTransferOperationFlag::GetFirstPart, 0, 64, 0),
                PdrRepositoryState::Available,
                Some(&record()),
                UNBOUNDED_CHUNK,
            )
            .unwrap();

        assert_eq!(part.transfer_flag, PlatformTransferFlag::StartAndEnd);
        assert_eq!(part.record_data, &RECORD_DATA);
        // A single-part transfer carries no CRC, so appending one would shift
        // every following byte in the requester's decoder.
        assert_eq!(part.transfer_crc, None);
        assert_eq!(part.next_data_transfer_handle, DATA_TRANSFER_HANDLE_NONE);
        assert_eq!(part.next_record_handle, NEXT_RECORD_HANDLE);
        assert!(!state.is_active());
    }

    #[test]
    fn test_record_exactly_the_request_count_still_transfers_in_one_part() {
        let mut state = PdrTransferState::new();
        let part = state
            .next_part(
                &request(
                    PlatformTransferOperationFlag::GetFirstPart,
                    0,
                    u16::try_from(RECORD_DATA.len()).unwrap(),
                    0,
                ),
                PdrRepositoryState::Available,
                Some(&record()),
                UNBOUNDED_CHUNK,
            )
            .unwrap();

        assert_eq!(part.transfer_flag, PlatformTransferFlag::StartAndEnd);
        assert!(!state.is_active());
    }

    #[test]
    fn test_three_part_transfer_ends_with_a_crc_over_the_whole_record() {
        let mut state = PdrTransferState::new();
        let record = record();

        let first = state
            .next_part(
                &request(PlatformTransferOperationFlag::GetFirstPart, 0, 4, 0),
                PdrRepositoryState::Available,
                Some(&record),
                UNBOUNDED_CHUNK,
            )
            .unwrap();
        assert_eq!(first.transfer_flag, PlatformTransferFlag::Start);
        assert_eq!(first.record_data, &RECORD_DATA[..4]);
        assert_eq!(first.transfer_crc, None);
        assert_ne!(first.next_data_transfer_handle, DATA_TRANSFER_HANDLE_NONE);

        let middle = state
            .next_part(
                &request(
                    PlatformTransferOperationFlag::GetNextPart,
                    first.next_data_transfer_handle,
                    4,
                    CHANGE_NUMBER,
                ),
                PdrRepositoryState::Available,
                Some(&record),
                UNBOUNDED_CHUNK,
            )
            .unwrap();
        assert_eq!(middle.transfer_flag, PlatformTransferFlag::Middle);
        assert_eq!(middle.record_data, &RECORD_DATA[4..8]);
        assert_eq!(middle.transfer_crc, None);

        let last = state
            .next_part(
                &request(
                    PlatformTransferOperationFlag::GetNextPart,
                    middle.next_data_transfer_handle,
                    4,
                    CHANGE_NUMBER,
                ),
                PdrRepositoryState::Available,
                Some(&record),
                UNBOUNDED_CHUNK,
            )
            .unwrap();
        assert_eq!(last.transfer_flag, PlatformTransferFlag::End);
        assert_eq!(last.record_data, &RECORD_DATA[8..]);
        assert_eq!(last.transfer_crc, Some(crc8_reference(&RECORD_DATA)));
        assert_eq!(last.next_data_transfer_handle, DATA_TRANSFER_HANDLE_NONE);
        // Every part names the same following record, Table 69.
        assert_eq!(last.next_record_handle, NEXT_RECORD_HANDLE);
        assert!(!state.is_active());
    }

    #[test]
    fn test_walk_started_at_the_first_handle_can_be_continued() {
        // Clause 26.2.7 has the requester repeat the handle it opened the
        // transfer with. A BMC walking the repository opens at
        // RECORD_HANDLE_FIRST and keeps sending it, while the repository
        // resolved that to record 5; comparing the two refuses the walk.
        let mut state = PdrTransferState::new();
        let record = record();

        let first = state
            .next_part(
                &GetPdrRequest::new(
                    0,
                    PldmMsgType::Request,
                    RECORD_HANDLE_FIRST,
                    DATA_TRANSFER_HANDLE_NONE,
                    PlatformTransferOperationFlag::GetFirstPart,
                    4,
                    0,
                ),
                PdrRepositoryState::Available,
                Some(&record),
                UNBOUNDED_CHUNK,
            )
            .unwrap();
        assert_eq!(first.transfer_flag, PlatformTransferFlag::Start);

        let next = state
            .next_part(
                &GetPdrRequest::new(
                    0,
                    PldmMsgType::Request,
                    RECORD_HANDLE_FIRST,
                    first.next_data_transfer_handle,
                    PlatformTransferOperationFlag::GetNextPart,
                    4,
                    CHANGE_NUMBER,
                ),
                PdrRepositoryState::Available,
                Some(&record),
                UNBOUNDED_CHUNK,
            )
            .unwrap();
        assert_eq!(next.record_data, &RECORD_DATA[4..8]);
        assert_eq!(next.transfer_flag, PlatformTransferFlag::Middle);
    }

    #[test]
    fn test_buffer_bound_shortens_the_part_below_the_request_count() {
        // Table 69 lets a responder return fewer bytes than asked for, which
        // is what keeps a requestCount larger than the response buffer from
        // producing no response at all.
        let mut state = PdrTransferState::new();
        let part = state
            .next_part(
                &request(PlatformTransferOperationFlag::GetFirstPart, 0, 64, 0),
                PdrRepositoryState::Available,
                Some(&record()),
                3,
            )
            .unwrap();

        assert_eq!(part.record_data, &RECORD_DATA[..3]);
        assert_eq!(part.transfer_flag, PlatformTransferFlag::Start);
        assert!(state.is_active());
    }

    #[test]
    fn test_record_shrinking_below_the_offset_is_refused_not_panicked() {
        let mut state = PdrTransferState::new();
        let record = record();
        let first = state
            .next_part(
                &request(PlatformTransferOperationFlag::GetFirstPart, 0, 6, 0),
                PdrRepositoryState::Available,
                Some(&record),
                UNBOUNDED_CHUNK,
            )
            .unwrap();
        assert_eq!(first.record_data.len(), 6);

        // A repository handing back a shorter record without touching its
        // change number, which is the one case the change-number check
        // cannot catch and a slice index would panic on.
        let shrunk = PdrRecord {
            data: &RECORD_DATA[..4],
            ..record
        };
        assert_eq!(
            state.next_part(
                &request(
                    PlatformTransferOperationFlag::GetNextPart,
                    first.next_data_transfer_handle,
                    6,
                    CHANGE_NUMBER,
                ),
                PdrRepositoryState::Available,
                Some(&shrunk),
                UNBOUNDED_CHUNK,
            ),
            Err(GetPdrError::Command(
                GetPdrCompletionCode::InvalidRecordChangeNumber
            ))
        );
        assert!(!state.is_active());
    }

    #[test]
    fn test_request_count_zero_starts_a_multipart_transfer_with_no_data() {
        // Table 69 answers responseCount 0 rather than an error, so the
        // transfer must still be resumable.
        let mut state = PdrTransferState::new();
        let part = state
            .next_part(
                &request(PlatformTransferOperationFlag::GetFirstPart, 0, 0, 0),
                PdrRepositoryState::Available,
                Some(&record()),
                UNBOUNDED_CHUNK,
            )
            .unwrap();

        assert_eq!(part.transfer_flag, PlatformTransferFlag::Start);
        assert!(part.record_data.is_empty());
        assert_ne!(part.next_data_transfer_handle, DATA_TRANSFER_HANDLE_NONE);
        assert!(state.is_active());
    }

    #[test]
    fn test_empty_record_transfers_in_one_part() {
        let mut state = PdrTransferState::new();
        let empty = PdrRecord {
            data: &[],
            ..record()
        };
        let part = state
            .next_part(
                &request(PlatformTransferOperationFlag::GetFirstPart, 0, 0, 0),
                PdrRepositoryState::Available,
                Some(&empty),
                UNBOUNDED_CHUNK,
            )
            .unwrap();

        assert_eq!(part.transfer_flag, PlatformTransferFlag::StartAndEnd);
        assert!(part.record_data.is_empty());
        assert!(!state.is_active());
    }

    #[test]
    fn test_unknown_record_handle_is_rejected() {
        let mut state = PdrTransferState::new();
        assert_eq!(
            state.next_part(
                &request(PlatformTransferOperationFlag::GetFirstPart, 0, 64, 0),
                PdrRepositoryState::Available,
                None,
                UNBOUNDED_CHUNK,
            ),
            Err(GetPdrError::Command(
                GetPdrCompletionCode::InvalidRecordHandle
            ))
        );
    }

    #[test]
    fn test_acknowledgement_only_is_rejected() {
        // The value is legal in Table 16 but not in Table 69, so it must fail
        // as an operation flag rather than decode into a transfer.
        let mut state = PdrTransferState::new();
        assert_eq!(
            state.next_part(
                &request(PlatformTransferOperationFlag::AcknowledgementOnly, 0, 64, 0),
                PdrRepositoryState::Available,
                Some(&record()),
                UNBOUNDED_CHUNK,
            ),
            Err(GetPdrError::Command(
                GetPdrCompletionCode::InvalidTransferOperationFlag
            ))
        );
    }

    #[test]
    fn test_undefined_operation_flag_is_rejected() {
        let mut state = PdrTransferState::new();
        let mut req = request(PlatformTransferOperationFlag::GetFirstPart, 0, 64, 0);
        req.transfer_operation_flag = 0x07;
        assert_eq!(
            state.next_part(
                &req,
                PdrRepositoryState::Available,
                Some(&record()),
                UNBOUNDED_CHUNK
            ),
            Err(GetPdrError::Command(
                GetPdrCompletionCode::InvalidTransferOperationFlag
            ))
        );
    }

    #[test]
    fn test_next_part_without_a_transfer_in_progress_is_rejected() {
        let mut state = PdrTransferState::new();
        assert_eq!(
            state.next_part(
                &request(
                    PlatformTransferOperationFlag::GetNextPart,
                    1,
                    4,
                    CHANGE_NUMBER
                ),
                PdrRepositoryState::Available,
                Some(&record()),
                UNBOUNDED_CHUNK,
            ),
            Err(GetPdrError::Command(
                GetPdrCompletionCode::InvalidDataTransferHandle
            ))
        );
    }

    #[test]
    fn test_stale_data_transfer_handle_is_rejected_and_keeps_the_transfer() {
        let mut state = PdrTransferState::new();
        let record = record();
        let first = state
            .next_part(
                &request(PlatformTransferOperationFlag::GetFirstPart, 0, 4, 0),
                PdrRepositoryState::Available,
                Some(&record),
                UNBOUNDED_CHUNK,
            )
            .unwrap();

        assert_eq!(
            state.next_part(
                &request(
                    PlatformTransferOperationFlag::GetNextPart,
                    first.next_data_transfer_handle.wrapping_add(1),
                    4,
                    CHANGE_NUMBER,
                ),
                PdrRepositoryState::Available,
                Some(&record),
                UNBOUNDED_CHUNK,
            ),
            Err(GetPdrError::Command(
                GetPdrCompletionCode::InvalidDataTransferHandle
            ))
        );
        // A rejected request must not consume the part the requester still
        // has to fetch.
        assert!(state.is_active());
        let retry = state
            .next_part(
                &request(
                    PlatformTransferOperationFlag::GetNextPart,
                    first.next_data_transfer_handle,
                    4,
                    CHANGE_NUMBER,
                ),
                PdrRepositoryState::Available,
                Some(&record),
                UNBOUNDED_CHUNK,
            )
            .unwrap();
        assert_eq!(retry.record_data, &RECORD_DATA[4..8]);
    }

    #[test]
    fn test_changed_record_change_number_aborts_the_transfer() {
        let mut state = PdrTransferState::new();
        let record = record();
        let first = state
            .next_part(
                &request(PlatformTransferOperationFlag::GetFirstPart, 0, 4, 0),
                PdrRepositoryState::Available,
                Some(&record),
                UNBOUNDED_CHUNK,
            )
            .unwrap();

        let updated = PdrRecord {
            record_change_number: CHANGE_NUMBER + 1,
            ..record
        };
        assert_eq!(
            state.next_part(
                &request(
                    PlatformTransferOperationFlag::GetNextPart,
                    first.next_data_transfer_handle,
                    4,
                    CHANGE_NUMBER,
                ),
                PdrRepositoryState::Available,
                Some(&updated),
                UNBOUNDED_CHUNK,
            ),
            Err(GetPdrError::Command(
                GetPdrCompletionCode::InvalidRecordChangeNumber
            ))
        );
        // The parts already sent describe a record that no longer exists, so
        // the requester has to start over.
        assert!(!state.is_active());
    }

    #[test]
    fn test_mismatched_record_handle_mid_transfer_is_rejected() {
        let mut state = PdrTransferState::new();
        let record = record();
        let first = state
            .next_part(
                &request(PlatformTransferOperationFlag::GetFirstPart, 0, 4, 0),
                PdrRepositoryState::Available,
                Some(&record),
                UNBOUNDED_CHUNK,
            )
            .unwrap();

        let mut req = request(
            PlatformTransferOperationFlag::GetNextPart,
            first.next_data_transfer_handle,
            4,
            CHANGE_NUMBER,
        );
        req.record_handle = RECORD_HANDLE + 1;
        assert_eq!(
            state.next_part(
                &req,
                PdrRepositoryState::Available,
                Some(&record),
                UNBOUNDED_CHUNK
            ),
            Err(GetPdrError::Command(
                GetPdrCompletionCode::InvalidRecordHandle
            ))
        );
    }

    #[test]
    fn test_first_part_abandons_a_transfer_already_in_progress() {
        let mut state = PdrTransferState::new();
        let record = record();
        let first = state
            .next_part(
                &request(PlatformTransferOperationFlag::GetFirstPart, 0, 4, 0),
                PdrRepositoryState::Available,
                Some(&record),
                UNBOUNDED_CHUNK,
            )
            .unwrap();

        let restart = state
            .next_part(
                &request(PlatformTransferOperationFlag::GetFirstPart, 0, 4, 0),
                PdrRepositoryState::Available,
                Some(&record),
                UNBOUNDED_CHUNK,
            )
            .unwrap();
        assert_eq!(restart.record_data, &RECORD_DATA[..4]);
        // The abandoned handle must not resume the old transfer.
        assert_ne!(
            restart.next_data_transfer_handle,
            first.next_data_transfer_handle
        );
    }

    #[test]
    fn test_repository_update_in_progress_is_rejected() {
        let mut state = PdrTransferState::new();
        assert_eq!(
            state.next_part(
                &request(PlatformTransferOperationFlag::GetFirstPart, 0, 64, 0),
                PdrRepositoryState::UpdateInProgress,
                Some(&record()),
                UNBOUNDED_CHUNK,
            ),
            Err(GetPdrError::Command(
                GetPdrCompletionCode::RepositoryUpdateInProgress
            ))
        );
    }

    #[test]
    fn test_failed_repository_answers_the_generic_error() {
        let mut state = PdrTransferState::new();
        let error = state
            .next_part(
                &request(PlatformTransferOperationFlag::GetFirstPart, 0, 64, 0),
                PdrRepositoryState::Failed,
                Some(&record()),
                UNBOUNDED_CHUNK,
            )
            .unwrap_err();
        assert_eq!(error, GetPdrError::Base(PldmBaseCompletionCode::Error));
        assert_eq!(error.completion_code(), PldmBaseCompletionCode::Error as u8);
    }

    #[test]
    fn test_completion_code_of_a_command_error() {
        assert_eq!(
            GetPdrError::Command(GetPdrCompletionCode::InvalidRecordHandle).completion_code(),
            0x82
        );
    }
}
