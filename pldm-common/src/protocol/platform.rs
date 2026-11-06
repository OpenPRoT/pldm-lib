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

//! Enumerations and shared types for PLDM Type 2, Platform Monitoring and
//! Control (DSP0248 1.3.0).
//!
//! Wire structures declare their `enum8` fields as `u8` because
//! `zerocopy::FromBytes` requires every bit pattern to be valid. The named
//! enumerations here convert after decoding, so an illegal value becomes a
//! completion code rather than a panic.
//!
//! DSP0248 lists most enumeration values by name only; those discriminants
//! follow the order the specification prints them, starting at zero.
//! [`PlatformEventClass`] is the exception, since Table 11 prints explicit
//! hexadecimal values.

use crate::error::PldmError;
use core::convert::TryFrom;
use zerocopy::{FromBytes, Immutable, IntoBytes};

/// PLDM Type 2 command codes.
///
/// DSP0248 clause 30, Table 110. Only the commands OpenPRoT answers are
/// listed; a request carrying any other code is rejected before dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PldmPlatformCmd {
    SetEventReceiver = 0x04,
    GetEventReceiver = 0x05,
    PollForPlatformEventMessage = 0x0B,
    EventMessageSupported = 0x0C,
    EventMessageBufferSize = 0x0D,
    SetNumericSensorEnable = 0x10,
    GetSensorReading = 0x11,
    SetStateSensorEnables = 0x20,
    GetStateSensorReadings = 0x21,
    GetPldmEventLogInfo = 0x40,
    EnablePldmEventLogging = 0x41,
    ClearPldmEventLog = 0x42,
    GetPldmEventLogTimestamp = 0x43,
    SetPldmEventLogTimestamp = 0x44,
    ReadPldmEventLog = 0x45,
    GetPldmEventLogPolicyInfo = 0x46,
    FindPldmEventLogEntry = 0x48,
    GetPdrRepositoryInfo = 0x50,
    GetPdr = 0x51,
    GetPdrRepositorySignature = 0x53,
}

impl TryFrom<u8> for PldmPlatformCmd {
    type Error = PldmError;

    fn try_from(value: u8) -> Result<Self, PldmError> {
        match value {
            0x04 => Ok(PldmPlatformCmd::SetEventReceiver),
            0x05 => Ok(PldmPlatformCmd::GetEventReceiver),
            0x0B => Ok(PldmPlatformCmd::PollForPlatformEventMessage),
            0x0C => Ok(PldmPlatformCmd::EventMessageSupported),
            0x0D => Ok(PldmPlatformCmd::EventMessageBufferSize),
            0x10 => Ok(PldmPlatformCmd::SetNumericSensorEnable),
            0x11 => Ok(PldmPlatformCmd::GetSensorReading),
            0x20 => Ok(PldmPlatformCmd::SetStateSensorEnables),
            0x21 => Ok(PldmPlatformCmd::GetStateSensorReadings),
            0x40 => Ok(PldmPlatformCmd::GetPldmEventLogInfo),
            0x41 => Ok(PldmPlatformCmd::EnablePldmEventLogging),
            0x42 => Ok(PldmPlatformCmd::ClearPldmEventLog),
            0x43 => Ok(PldmPlatformCmd::GetPldmEventLogTimestamp),
            0x44 => Ok(PldmPlatformCmd::SetPldmEventLogTimestamp),
            0x45 => Ok(PldmPlatformCmd::ReadPldmEventLog),
            0x46 => Ok(PldmPlatformCmd::GetPldmEventLogPolicyInfo),
            0x48 => Ok(PldmPlatformCmd::FindPldmEventLogEntry),
            0x50 => Ok(PldmPlatformCmd::GetPdrRepositoryInfo),
            0x51 => Ok(PldmPlatformCmd::GetPdr),
            0x53 => Ok(PldmPlatformCmd::GetPdrRepositorySignature),
            _ => Err(PldmError::UnsupportedCmd),
        }
    }
}

/// Multipart transfer position, DSP0248 Table 69.
///
/// These values differ from the DSP0267 `TransferRespFlag` of the firmware
/// update type, whose `Start` is `0x01`. Type 2 must not reuse that type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PlatformTransferFlag {
    Start = 0x00,
    Middle = 0x01,
    End = 0x04,
    StartAndEnd = 0x05,
}

impl TryFrom<u8> for PlatformTransferFlag {
    type Error = PldmError;

    fn try_from(value: u8) -> Result<Self, PldmError> {
        match value {
            0x00 => Ok(PlatformTransferFlag::Start),
            0x01 => Ok(PlatformTransferFlag::Middle),
            0x04 => Ok(PlatformTransferFlag::End),
            0x05 => Ok(PlatformTransferFlag::StartAndEnd),
            _ => Err(PldmError::InvalidTransferRespFlag),
        }
    }
}

impl PlatformTransferFlag {
    /// Whether a `transferCRC` byte trails the payload.
    ///
    /// DSP0248 places the CRC on `End` only; a single-part transfer answers
    /// `StartAndEnd` and carries no CRC.
    pub fn carries_transfer_crc(&self) -> bool {
        matches!(self, PlatformTransferFlag::End)
    }
}

/// Multipart transfer request operation, DSP0248 Table 69 and Table 16.
///
/// `AcknowledgementOnly` is legal only for PollForPlatformEventMessage; GetPDR
/// and ReadPLDMEventLog accept the first two values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PlatformTransferOperationFlag {
    GetNextPart = 0x00,
    GetFirstPart = 0x01,
    AcknowledgementOnly = 0x02,
}

impl TryFrom<u8> for PlatformTransferOperationFlag {
    type Error = PldmError;

    fn try_from(value: u8) -> Result<Self, PldmError> {
        match value {
            0x00 => Ok(PlatformTransferOperationFlag::GetNextPart),
            0x01 => Ok(PlatformTransferOperationFlag::GetFirstPart),
            0x02 => Ok(PlatformTransferOperationFlag::AcknowledgementOnly),
            _ => Err(PldmError::InvalidTransferOpFlag),
        }
    }
}

/// Size in bytes of the `timestamp104` type.
pub const TIMESTAMP104_LEN: usize = 13;

/// The DSP0240 `timestamp104` type, used by GetPDRRepositoryInfo.
///
/// DSP0240 describes the layout from the most significant byte down; the
/// fields below are in ascending address order, which is how the value appears
/// on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, FromBytes, IntoBytes, Immutable)]
#[repr(C, packed)]
pub struct Timestamp104 {
    /// UTC offset in minutes.
    pub utc_offset_minutes: i16,
    /// Microsecond within the second, a 24-bit integer.
    pub microsecond: [u8; 3],
    pub second: u8,
    pub minute: u8,
    pub hour: u8,
    /// Day within the month, starting at 1.
    pub day: u8,
    /// Month, starting at 1.
    pub month: u8,
    pub year: u16,
    /// `[7:4]` UTC resolution, `[3:0]` time resolution.
    pub resolution: u8,
}

