# Firmware Device Context and Operations

This directory implements the Firmware Device (FD) side of the PLDM firmware
update protocol. The implementation is split into three parts:

- `fd_context.rs` implements protocol message handling and drives the firmware
  update state machine.
- `fd_internal.rs` stores protocol state, the active component, request state,
  progress, transfer limits, and timeout data.
- `fd_ops.rs` defines `FdOps`, the platform callback interface used by the
  protocol implementation to inspect or modify the actual device.

## `FirmwareDeviceContext`

`FirmwareDeviceContext<'a, O: FdOps>` combines an `FdInternal` state object
with a borrowed platform implementation of `FdOps`. Construct it with
`FirmwareDeviceContext::new(&ops)`. The context does not own the platform
implementation.

The context has two roles:

1. Its `*_rsp` methods handle requests sent by the Update Agent (UA), validate
   the current state, invoke the required `FdOps` callback, encode a response,
   and update `FdInternal`.
2. In the `Download`, `Verify`, and `Apply` states, `fd_progress` drives
   FD-initiated requests. `handle_response` accepts the matching UA response
   and advances the state machine.

A successful update normally follows this state sequence:

```text
Idle
  -> LearnComponents       RequestUpdate
  -> ReadyXfer             final PassComponentTable
  -> Download              accepted UpdateComponent
  -> Verify                successful TransferComplete exchange
  -> Apply                 successful VerifyComplete exchange
  -> ReadyXfer             successful ApplyComplete exchange
  -> Activate -> Idle      successful ActivateFirmware
```

Additional components can be updated by repeating the `ReadyXfer` through
`Apply` portion before activation. Cancellation, operation failure, or a T1
timeout can take the flow to another state or back to `Idle`.

Every `FdOpsError` is converted to `MsgHandlerError::FdOps` and returned to the
caller, except for `query_download_progress`: `get_status_rsp` ignores that
error and leaves whatever the callback wrote in `ProgressPercent`. If the
callback wrote nothing, that is `ProgressPercent::default()`, which is 101
(`PROGRESS_PERCENT_NOT_SUPPORTED`).

## `FdOps` callback map

Every required method is implemented by the platform integrating this crate.
`get_non_functional_component_info` and `now` have defaults, although a real
device will normally override `now` with a monotonic millisecond clock.

### `get_device_identifiers`

Fills the provided descriptor array and returns the number of valid entries.
The first returned entry is used as the initial descriptor and any remaining
entries are encoded as additional descriptors.

Called by `FirmwareDeviceContext::query_devid_rsp` while handling
`QueryDeviceIdentifiers`. The returned count must fit the provided slice and
must include at least one descriptor.

### `get_firmware_parms`

Populates `FirmwareParameters` with the device's active and pending firmware
information and component parameter entries.

Called by:

- `get_firmware_parameters_rsp`, to answer `GetFirmwareParameters`.
- `pass_component_rsp`, before checking a component with
  `handle_component(PassComponent)`.
- `update_component_rsp`, before checking a component with
  `handle_component(UpdateComponent)`.

### `get_xfer_size`

Chooses the transfer size the FD will use, given the maximum transfer size
offered by the UA. The context stores the result in `FdInternal`, which later
caps each `RequestFirmwareData` chunk to that size.

Called by `request_update_rsp` after the UA's size has passed the PLDM baseline
size check and before the context enters `LearnComponents`.

### `handle_component`

Checks whether the device can process a component and returns the appropriate
`ComponentResponseCode`. The `ComponentOperation` argument distinguishes the
two call sites:

- `pass_component_rsp` passes `PassComponent` while the context is learning
  the UA's component table.
- `update_component_rsp` passes `UpdateComponent` when the UA requests an
  actual update. If the result is `CompCanBeUpdated`, the context enters
  `Download` and enables FD-initiated progress.

The accompanying `FirmwareParameters` are freshly obtained through
`get_firmware_parms` at each call site.

### `query_download_offset_and_length`

Returns the next component-relative byte offset and requested byte count. This
allows the platform to select missing ranges or resume a partial transfer. The
context validates the range against the component image size, permits the PLDM
maximum padding, and caps the request to the negotiated transfer size.

Called by `fd_progress_download` when the context is in `Download`, an FD
request may be sent (state `Ready`, or `Sent` with the T2 retry time elapsed),
and transfer completion has not yet been reported. The result is encoded into
`RequestFirmwareData`. The callback runs again on every T2 retry, so it can
return a different range while a `RequestFirmwareData` is still outstanding.

