# External-input USB command protocol

This broker is a narrow transport boundary. It never opens an input event device,
XTEST, `uinput`, a browser interface, or a network socket. It accepts one bounded
action per local Unix-socket connection and forwards a typed JSON Lines frame to
the exact character device mounted at `HM_EXTERNAL_USB_COMMAND_DEVICE`.

## Startup

The trusted host supervisor writes `usb-attestation.json`. It contains the
allowlisted vendor/product IDs, hashed serial, descriptor, topology and firmware
identity, the exact `command+keyboard+pointer` interface set, dedicated-seat and
exclusive-assignment results, and emergency-stop/dead-man readiness. Raw serial
numbers are not accepted.

Before opening the controller socket, the broker:

1. validates every attestation field and rejects undeclared fields;
2. sends a fresh `hello` frame containing a random session ID and nonce;
3. requires `helloAck` to echo both and exactly match every pinned device identity;
4. requires the physical emergency stop to be ready, not engaged, and the dead-man
   release to be armed at the attested interval; and
5. sends `releaseAll` as command sequence 1 and requires an explicit release ACK.

The USB firmware must already expose a newline-preserving, full-duplex character
device. The broker deliberately has no generic serial configuration or shell
surface.

## Commands

Protocol version `1.0.0` uses newline-delimited JSON frames capped at 4096 bytes.
One command is in flight at a time. A firmware command contains:

- the startup session ID;
- a broker-monotonic positive integer sequence;
- the controller's UUID as `commandId`;
- an absolute millisecond deadline; and
- one validated action.

The only actions are bounded pointer movement, left click, vertical scroll,
allowlisted key strokes, the fixed synthetic task text, and `releaseAll`. Address
bar shortcuts, operating-system shortcuts, arbitrary text, shell content and raw
serial payloads have no protocol representation.

Every ACK must match the session, integer sequence and command ID. It must also
report emergency-stop and dead-man readiness. A mismatched, late, unsolicited,
duplicate, oversized, negative or safety-degraded ACK fails closed. After the first
firmware failure the command queue remains failed; later controller requests are
not dispatched.

## Shutdown and trust boundary

Normal stop and termination attempt a final sequence-bound `releaseAll`. Firmware
must independently release every key and button after the attested dead-man
interval, including when the container is killed, detached or cannot deliver the
final command.

The nonce proves freshness of this broker/firmware conversation. A firmware hash
self-reported by the device is not a cryptographic measurement. The trusted
hardware-in-loop supervisor remains responsible for measuring the pinned firmware,
USB descriptors and topology from the host, assigning the dedicated seat, proving
the physical keyboard/pointer events, and testing the hardware emergency stop.
Without those external checks, Modes 3 and 4 are unavailable rather than passed.