/// Time resolution value marking a `timestamp104` as unknown, DSP0240.
pub const TIMESTAMP104_RESOLUTION_NULL: u8 = 0x0F;

impl Timestamp104 {
    /// Microsecond field widened from its 24-bit wire form.
    pub fn microsecond(&self) -> u32 {
        let m = self.microsecond;
        u32::from(m[0]) | (u32::from(m[1]) << 8) | (u32::from(m[2]) << 16)
    }

    /// Sets the 24-bit microsecond field, discarding bits above bit 23.
    pub fn set_microsecond(&mut self, value: u32) {
        self.microsecond = [value as u8, (value >> 8) as u8, (value >> 16) as u8];
    }

    /// A timestamp the requester must read as "unknown time".
    pub fn unknown() -> Self {
        Timestamp104 {
            utc_offset_minutes: 0,
            microsecond: [0; 3],
            second: 0,
            minute: 0,
            hour: 0,
            day: 1,
            month: 1,
            year: 0,
            resolution: TIMESTAMP104_RESOLUTION_NULL,
        }
    }
}

/// Size in bytes of the PLDM Event Log timestamp.
pub const LOG_TIMESTAMP_LEN: usize = 7;

/// Value marking a log timestamp field as unspecified, DSP0248 Table 60.
pub const LOG_TIMESTAMP_UNSPECIFIED: u8 = 0xFF;

/// The timestamp carried by the PLDM Event Log commands, DSP0248 Table 60.
///
/// This is a different encoding from [`Timestamp104`] and the two are not
/// interchangeable: the log form is 7 bytes and counts seconds since the Unix
/// epoch, while `timestamp104` is 13 bytes of broken-down calendar fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, FromBytes, IntoBytes, Immutable)]
#[repr(C, packed)]
pub struct LogTimestamp {
    /// UTC offset in half-hour increments; `0xFF` means unspecified.
    pub utc_offset_half_hours: i8,
    /// Seconds since midnight UTC 1970-01-01, a 40-bit integer.
    pub seconds: [u8; 5],
    /// Hundredths of a second, 0 to 99; `0xFF` means unspecified.
    pub hundredths: u8,
}

impl LogTimestamp {
    /// Seconds field widened from its 40-bit wire form.
    pub fn seconds(&self) -> u64 {
        let s = self.seconds;
        u64::from(s[0])
            | (u64::from(s[1]) << 8)
            | (u64::from(s[2]) << 16)
            | (u64::from(s[3]) << 24)
            | (u64::from(s[4]) << 32)
    }

    /// Sets the 40-bit seconds field, discarding bits above bit 39.
    pub fn set_seconds(&mut self, value: u64) {
        self.seconds = [
            value as u8,
            (value >> 8) as u8,
            (value >> 16) as u8,
            (value >> 24) as u8,
            (value >> 32) as u8,
        ];
    }

    /// A timestamp whose every field reads as unspecified.
    pub fn unspecified() -> Self {
        LogTimestamp {
            utc_offset_half_hours: LOG_TIMESTAMP_UNSPECIFIED as i8,
            seconds: [0; 5],
            hundredths: LOG_TIMESTAMP_UNSPECIFIED,
        }
    }
}

/// Width and signedness of a numeric sensor reading, DSP0248 Table 31.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SensorDataSize {
    UInt8 = 0,
    SInt8 = 1,
    UInt16 = 2,
    SInt16 = 3,
    UInt32 = 4,
    SInt32 = 5,
    UInt64 = 6,
    SInt64 = 7,
}

impl TryFrom<u8> for SensorDataSize {
    type Error = PldmError;

    fn try_from(value: u8) -> Result<Self, PldmError> {
        match value {
            0 => Ok(SensorDataSize::UInt8),
            1 => Ok(SensorDataSize::SInt8),
            2 => Ok(SensorDataSize::UInt16),
            3 => Ok(SensorDataSize::SInt16),
            4 => Ok(SensorDataSize::UInt32),
            5 => Ok(SensorDataSize::SInt32),
            6 => Ok(SensorDataSize::UInt64),
            7 => Ok(SensorDataSize::SInt64),
            _ => Err(PldmError::InvalidSensorDataSize),
        }
    }
}

impl SensorDataSize {
    /// Byte width of a `presentReading` or threshold field of this size.
    pub fn byte_len(&self) -> usize {
        match self {
            SensorDataSize::UInt8 | SensorDataSize::SInt8 => 1,
            SensorDataSize::UInt16 | SensorDataSize::SInt16 => 2,
            SensorDataSize::UInt32 | SensorDataSize::SInt32 => 4,
            SensorDataSize::UInt64 | SensorDataSize::SInt64 => 8,
        }
    }
}

/// Operational state of a sensor, DSP0248 Table 31.
///
/// SetNumericSensorEnable and SetStateSensorEnables accept the first three
/// values only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SensorOperationalState {
    Enabled = 0,
    Disabled = 1,
    Unavailable = 2,
    StatusUnknown = 3,
    Failed = 4,
    Initializing = 5,
    ShuttingDown = 6,
    InTest = 7,
}

impl TryFrom<u8> for SensorOperationalState {
    type Error = PldmError;

    fn try_from(value: u8) -> Result<Self, PldmError> {
        match value {
            0 => Ok(SensorOperationalState::Enabled),
            1 => Ok(SensorOperationalState::Disabled),
            2 => Ok(SensorOperationalState::Unavailable),
            3 => Ok(SensorOperationalState::StatusUnknown),
            4 => Ok(SensorOperationalState::Failed),
            5 => Ok(SensorOperationalState::Initializing),
            6 => Ok(SensorOperationalState::ShuttingDown),
            7 => Ok(SensorOperationalState::InTest),
            _ => Err(PldmError::InvalidSensorOperationalState),
        }
    }
}

/// Sensor event message enable setting, DSP0248 Table 30.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SensorEventMessageEnable {
    NoChange = 0,
    DisableEvents = 1,
    EnableEvents = 2,
    EnableOpEventsOnly = 3,
    EnableStateEventsOnly = 4,
}

impl TryFrom<u8> for SensorEventMessageEnable {
    type Error = PldmError;

    fn try_from(value: u8) -> Result<Self, PldmError> {
        match value {
            0 => Ok(SensorEventMessageEnable::NoChange),
            1 => Ok(SensorEventMessageEnable::DisableEvents),
            2 => Ok(SensorEventMessageEnable::EnableEvents),
            3 => Ok(SensorEventMessageEnable::EnableOpEventsOnly),
            4 => Ok(SensorEventMessageEnable::EnableStateEventsOnly),
            _ => Err(PldmError::InvalidSensorEventMessageEnable),
        }
    }
}

/// Initialization Agent handling for a sensor, DSP0248 Table 79 and Table 81.
///
/// OpenPRoT publishes `NoInit`, because it admits no Initialization PDRs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SensorInit {
    NoInit = 0,
    UseInitPdr = 1,
    EnableSensor = 2,
    DisableSensor = 3,
}

impl TryFrom<u8> for SensorInit {
    type Error = PldmError;

