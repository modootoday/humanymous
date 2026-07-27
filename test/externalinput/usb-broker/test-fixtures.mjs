export const attestation = Object.freeze({
  vid: '1209',
  pid: '0001',
  serialSha256: '1'.repeat(64),
  descriptorSha256: '2'.repeat(64),
  topologySha256: '3'.repeat(64),
  firmwareSha256: '4'.repeat(64),
  dedicatedSeat: true,
  seatEventObserved: true,
  physicalUsb: true,
  uinputPresent: false,
  interfaceSet: 'command+keyboard+pointer',
  exclusiveAssignment: true,
  emergencyStopReady: true,
  deadManReleaseMs: 500,
});

export const safety = Object.freeze({
  emergencyStopReady: true,
  emergencyStopEngaged: false,
  deadManArmed: true,
  deadManReleaseMs: 500,
});
