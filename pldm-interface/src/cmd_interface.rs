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

use crate::control_context::{ControlContext, CtrlCmdResponder, ProtocolCapability};
use crate::error::MsgHandlerError;
use crate::firmware_device::fd_context::FirmwareDeviceContext;
use crate::firmware_device::fd_ops::FdOps;
use core::sync::atomic::{AtomicBool, Ordering};
use pldm_common::codec::PldmCodec;
use pldm_common::protocol::base::{
    PldmBaseCompletionCode, PldmControlCmd, PldmFailureResponse, PldmMsgHeader, PldmSupportedType,
};
use pldm_common::protocol::firmware_update::FwUpdateCmd;
use pldm_common::util::mctp_transport::{
    construct_mctp_pldm_msg, extract_pldm_msg, PLDM_MSG_OFFSET,
};

pub type PldmCompletionErrorCode = u8;

// Helper function to write a failure response message into payload
pub(crate) fn generate_failure_response(
    payload: &mut [u8],
    completion_code: u8,
) -> Result<usize, MsgHandlerError> {
    let header = PldmMsgHeader::decode(payload).map_err(MsgHandlerError::Codec)?;
    let resp = PldmFailureResponse {
        hdr: header.into_response(),
        completion_code,
    };
    resp.encode(payload).map_err(MsgHandlerError::Codec)
}

pub struct CmdInterface<'a, O: FdOps> {
    pub ctrl_ctx: ControlContext<'a>,
    pub fd_ctx: FirmwareDeviceContext<'a, O>,
    busy: AtomicBool,
}

impl<'a, O: FdOps> CmdInterface<'a, O> {
    pub fn new(
        protocol_capabilities: &'a [ProtocolCapability],
        fd_ctx: FirmwareDeviceContext<'a, O>,
    ) -> Self {
        let ctrl_ctx = ControlContext::new(protocol_capabilities);
        Self {
            ctrl_ctx,
            fd_ctx,
            busy: AtomicBool::new(false),
        }
    }

    pub fn is_update_mode(&self) -> bool {
        self.fd_ctx.is_update_mode()
    }

    pub fn handle_responder_msg(&mut self, msg_buf: &mut [u8]) -> Result<usize, MsgHandlerError> {
        // Process the request
        let resp_len = self.process_request(msg_buf)?;

        Ok(resp_len)
    }

    pub fn handle_initiator_msg(&mut self, msg_buf: &mut [u8]) -> Result<(), MsgHandlerError> {
        // Prepare the request payload
        let payload = construct_mctp_pldm_msg(msg_buf).map_err(MsgHandlerError::Util)?;

        // Generate the request
        let req_len = self.fd_ctx.fd_progress(payload)?;
        if req_len == 0 {
            return Ok(());
        }

        Ok(())
    }

    pub fn handle_initiator_response(&mut self, msg_buf: &mut [u8]) -> Result<(), MsgHandlerError> {
        // Recieve and parse response
        let payload = extract_pldm_msg(msg_buf).map_err(MsgHandlerError::Util)?;

        // Handle the response
        self.fd_ctx.handle_response(payload)?;

        Ok(())
    }

    /// Generate the next FD-initiated PLDM request into `msg_buf`.
    ///
    /// Sets the MCTP message-type byte at `msg_buf[0]` and writes the PLDM
    /// request starting at `msg_buf[1]`.
    ///
    /// Returns `Ok(0)` when no request is pending (nothing to send).
    /// Returns `Ok(n)` where `n` is the total number of bytes written to
    /// `msg_buf` (1 MCTP header byte + `n - 1` PLDM bytes) when a request
    /// was generated.
    pub fn generate_initiator_request(
        &mut self,
        msg_buf: &mut [u8],
    ) -> Result<usize, MsgHandlerError> {
        let payload = construct_mctp_pldm_msg(msg_buf).map_err(MsgHandlerError::Util)?;
        let pldm_len = self.fd_ctx.fd_progress(payload)?;
        Ok(pldm_len)
    }

    /// Process a received FD-initiated PLDM response from `msg_buf`.
    ///
    /// `msg_buf[0]` must be the MCTP message-type byte (0x01).
    /// `msg_buf[1..]` must contain the PLDM response payload.
    pub fn process_initiator_response(
        &mut self,
        msg_buf: &mut [u8],
    ) -> Result<(), MsgHandlerError> {
        let payload = extract_pldm_msg(msg_buf).map_err(MsgHandlerError::Util)?;
        self.fd_ctx.handle_response(payload)
    }