    fn try_from(value: u8) -> Result<Self, PldmError> {
        match value {
            0 => Ok(SensorInit::NoInit),
            1 => Ok(SensorInit::UseInitPdr),
            2 => Ok(SensorInit::EnableSensor),
            3 => Ok(SensorInit::DisableSensor),
            _ => Err(PldmError::InvalidSensorInit),
        }
    }
}

/// State of the PDR repository, DSP0248 Table 68.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PdrRepositoryState {
    Available = 0,
    UpdateInProgress = 1,
    Failed = 2,
}

impl TryFrom<u8> for PdrRepositoryState {
    type Error = PldmError;

    fn try_from(value: u8) -> Result<Self, PldmError> {
        match value {
            0 => Ok(PdrRepositoryState::Available),
            1 => Ok(PdrRepositoryState::UpdateInProgress),
            2 => Ok(PdrRepositoryState::Failed),
            _ => Err(PldmError::InvalidPdrRepositoryState),
        }
    }
}

/// Whether a Terminus Locator PDR describes a live terminus, DSP0248 Table 78.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PdrValidity {
    NotValid = 0,
    Valid = 1,
}

impl TryFrom<u8> for PdrValidity {
    type Error = PldmError;

    fn try_from(value: u8) -> Result<Self, PldmError> {
        match value {
            0 => Ok(PdrValidity::NotValid),
            1 => Ok(PdrValidity::Valid),
            _ => Err(PldmError::InvalidPdrValidity),
        }
    }
}

/// Addressing form of a Terminus Locator PDR, DSP0248 Table 78.
///
/// The value selects the layout of the `terminusLocatorValue` that follows.
/// OpenPRoT emits `MctpEid` only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TerminusLocatorType {
    Uid = 0,
    MctpEid = 1,
    SmbusRelative = 2,
    SystemSoftware = 3,
    NcSi = 4,
}

impl TryFrom<u8> for TerminusLocatorType {
    type Error = PldmError;

    fn try_from(value: u8) -> Result<Self, PldmError> {
        match value {
            0 => Ok(TerminusLocatorType::Uid),
            1 => Ok(TerminusLocatorType::MctpEid),
            2 => Ok(TerminusLocatorType::SmbusRelative),
            3 => Ok(TerminusLocatorType::SystemSoftware),
            4 => Ok(TerminusLocatorType::NcSi),
            _ => Err(PldmError::InvalidTerminusLocatorType),
        }
    }
}

/// PDR type numbers, DSP0248 Table 77.
///
/// Only the types OpenPRoT publishes or is considering are listed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PdrType {
    TerminusLocator = 1,
    NumericSensor = 2,
    StateSensor = 4,
    OemStateSet = 8,
    EventLog = 19,
    CompactNumericSensor = 21,
}

impl TryFrom<u8> for PdrType {
    type Error = PldmError;

    fn try_from(value: u8) -> Result<Self, PldmError> {
        match value {
            1 => Ok(PdrType::TerminusLocator),
            2 => Ok(PdrType::NumericSensor),
            4 => Ok(PdrType::StateSensor),
            8 => Ok(PdrType::OemStateSet),
            19 => Ok(PdrType::EventLog),
            21 => Ok(PdrType::CompactNumericSensor),
            _ => Err(PldmError::InvalidPdrType),
        }
    }
}

/// `PDRHeaderVersion` for PDRs defined by DSP0248, Table 76.
pub const PDR_HEADER_VERSION: u8 = 0x01;

/// Event message generation mode set by SetEventReceiver, DSP0248 Table 13.
///
/// OpenPRoT accepts `EnablePolling` and answers the rest with
/// [`SetEventReceiverCompletionCode::EnableMethodNotSupported`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum EventMessageGlobalEnable {
    Disable = 0,
    EnableAsync = 1,
    EnablePolling = 2,
    EnableAsyncKeepAlive = 3,
}

impl TryFrom<u8> for EventMessageGlobalEnable {
    type Error = PldmError;

    fn try_from(value: u8) -> Result<Self, PldmError> {
        match value {
            0 => Ok(EventMessageGlobalEnable::Disable),
            1 => Ok(EventMessageGlobalEnable::EnableAsync),
            2 => Ok(EventMessageGlobalEnable::EnablePolling),
            3 => Ok(EventMessageGlobalEnable::EnableAsyncKeepAlive),
            _ => Err(PldmError::InvalidEventMessageGlobalEnable),
        }
    }
}

/// Messaging style most recently configured, DSP0248 Table 17.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SynchronyConfiguration {
    NotConfigured = 0,
    Asynchronous = 1,
    Synchronous = 2,
    AsynchronousWithHeartbeat = 3,
}

impl TryFrom<u8> for SynchronyConfiguration {
    type Error = PldmError;

    fn try_from(value: u8) -> Result<Self, PldmError> {
        match value {
            0 => Ok(SynchronyConfiguration::NotConfigured),
            1 => Ok(SynchronyConfiguration::Asynchronous),
            2 => Ok(SynchronyConfiguration::Synchronous),
            3 => Ok(SynchronyConfiguration::AsynchronousWithHeartbeat),
            _ => Err(PldmError::InvalidSynchronyConfiguration),
        }
    }
}

/// Bit positions of `synchronyConfigurationSupported`, DSP0248 Table 17.
///
/// Bit 0 is reserved and shall be zero. OpenPRoT sets the synchronous bit
/// alone, so the BMC learns the terminus is poll-only without first trying
/// `enableAsync`.
pub mod synchrony_support {
    pub const ASYNCHRONOUS: u8 = 1 << 1;
    pub const SYNCHRONOUS: u8 = 1 << 2;
    pub const ASYNCHRONOUS_WITH_HEARTBEAT: u8 = 1 << 3;
}

/// Event class of a platform event message, DSP0248 Table 11.
///
/// Table 11 prints explicit values, and `Oem` covers the range `0xF0` to
/// `0xFE`, so this enumeration carries the raw value rather than a
/// discriminant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformEventClass {
    SensorEvent,
    EffecterEvent,
    RedfishTaskExecutedEvent,
    RedfishMessageEvent,
    PldmPdrRepositoryChgEvent,
    PldmMessagePollEvent,
    HeartbeatTimerElapsedEvent,
    CperEvent,
    Oem(u8),
}

impl TryFrom<u8> for PlatformEventClass {
    type Error = PldmError;

    fn try_from(value: u8) -> Result<Self, PldmError> {
        match value {
            0x00 => Ok(PlatformEventClass::SensorEvent),
            0x01 => Ok(PlatformEventClass::EffecterEvent),
            0x02 => Ok(PlatformEventClass::RedfishTaskExecutedEvent),
            0x03 => Ok(PlatformEventClass::RedfishMessageEvent),
            0x04 => Ok(PlatformEventClass::PldmPdrRepositoryChgEvent),
            0x05 => Ok(PlatformEventClass::PldmMessagePollEvent),
            0x06 => Ok(PlatformEventClass::HeartbeatTimerElapsedEvent),
            0x07 => Ok(PlatformEventClass::CperEvent),
            0xF0..=0xFE => Ok(PlatformEventClass::Oem(value)),
            _ => Err(PldmError::InvalidPlatformEventClass),
        }
    }
}