### `download_fw_data`

Consumes one successful `RequestFirmwareData` response. `offset` is the offset
previously selected for the request, `data` is the returned firmware chunk,
and `component` is the active component. The returned `TransferResult` controls
whether downloading continues or a `TransferComplete` request is prepared.

Called by `process_request_fw_data_rsp`, which is reached from
`handle_response` for a matching `RequestFirmwareData` response in the
`Download` state.

### `is_download_complete`

Reports whether all data required for the active component has been received.

Called by `process_request_fw_data_rsp` only after `download_fw_data` returns
`TransferSuccess`. A `true` result marks the FD request complete so the next
`fd_progress` call sends `TransferComplete`; a `false` result makes another
firmware-data request eligible.

### `query_download_progress`

Writes the current download percentage into `ProgressPercent` for status
reporting.

Called by `get_status_rsp` when the current state is `Download`. Errors from
this callback are ignored and leave the response at its default progress.

### `verify`

Performs or advances verification of the active component and updates the
reported percentage. Returning `VerifySuccess` with progress below 100 means
verification is still in progress; `fd_progress` returns no message and calls
the callback again later. At completion or on failure, the context sends a
`VerifyComplete` request containing the returned result.

Called by `pldm_fd_progress_verify`, selected by `fd_progress` in the `Verify`
state when an FD request may be sent.

### `apply`

Performs or advances application of the active component and updates the
reported percentage. Returning `ApplySuccess` with progress below 100 keeps
the operation in progress. At completion or on failure, the context sends an
`ApplyComplete` request containing the returned result.

Called by `pldm_fd_progress_apply`, selected by `fd_progress` in the `Apply`
state when an FD request may be sent.

### `activate`

Activates the newly applied firmware. It receives the UA's self-contained
activation request and may set the estimated activation time. It returns the
PLDM completion code directly; implementations should return
`PLDM_FWUP_INCOMPLETE_UPDATE` if expected components were not updated.

Called by `activate_firmware_rsp` in `ReadyXfer`. Success or
`ActivationNotRequired` moves the context through `Activate` and then to
`Idle` with `ActivateFw` as the status reason.

### `cancel_update_component`

Stops and cleans up work on the active component.

Called from three paths:

- `cancel_update_component_rsp`, for a component cancellation during
  `Download`, `Verify`, or an incomplete/failed `Apply`.
- `cancel_update_rsp`, for a complete update cancellation in the same active
  operation states.
- `fd_progress`, when an FD request is outstanding (`FdReqState::Sent`) in
  `Download`, `Verify`, or `Apply` and the UA has been quiet for longer than
  the T1 timeout; the context then transitions to `Idle` and rejects a late UA
  response.

### `get_non_functional_component_info`

Returns the indication and bitmap describing components that will not function
after leaving update mode. The default implementation reports that all
components are functioning with an empty bitmap.

Called by `cancel_update_rsp` to populate every `CancelUpdate` response after
the context has determined whether an active component operation should be
cancelled.

### `now`

Returns the current time in milliseconds as `PldmFdTime`. Production
implementations should use a monotonic time source because the context uses
saturating elapsed-time calculations and rejects retry calculations when time
moves backwards. The trait's fixed default value is mainly useful as a stub.

Called throughout `fd_context.rs` to:

- set or refresh the T1 activity timestamp in `set_fd_t1_ts`, including UA
  request/response handling and verify/apply work;
- timestamp outgoing `RequestFirmwareData`, `TransferComplete`,
  `VerifyComplete`, and `ApplyComplete` requests;
- detect T1 response timeouts in `fd_progress`;
- determine T2 retransmission eligibility in `should_send_fd_request`.

## Driving initiator mode

The owner of `FirmwareDeviceContext` should use
`should_start_initiator_mode`/`should_stop_initiator_mode` to coordinate its
transport loop. While initiator mode is active, call `fd_progress` to obtain the
next encoded FD request. A return value of zero during verify or apply means
local work is still progressing and no PLDM message should be sent. Pass a UA
response to `handle_response`; it accepts only the command and instance ID of
the currently sent request.

`fd_progress` also handles T2 retries for sent requests and T1 cancellation
when the UA remains silent. The embedding application is responsible for
scheduling calls often enough for those timers and platform operations to make
progress.