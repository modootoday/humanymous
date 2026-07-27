export class CapabilityUnavailableError extends Error {
  constructor(capability, message) {
    super(message || `required capability is unavailable: ${capability}`);
    this.name = 'CapabilityUnavailableError';
    this.capability = capability;
  }
}

export class ContractViolationError extends Error {
  constructor(message) {
    super(message);
    this.name = 'ContractViolationError';
  }
}