impl From<PlatformEventClass> for u8 {
    fn from(class: PlatformEventClass) -> u8 {
        match class {
            PlatformEventClass::SensorEvent => 0x00,
            PlatformEventClass::EffecterEvent => 0x01,
            PlatformEventClass::RedfishTaskExecutedEvent => 0x02,
            PlatformEventClass::RedfishMessageEvent => 0x03,
            PlatformEventClass::PldmPdrRepositoryChgEvent => 0x04,
            PlatformEventClass::PldmMessagePollEvent => 0x05,
            PlatformEventClass::HeartbeatTimerElapsedEvent => 0x06,
            PlatformEventClass::CperEvent => 0x07,
            PlatformEventClass::Oem(value) => value,
        }
    }
}

/// Transport binding of an event receiver address, DSP0245.
///
/// OpenPRoT rejects anything other than MCTP with
/// [`SetEventReceiverCompletionCode::InvalidProtocolType`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TransportProtocolType {
    Mctp = 0x00,
}

impl TryFrom<u8> for TransportProtocolType {
    type Error = PldmError;

    fn try_from(value: u8) -> Result<Self, PldmError> {
        match value {
            0x00 => Ok(TransportProtocolType::Mctp),
            _ => Err(PldmError::InvalidTransportProtocolType),
        }
    }
}

/// Buffer size in effect when EventMessageBufferSize is never issued,
/// DSP0248 clause 16.9. Also the smallest legal negotiated size.
pub const EVENT_MESSAGE_BUFFER_SIZE_DEFAULT: u16 = 256;

/// `eventID` meaning the terminus event queue is empty, DSP0248 Table 16.
pub const EVENT_ID_NONE: u16 = 0x0000;

/// `eventID` used mid-transfer and returned for an acknowledgement-only
/// request against a non-empty queue, DSP0248 Table 16.
pub const EVENT_ID_IN_PROGRESS: u16 = 0xFFFF;

/// Operational status of the PLDM Event Log, DSP0248 Table 57.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum LogOperationalStatus {
    LoggingDisabled = 0,
    EnabledReady = 1,
    ClearInProgress = 2,
    EnabledFull = 3,
    FailedLoggingDisabled = 4,
    FailedDisabled = 5,
    Corrupted = 6,
}

impl TryFrom<u8> for LogOperationalStatus {
    type Error = PldmError;

    fn try_from(value: u8) -> Result<Self, PldmError> {
        match value {
            0 => Ok(LogOperationalStatus::LoggingDisabled),
            1 => Ok(LogOperationalStatus::EnabledReady),
            2 => Ok(LogOperationalStatus::ClearInProgress),
            3 => Ok(LogOperationalStatus::EnabledFull),
            4 => Ok(LogOperationalStatus::FailedLoggingDisabled),
            5 => Ok(LogOperationalStatus::FailedDisabled),
            6 => Ok(LogOperationalStatus::Corrupted),
            _ => Err(PldmError::InvalidLogOperationalStatus),
        }
    }
}

/// Log clearing policy, DSP0248 clause 13.4 and Table 57.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum LogClearingPolicy {
    FillAndStop = 0,
    Fifo = 1,
    ClearOnAge = 2,
}

impl TryFrom<u8> for LogClearingPolicy {
    type Error = PldmError;

    fn try_from(value: u8) -> Result<Self, PldmError> {
        match value {
            0 => Ok(LogClearingPolicy::FillAndStop),
            1 => Ok(LogClearingPolicy::Fifo),
            2 => Ok(LogClearingPolicy::ClearOnAge),
            _ => Err(PldmError::InvalidLogClearingPolicy),
        }
    }
}

/// Bit positions of `supportedLogClearingPolicies`, DSP0248 Table 99.
pub mod log_clearing_policy_support {
    pub const FILL_AND_STOP: u8 = 1 << 0;
    pub const FIFO: u8 = 1 << 1;
    pub const CLEAR_ON_AGE: u8 = 1 << 2;
}

/// Requested logging state, DSP0248 Table 58.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum EnableLogging {
    DisableLogging = 0,
    EnableLogging = 1,
}

impl TryFrom<u8> for EnableLogging {
    type Error = PldmError;

    fn try_from(value: u8) -> Result<Self, PldmError> {
        match value {
            0 => Ok(EnableLogging::DisableLogging),
            1 => Ok(EnableLogging::EnableLogging),
            _ => Err(PldmError::InvalidEnableLogging),
        }
    }
}

/// Whether setting the log clock also logs the change, DSP0248 Table 61.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum LogUpdateEvent {
    NoEvent = 0,
    LogEvent = 1,
}

impl TryFrom<u8> for LogUpdateEvent {
    type Error = PldmError;

    fn try_from(value: u8) -> Result<Self, PldmError> {
        match value {
            0 => Ok(LogUpdateEvent::NoEvent),
            1 => Ok(LogUpdateEvent::LogEvent),
            _ => Err(PldmError::InvalidLogUpdateEvent),
        }
    }
}

/// Fragmentation state of a ReadPLDMEventLog response, DSP0248 Table 62.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SplitEntry {
    Full = 0,
    FirstFragment = 1,
    MiddleFragment = 2,
    LastFragment = 3,
}

impl TryFrom<u8> for SplitEntry {
    type Error = PldmError;

    fn try_from(value: u8) -> Result<Self, PldmError> {
        match value {
            0 => Ok(SplitEntry::Full),
            1 => Ok(SplitEntry::FirstFragment),
            2 => Ok(SplitEntry::MiddleFragment),
            3 => Ok(SplitEntry::LastFragment),
            _ => Err(PldmError::InvalidSplitEntry),
        }
    }
}

impl SplitEntry {
    /// Whether a `transferCRC` byte trails the entry data.
    ///
    /// DSP0248 places it on `LastFragment` only, so a `Full` entry carries no
    /// CRC even though it is also the end of the entry.
    pub fn carries_transfer_crc(&self) -> bool {
        matches!(self, SplitEntry::LastFragment)
    }
}

/// Search direction of FindPLDMEventLogEntry, DSP0248 Table 66.
///
/// The first two carry a `compareTimestamp` in the request; the last two do
/// not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum EntrySearchType {
    NewerThan = 0,
    OlderThan = 1,
    OffsetFromStart = 2,
    OffsetFromEnd = 3,
}

impl TryFrom<u8> for EntrySearchType {
    type Error = PldmError;

    fn try_from(value: u8) -> Result<Self, PldmError> {
        match value {
            0 => Ok(EntrySearchType::NewerThan),
            1 => Ok(EntrySearchType::OlderThan),
            2 => Ok(EntrySearchType::OffsetFromStart),
            3 => Ok(EntrySearchType::OffsetFromEnd),
            _ => Err(PldmError::InvalidEntrySearchType),
        }
    }
}