    fn process_request(&mut self, msg_buf: &mut [u8]) -> Result<usize, MsgHandlerError> {
        // Check if the handler is busy processing a request
        if self.busy.load(Ordering::SeqCst) {
            return Err(MsgHandlerError::NotReady);
        }

        self.busy.store(true, Ordering::SeqCst);

        // Get the pldm payload from msg_buf
        let payload = &mut msg_buf[PLDM_MSG_OFFSET..];
        let reserved_len = PLDM_MSG_OFFSET;

        let (pldm_type, cmd_opcode) = match self.preprocess_request(payload) {
            Ok(result) => result,
            Err(e) => {
                self.busy.store(false, Ordering::SeqCst);
                return Ok(reserved_len + generate_failure_response(payload, e)?);
            }
        };

        let resp_len = match pldm_type {
            PldmSupportedType::Base => self.process_control_cmd(cmd_opcode, payload),
            PldmSupportedType::FwUpdate => self.process_fw_update_cmd(cmd_opcode, payload),
            _ => {
                unreachable!()
            }
        };

        self.busy.store(false, Ordering::SeqCst);

        match resp_len {
            Ok(bytes) => Ok(reserved_len + bytes),
            Err(e) => Err(e),
        }
    }

    fn process_control_cmd(
        &self,
        cmd_opcode: u8,
        payload: &mut [u8],
    ) -> Result<usize, MsgHandlerError> {
        match PldmControlCmd::try_from(cmd_opcode) {
            Ok(cmd) => match cmd {
                PldmControlCmd::GetTid => self.ctrl_ctx.get_tid_rsp(payload),
                PldmControlCmd::SetTid => self.ctrl_ctx.set_tid_rsp(payload),
                PldmControlCmd::GetPldmTypes => self.ctrl_ctx.get_pldm_types_rsp(payload),
                PldmControlCmd::GetPldmCommands => self.ctrl_ctx.get_pldm_commands_rsp(payload),
                PldmControlCmd::GetPldmVersion => self.ctrl_ctx.get_pldm_version_rsp(payload),
            },
            Err(_) => {
                generate_failure_response(payload, PldmBaseCompletionCode::UnsupportedPldmCmd as u8)
            }
        }
    }

    fn process_fw_update_cmd(
        &mut self,
        cmd_opcode: u8,
        payload: &mut [u8],
    ) -> Result<usize, MsgHandlerError> {
        match FwUpdateCmd::try_from(cmd_opcode) {
            Ok(cmd) => match cmd {
                FwUpdateCmd::QueryDeviceIdentifiers => self.fd_ctx.query_devid_rsp(payload),
                FwUpdateCmd::GetFirmwareParameters => {
                    self.fd_ctx.get_firmware_parameters_rsp(payload)
                }
                FwUpdateCmd::RequestUpdate => self.fd_ctx.request_update_rsp(payload),
                FwUpdateCmd::PassComponentTable => self.fd_ctx.pass_component_rsp(payload),
                FwUpdateCmd::UpdateComponent => self.fd_ctx.update_component_rsp(payload),

                FwUpdateCmd::ActivateFirmware => self.fd_ctx.activate_firmware_rsp(payload),
                FwUpdateCmd::CancelUpdateComponent => {
                    self.fd_ctx.cancel_update_component_rsp(payload)
                }
                FwUpdateCmd::CancelUpdate => self.fd_ctx.cancel_update_rsp(payload),
                FwUpdateCmd::GetStatus => self.fd_ctx.get_status_rsp(payload),
                _ => generate_failure_response(
                    payload,
                    PldmBaseCompletionCode::UnsupportedPldmCmd as u8,
                ),
            },
            Err(_) => {
                generate_failure_response(payload, PldmBaseCompletionCode::UnsupportedPldmCmd as u8)
            }
        }
    }

    fn preprocess_request(
        &self,
        payload: &[u8],
    ) -> Result<(PldmSupportedType, u8), PldmCompletionErrorCode> {
        let header = PldmMsgHeader::decode(payload)
            .map_err(|_| PldmBaseCompletionCode::InvalidData as u8)?;
        if !(header.is_request() && header.is_hdr_ver_valid()) {
            Err(PldmBaseCompletionCode::InvalidData as u8)?;
        }

        let pldm_type = PldmSupportedType::try_from(header.pldm_type())
            .map_err(|_| PldmBaseCompletionCode::InvalidPldmType as u8)?;

        if !self.ctrl_ctx.is_supported_type(pldm_type) {
            Err(PldmBaseCompletionCode::InvalidPldmType as u8)?;
        }

        let cmd_opcode = header.cmd_code();
        if self.ctrl_ctx.is_supported_command(pldm_type, cmd_opcode) {
            Ok((pldm_type, cmd_opcode))
        } else {
            Err(PldmBaseCompletionCode::UnsupportedPldmCmd as u8)
        }
    }
}