impl EntrySearchType {
    /// Whether the request carries the three `compareTimestamp` fields.
    pub fn carries_compare_timestamp(&self) -> bool {
        matches!(
            self,
            EntrySearchType::NewerThan | EntrySearchType::OlderThan
        )
    }
}

/// `entryID` returned when FindPLDMEventLogEntry matches nothing,
/// DSP0248 Table 66.
pub const ENTRY_ID_NOT_FOUND: u32 = 0xFFFF_FFFF;

// ---------------------------------------------------------------------------
// Command-specific completion codes
//
// DSP0248 reuses the values 0x80 upwards with a different meaning per command,
// unlike DSP0267 whose codes share one space. The `pldm_completion_code!`
// macro models the DSP0267 shape, so Type 2 defines one enumeration per
// command instead. Values below 0x80 belong to PldmBaseCompletionCode.
// ---------------------------------------------------------------------------

/// SetEventReceiver completion codes, DSP0248 Table 13.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SetEventReceiverCompletionCode {
    InvalidProtocolType = 0x80,
    EnableMethodNotSupported = 0x81,
    HeartbeatFrequencyTooHigh = 0x82,
}

impl TryFrom<u8> for SetEventReceiverCompletionCode {
    type Error = PldmError;

    fn try_from(value: u8) -> Result<Self, PldmError> {
        match value {
            0x80 => Ok(SetEventReceiverCompletionCode::InvalidProtocolType),
            0x81 => Ok(SetEventReceiverCompletionCode::EnableMethodNotSupported),
            0x82 => Ok(SetEventReceiverCompletionCode::HeartbeatFrequencyTooHigh),
            _ => Err(PldmError::InvalidCompletionCode),
        }
    }
}

/// EventMessageSupported completion codes, DSP0248 Table 17.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum EventMessageSupportedCompletionCode {
    UnsupportedEventFormatVersion = 0x81,
}

impl TryFrom<u8> for EventMessageSupportedCompletionCode {
    type Error = PldmError;

    fn try_from(value: u8) -> Result<Self, PldmError> {
        match value {
            0x81 => Ok(EventMessageSupportedCompletionCode::UnsupportedEventFormatVersion),
            _ => Err(PldmError::InvalidCompletionCode),
        }
    }
}

/// EventMessageBufferSize completion codes, DSP0248 Table 18.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum EventMessageBufferSizeCompletionCode {
    IllegalMessageBufferSize = 0x80,
}

impl TryFrom<u8> for EventMessageBufferSizeCompletionCode {
    type Error = PldmError;

    fn try_from(value: u8) -> Result<Self, PldmError> {
        match value {
            0x80 => Ok(EventMessageBufferSizeCompletionCode::IllegalMessageBufferSize),
            _ => Err(PldmError::InvalidCompletionCode),
        }
    }
}

/// PollForPlatformEventMessage completion codes, DSP0248 Table 16.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PollForPlatformEventMessageCompletionCode {
    InvalidDataTransferHandle = 0x80,
    UnsupportedEventFormatVersion = 0x81,
    EventIdNotValid = 0x82,
}

impl TryFrom<u8> for PollForPlatformEventMessageCompletionCode {
    type Error = PldmError;

    fn try_from(value: u8) -> Result<Self, PldmError> {
        match value {
            0x80 => Ok(PollForPlatformEventMessageCompletionCode::InvalidDataTransferHandle),
            0x81 => Ok(PollForPlatformEventMessageCompletionCode::UnsupportedEventFormatVersion),
            0x82 => Ok(PollForPlatformEventMessageCompletionCode::EventIdNotValid),
            _ => Err(PldmError::InvalidCompletionCode),
        }
    }
}

/// SetNumericSensorEnable and SetStateSensorEnables completion codes,
/// DSP0248 Table 30 and Table 39.
///
/// `InvalidSensorOperationalState` is defined for the numeric command only;
/// the state command shares the other two values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SetSensorEnableCompletionCode {
    InvalidSensorId = 0x80,
    InvalidSensorOperationalState = 0x81,
    EventGenerationNotSupported = 0x82,
}

impl TryFrom<u8> for SetSensorEnableCompletionCode {
    type Error = PldmError;

    fn try_from(value: u8) -> Result<Self, PldmError> {
        match value {
            0x80 => Ok(SetSensorEnableCompletionCode::InvalidSensorId),
            0x81 => Ok(SetSensorEnableCompletionCode::InvalidSensorOperationalState),
            0x82 => Ok(SetSensorEnableCompletionCode::EventGenerationNotSupported),
            _ => Err(PldmError::InvalidCompletionCode),
        }
    }
}

/// GetSensorReading and GetStateSensorReadings completion codes,
/// DSP0248 Table 31 and Table 41.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum GetSensorReadingCompletionCode {
    InvalidSensorId = 0x80,
    RearmUnavailableInPresentState = 0x81,
}

impl TryFrom<u8> for GetSensorReadingCompletionCode {
    type Error = PldmError;

    fn try_from(value: u8) -> Result<Self, PldmError> {
        match value {
            0x80 => Ok(GetSensorReadingCompletionCode::InvalidSensorId),
            0x81 => Ok(GetSensorReadingCompletionCode::RearmUnavailableInPresentState),
            _ => Err(PldmError::InvalidCompletionCode),
        }
    }
}

/// ReadPLDMEventLog completion codes, DSP0248 Table 62.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ReadPldmEventLogCompletionCode {
    InvalidTransferOperationFlag = 0x81,
    InvalidEntryId = 0x82,
}

impl TryFrom<u8> for ReadPldmEventLogCompletionCode {
    type Error = PldmError;

    fn try_from(value: u8) -> Result<Self, PldmError> {
        match value {
            0x81 => Ok(ReadPldmEventLogCompletionCode::InvalidTransferOperationFlag),
            0x82 => Ok(ReadPldmEventLogCompletionCode::InvalidEntryId),
            _ => Err(PldmError::InvalidCompletionCode),
        }
    }
}

/// FindPLDMEventLogEntry completion codes, DSP0248 Table 66.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FindPldmEventLogEntryCompletionCode {
    InvalidSearchType = 0x80,
}

impl TryFrom<u8> for FindPldmEventLogEntryCompletionCode {
    type Error = PldmError;

    fn try_from(value: u8) -> Result<Self, PldmError> {
        match value {
            0x80 => Ok(FindPldmEventLogEntryCompletionCode::InvalidSearchType),
            _ => Err(PldmError::InvalidCompletionCode),
        }
    }
}

/// GetPDR completion codes, DSP0248 Table 69.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum GetPdrCompletionCode {
    InvalidDataTransferHandle = 0x80,
    InvalidTransferOperationFlag = 0x81,
    InvalidRecordHandle = 0x82,
    InvalidRecordChangeNumber = 0x83,
    TransferTimeout = 0x84,
    RepositoryUpdateInProgress = 0x85,
}

impl TryFrom<u8> for GetPdrCompletionCode {
    type Error = PldmError;

    fn try_from(value: u8) -> Result<Self, PldmError> {
        match value {
            0x80 => Ok(GetPdrCompletionCode::InvalidDataTransferHandle),
            0x81 => Ok(GetPdrCompletionCode::InvalidTransferOperationFlag),
            0x82 => Ok(GetPdrCompletionCode::InvalidRecordHandle),
            0x83 => Ok(GetPdrCompletionCode::InvalidRecordChangeNumber),
            0x84 => Ok(GetPdrCompletionCode::TransferTimeout),
            0x85 => Ok(GetPdrCompletionCode::RepositoryUpdateInProgress),
            _ => Err(PldmError::InvalidCompletionCode),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::codec::PldmCodec;
    use crate::protocol::base::{PldmSupportedType, TransferRespFlag};

    #[test]
    fn test_platform_cmd_round_trip() {
        for (code, cmd) in [
            (0x04u8, PldmPlatformCmd::SetEventReceiver),
            (0x0B, PldmPlatformCmd::PollForPlatformEventMessage),
            (0x21, PldmPlatformCmd::GetStateSensorReadings),
            (0x48, PldmPlatformCmd::FindPldmEventLogEntry),
            (0x53, PldmPlatformCmd::GetPdrRepositorySignature),
        ] {
            assert_eq!(PldmPlatformCmd::try_from(code).unwrap(), cmd);
            assert_eq!(cmd as u8, code);
        }
    }

    #[test]
    fn test_platform_cmd_rejects_unimplemented_opcode() {
        // 0x0A PlatformEventMessage needs the RoT to be a requester, and 0x52
        // FindPDR is not implemented; both must fall through to the caller's
        // ERROR_UNSUPPORTED_PLDM_CMD path rather than decode.
        assert_eq!(
            PldmPlatformCmd::try_from(0x0A),
            Err(PldmError::UnsupportedCmd)
        );
        assert_eq!(
            PldmPlatformCmd::try_from(0x52),
            Err(PldmError::UnsupportedCmd)
        );
    }

    #[test]
    fn test_platform_type_code_is_two() {
        assert_eq!(PldmSupportedType::Platform as u8, 0x02);
    }

    #[test]
    fn test_platform_transfer_flag_start_differs_from_firmware_update() {
        // DSP0248 Table 69 numbers Start as 0x00 while DSP0267 numbers it as
        // 0x01. Reusing TransferRespFlag here would put the wrong byte on the
        // wire, so the two must not agree.
        assert_eq!(PlatformTransferFlag::Start as u8, 0x00);
        assert_eq!(TransferRespFlag::Start as u8, 0x01);
    }

    #[test]
    fn test_platform_transfer_flag_round_trip() {
        for (code, flag) in [
            (0x00u8, PlatformTransferFlag::Start),
            (0x01, PlatformTransferFlag::Middle),
            (0x04, PlatformTransferFlag::End),
            (0x05, PlatformTransferFlag::StartAndEnd),
        ] {
            assert_eq!(PlatformTransferFlag::try_from(code).unwrap(), flag);
            assert_eq!(flag as u8, code);
        }
        // 0x02 and 0x03 are unassigned in Table 69.
        assert_eq!(
            PlatformTransferFlag::try_from(0x02),
            Err(PldmError::InvalidTransferRespFlag)
        );
    }

    #[test]
    fn test_transfer_crc_rides_end_but_not_start_and_end() {
        // A single-part transfer answers StartAndEnd and carries no CRC;
        // appending one there shifts every following field for the requester.
        assert!(PlatformTransferFlag::End.carries_transfer_crc());
        assert!(!PlatformTransferFlag::StartAndEnd.carries_transfer_crc());
        assert!(!PlatformTransferFlag::Start.carries_transfer_crc());
        assert!(!PlatformTransferFlag::Middle.carries_transfer_crc());
    }

    #[test]
    fn test_transfer_operation_flag_admits_acknowledgement_only() {
        // GetPDR knows two values; PollForPlatformEventMessage adds a third,
        // so the shared type has to carry it.
        assert_eq!(
            PlatformTransferOperationFlag::try_from(0x02).unwrap(),
            PlatformTransferOperationFlag::AcknowledgementOnly
        );
        assert_eq!(
            PlatformTransferOperationFlag::try_from(0x03),
            Err(PldmError::InvalidTransferOpFlag)
        );
    }

    #[test]
    fn test_timestamp104_is_thirteen_bytes_on_the_wire() {
        assert_eq!(core::mem::size_of::<Timestamp104>(), TIMESTAMP104_LEN);
    }

    #[test]
    fn test_timestamp104_encodes_little_endian_fields() {
        let mut ts = Timestamp104::unknown();
        ts.utc_offset_minutes = 60; // UTC+1
        ts.set_microsecond(0x00_1234);
        ts.second = 45;
        ts.minute = 30;
        ts.hour = 13;
        ts.day = 6;
        ts.month = 8;
        ts.year = 2026;
        ts.resolution = 0x16; // hour UTC resolution, second time resolution

        let mut buffer = [0u8; TIMESTAMP104_LEN];
        assert_eq!(ts.encode(&mut buffer).unwrap(), TIMESTAMP104_LEN);
        assert_eq!(
            buffer,
            [
                0x3C, 0x00, // utcOffset 60 minutes, little endian
                0x34, 0x12, 0x00, // microsecond 0x001234
                45, 30, 13, 6, 8, // second, minute, hour, day, month
                0xEA, 0x07, // year 2026 = 0x07EA, little endian
                0x16,
            ]
        );
        assert_eq!(Timestamp104::decode(&buffer).unwrap(), ts);
    }

    #[test]
    fn test_timestamp104_microsecond_is_twenty_four_bits() {
        let mut ts = Timestamp104::unknown();
        ts.set_microsecond(0xFF_FFFF);
        assert_eq!(ts.microsecond(), 0xFF_FFFF);
        // Bits above bit 23 have no field to live in and are dropped.
        ts.set_microsecond(0x0100_0000);
        assert_eq!(ts.microsecond(), 0);
    }

    #[test]
    fn test_timestamp104_unknown_uses_null_resolution() {
        // DSP0240 reads the whole timestamp as unknown when the time
        // resolution nibble is 15, which is what a RoT with no wall clock
        // must report.
        assert_eq!(
            Timestamp104::unknown().resolution & 0x0F,
            TIMESTAMP104_RESOLUTION_NULL
        );
    }

    #[test]
    fn test_log_timestamp_is_seven_bytes_on_the_wire() {
        // Distinct from the 13-byte timestamp104; mixing the two would
        // misalign every field after it.
        assert_eq!(core::mem::size_of::<LogTimestamp>(), LOG_TIMESTAMP_LEN);
        assert_ne!(LOG_TIMESTAMP_LEN, TIMESTAMP104_LEN);
    }

    #[test]
    fn test_log_timestamp_encodes_uint40_seconds() {
        let mut ts = LogTimestamp::unspecified();
        ts.utc_offset_half_hours = 16; // UTC+8
        ts.set_seconds(0x01_0203_0405);
        ts.hundredths = 99;

        let mut buffer = [0u8; LOG_TIMESTAMP_LEN];
        assert_eq!(ts.encode(&mut buffer).unwrap(), LOG_TIMESTAMP_LEN);
        assert_eq!(buffer, [16, 0x05, 0x04, 0x03, 0x02, 0x01, 99]);
        assert_eq!(LogTimestamp::decode(&buffer).unwrap(), ts);
        assert_eq!(ts.seconds(), 0x01_0203_0405);
    }

    #[test]
    fn test_log_timestamp_seconds_saturates_at_forty_bits() {
        let mut ts = LogTimestamp::unspecified();
        ts.set_seconds(u64::MAX);
        assert_eq!(ts.seconds(), 0xFF_FFFF_FFFF);
    }

    #[test]
    fn test_sensor_data_size_byte_len() {
        // The presentReading field length follows this value, so a wrong
        // width truncates or overruns the response.
        assert_eq!(SensorDataSize::UInt8.byte_len(), 1);
        assert_eq!(SensorDataSize::SInt16.byte_len(), 2);
        assert_eq!(SensorDataSize::UInt32.byte_len(), 4);
        assert_eq!(SensorDataSize::SInt64.byte_len(), 8);
        assert_eq!(
            SensorDataSize::try_from(8),
            Err(PldmError::InvalidSensorDataSize)
        );
    }

    #[test]
    fn test_sensor_operational_state_round_trip() {
        assert_eq!(
            SensorOperationalState::try_from(1).unwrap(),
            SensorOperationalState::Disabled
        );
        assert_eq!(SensorOperationalState::InTest as u8, 7);
        assert_eq!(
            SensorOperationalState::try_from(8),
            Err(PldmError::InvalidSensorOperationalState)
        );
    }

    #[test]
    fn test_sensor_event_message_enable_round_trip() {
        assert_eq!(
            SensorEventMessageEnable::try_from(0).unwrap(),
            SensorEventMessageEnable::NoChange
        );
        assert_eq!(SensorEventMessageEnable::EnableStateEventsOnly as u8, 4);
        assert_eq!(
            SensorEventMessageEnable::try_from(5),
            Err(PldmError::InvalidSensorEventMessageEnable)
        );
    }

    #[test]
    fn test_sensor_init_round_trip() {
        assert_eq!(SensorInit::try_from(0).unwrap(), SensorInit::NoInit);
        assert_eq!(SensorInit::DisableSensor as u8, 3);
        assert_eq!(SensorInit::try_from(4), Err(PldmError::InvalidSensorInit));
    }

    #[test]
    fn test_pdr_repository_state_round_trip() {
        assert_eq!(
            PdrRepositoryState::try_from(0).unwrap(),
            PdrRepositoryState::Available
        );
        assert_eq!(PdrRepositoryState::Failed as u8, 2);
        assert_eq!(
            PdrRepositoryState::try_from(3),
            Err(PldmError::InvalidPdrRepositoryState)
        );
    }

    #[test]
    fn test_pdr_validity_round_trip() {
        // notValid is zero, so a zeroed PDR reads as "ignore me" rather than
        // as a live terminus.
        assert_eq!(PdrValidity::try_from(0).unwrap(), PdrValidity::NotValid);
        assert_eq!(PdrValidity::Valid as u8, 1);
        assert_eq!(PdrValidity::try_from(2), Err(PldmError::InvalidPdrValidity));
    }

    #[test]
    fn test_terminus_locator_type_round_trip() {
        assert_eq!(
            TerminusLocatorType::try_from(1).unwrap(),
            TerminusLocatorType::MctpEid
        );
        assert_eq!(TerminusLocatorType::NcSi as u8, 4);
        assert_eq!(
            TerminusLocatorType::try_from(5),
            Err(PldmError::InvalidTerminusLocatorType)
        );
    }

    #[test]
    fn test_pdr_type_numbers() {
        // Table 77 numbers; type 4 is the one the OpenPRoT PDR type list has
        // yet to admit, and type 19 backs the event log tier.
        assert_eq!(PdrType::TerminusLocator as u8, 1);
        assert_eq!(PdrType::NumericSensor as u8, 2);
        assert_eq!(PdrType::StateSensor as u8, 4);
        assert_eq!(PdrType::OemStateSet as u8, 8);
        assert_eq!(PdrType::EventLog as u8, 19);
        assert_eq!(PdrType::CompactNumericSensor as u8, 21);
        // Type 3 is Numeric Sensor Initialization, which OpenPRoT rejects.
        assert_eq!(PdrType::try_from(3), Err(PldmError::InvalidPdrType));
    }

    #[test]
    fn test_event_message_global_enable_round_trip() {
        assert_eq!(
            EventMessageGlobalEnable::try_from(2).unwrap(),
            EventMessageGlobalEnable::EnablePolling
        );
        assert_eq!(EventMessageGlobalEnable::EnableAsyncKeepAlive as u8, 3);
        assert_eq!(
            EventMessageGlobalEnable::try_from(4),
            Err(PldmError::InvalidEventMessageGlobalEnable)
        );
    }

    #[test]
    fn test_synchrony_support_bits() {
        // Bit 0 is reserved and shall stay clear; a poll-only terminus
        // advertises bit 2 alone so the BMC never tries enableAsync.
        assert_eq!(synchrony_support::SYNCHRONOUS, 0b0000_0100);
        assert_eq!(synchrony_support::ASYNCHRONOUS, 0b0000_0010);
        assert_eq!(synchrony_support::ASYNCHRONOUS_WITH_HEARTBEAT, 0b0000_1000);
        assert_eq!(synchrony_support::SYNCHRONOUS & 0b0000_0001, 0);
    }

    #[test]
    fn test_synchrony_configuration_round_trip() {
        assert_eq!(
            SynchronyConfiguration::try_from(2).unwrap(),
            SynchronyConfiguration::Synchronous
        );
        assert_eq!(
            SynchronyConfiguration::try_from(4),
            Err(PldmError::InvalidSynchronyConfiguration)
        );
    }

    #[test]
    fn test_platform_event_class_explicit_values() {
        for (code, class) in [
            (0x00u8, PlatformEventClass::SensorEvent),
            (0x04, PlatformEventClass::PldmPdrRepositoryChgEvent),
            (0x07, PlatformEventClass::CperEvent),
        ] {
            assert_eq!(PlatformEventClass::try_from(code).unwrap(), class);
            assert_eq!(u8::from(class), code);
        }
    }

    #[test]
    fn test_platform_event_class_oem_covers_a_range() {
        // Table 11 gives oemEvent the range 0xF0 to 0xFE, so the variant has
        // to carry the value instead of naming a single discriminant.
        for code in [0xF0u8, 0xF7, 0xFE] {
            assert_eq!(
                PlatformEventClass::try_from(code).unwrap(),
                PlatformEventClass::Oem(code)
            );
            assert_eq!(u8::from(PlatformEventClass::Oem(code)), code);
        }
        // 0x08 to 0xEF and 0xFF are reserved.
        assert_eq!(
            PlatformEventClass::try_from(0x08),
            Err(PldmError::InvalidPlatformEventClass)
        );
        assert_eq!(
            PlatformEventClass::try_from(0xFF),
            Err(PldmError::InvalidPlatformEventClass)
        );
    }

    #[test]
    fn test_transport_protocol_type_admits_mctp_only() {
        assert_eq!(
            TransportProtocolType::try_from(0x00).unwrap(),
            TransportProtocolType::Mctp
        );
        // 0x01 is RBT, which OpenPRoT has no binding for.
        assert_eq!(
            TransportProtocolType::try_from(0x01),
            Err(PldmError::InvalidTransportProtocolType)
        );
    }

    #[test]
    fn test_event_message_buffer_floor_is_256() {
        // Clause 16.9 sets both the default and the smallest legal value,
        // which is why single-part event responses always fit.
        assert_eq!(EVENT_MESSAGE_BUFFER_SIZE_DEFAULT, 256);
    }

    #[test]
    fn test_event_id_sentinels_differ() {
        assert_eq!(EVENT_ID_NONE, 0x0000);
        assert_eq!(EVENT_ID_IN_PROGRESS, 0xFFFF);
    }

    #[test]
    fn test_log_operational_status_round_trip() {
        assert_eq!(
            LogOperationalStatus::try_from(3).unwrap(),
            LogOperationalStatus::EnabledFull
        );
        assert_eq!(LogOperationalStatus::Corrupted as u8, 6);
        assert_eq!(
            LogOperationalStatus::try_from(7),
            Err(PldmError::InvalidLogOperationalStatus)
        );
    }

    #[test]
    fn test_log_clearing_policy_support_bits() {
        assert_eq!(log_clearing_policy_support::FILL_AND_STOP, 0b0000_0001);
        assert_eq!(log_clearing_policy_support::FIFO, 0b0000_0010);
        assert_eq!(log_clearing_policy_support::CLEAR_ON_AGE, 0b0000_0100);
    }

    #[test]
    fn test_log_clearing_policy_round_trip() {
        assert_eq!(
            LogClearingPolicy::try_from(2).unwrap(),
            LogClearingPolicy::ClearOnAge
        );
        assert_eq!(
            LogClearingPolicy::try_from(3),
            Err(PldmError::InvalidLogClearingPolicy)
        );
    }

    #[test]
    fn test_enable_logging_round_trip() {
        assert_eq!(
            EnableLogging::try_from(1).unwrap(),
            EnableLogging::EnableLogging
        );
        assert_eq!(
            EnableLogging::try_from(2),
            Err(PldmError::InvalidEnableLogging)
        );
    }

    #[test]
    fn test_log_update_event_round_trip() {
        assert_eq!(
            LogUpdateEvent::try_from(0).unwrap(),
            LogUpdateEvent::NoEvent
        );
        assert_eq!(
            LogUpdateEvent::try_from(2),
            Err(PldmError::InvalidLogUpdateEvent)
        );
    }

    #[test]
    fn test_split_entry_crc_rides_last_fragment_only() {
        // A Full entry ends the entry too, but Table 62 attaches the CRC to
        // lastFragment alone.
        assert!(SplitEntry::LastFragment.carries_transfer_crc());
        assert!(!SplitEntry::Full.carries_transfer_crc());
        assert!(!SplitEntry::FirstFragment.carries_transfer_crc());
        assert_eq!(SplitEntry::try_from(4), Err(PldmError::InvalidSplitEntry));
    }

    #[test]
    fn test_entry_search_type_timestamp_presence() {
        // The compareTimestamp fields are present for the two time-ordered
        // searches and absent for the two offset searches, so the request
        // length depends on this value.
        assert!(EntrySearchType::NewerThan.carries_compare_timestamp());
        assert!(EntrySearchType::OlderThan.carries_compare_timestamp());
        assert!(!EntrySearchType::OffsetFromStart.carries_compare_timestamp());
        assert!(!EntrySearchType::OffsetFromEnd.carries_compare_timestamp());
        assert_eq!(
            EntrySearchType::try_from(4),
            Err(PldmError::InvalidEntrySearchType)
        );
    }

    #[test]
    fn test_entry_id_not_found_sentinel() {
        assert_eq!(ENTRY_ID_NOT_FOUND, 0xFFFF_FFFF);
    }

    #[test]
    fn test_completion_codes_are_command_specific() {
        // The same byte means different things per command, which is why
        // Type 2 needs one enumeration per command rather than the single
        // shared space DSP0267 uses.
        assert_eq!(GetPdrCompletionCode::InvalidRecordHandle as u8, 0x82);
        assert_eq!(
            PollForPlatformEventMessageCompletionCode::EventIdNotValid as u8,
            0x82
        );
        assert_eq!(
            SetSensorEnableCompletionCode::EventGenerationNotSupported as u8,
            0x82
        );
        assert_eq!(
            SetEventReceiverCompletionCode::HeartbeatFrequencyTooHigh as u8,
            0x82
        );
    }

    #[test]
    fn test_get_pdr_completion_code_round_trip() {
        for (code, expected) in [
            (0x80u8, GetPdrCompletionCode::InvalidDataTransferHandle),
            (0x81, GetPdrCompletionCode::InvalidTransferOperationFlag),
            (0x82, GetPdrCompletionCode::InvalidRecordHandle),
            (0x83, GetPdrCompletionCode::InvalidRecordChangeNumber),
            (0x84, GetPdrCompletionCode::TransferTimeout),
            (0x85, GetPdrCompletionCode::RepositoryUpdateInProgress),
        ] {
            assert_eq!(GetPdrCompletionCode::try_from(code).unwrap(), expected);
            assert_eq!(expected as u8, code);
        }
        assert_eq!(
            GetPdrCompletionCode::try_from(0x86),
            Err(PldmError::InvalidCompletionCode)
        );
    }

    #[test]
    fn test_command_completion_codes_round_trip() {
        assert_eq!(
            SetEventReceiverCompletionCode::try_from(0x81).unwrap(),
            SetEventReceiverCompletionCode::EnableMethodNotSupported
        );
        assert_eq!(
            EventMessageSupportedCompletionCode::try_from(0x81).unwrap(),
            EventMessageSupportedCompletionCode::UnsupportedEventFormatVersion
        );
        assert_eq!(
            EventMessageBufferSizeCompletionCode::try_from(0x80).unwrap(),
            EventMessageBufferSizeCompletionCode::IllegalMessageBufferSize
        );
        assert_eq!(
            GetSensorReadingCompletionCode::try_from(0x81).unwrap(),
            GetSensorReadingCompletionCode::RearmUnavailableInPresentState
        );
        assert_eq!(
            ReadPldmEventLogCompletionCode::try_from(0x82).unwrap(),
            ReadPldmEventLogCompletionCode::InvalidEntryId
        );
        assert_eq!(
            FindPldmEventLogEntryCompletionCode::try_from(0x80).unwrap(),
            FindPldmEventLogEntryCompletionCode::InvalidSearchType
        );
    }

    #[test]
    fn test_pdr_header_version_is_one() {
        assert_eq!(PDR_HEADER_VERSION, 0x01);
    }
}
