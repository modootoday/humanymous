"""Bounded negative reachability, exclusively owned Docker target addresses.

The host supplies addresses obtained from the owned project's Docker inspection.
No general URL/hostname or public-target option exists.
"""
import ipaddress
import json
import os
import socket
import sys


def private_address(value):
    address = ipaddress.ip_address(value)
    ranges = [ipaddress.ip_network(cidr) for cidr in ("10.0.0.0/8", "172.16.0.0/12", "192.168.0.0/16")]
    if address.version != 4 or not any(address in network for network in ranges):
        raise ValueError("target must be an inspected private owned Docker IPv4 address")
    return str(address)


def cannot_connect(host, port):
    try:
        with socket.create_connection((host, port), timeout=1):
            return False
    except OSError:
        return True


def run(origin, outside, gateway):
    cases = []

    def record(name, passed, detail):
        cases.append({"id": name, "status": "passed" if passed else "failed", "detail": detail})

    for name, host, port in [
        ("no-origin-bypass-ip", origin, 8080),
        ("no-outside-bridge", outside, 8080),
        ("no-gateway-docker-api", gateway, 2375),
        ("no-gateway-docker-tls", gateway, 2376),
        ("no-metadata-http", "169.254.169.254", 80),
    ]:
        record(name, cannot_connect(host, port), {"target": host, "port": port})
    for name in ("origin", "outside-canary"):
        try:
            socket.getaddrinfo(name, 8080)
            blocked = False
        except socket.gaierror:
            blocked = True
        record("no-private-dns-" + name, blocked, "paired with direct IP assertion")
    record("no-container-api-mount", not os.path.exists("/var/run/docker.sock"), "socket absent")
    record("nonroot-client", os.getuid() == 65532, os.getuid())
    try:
        with open("/rootfs-write-probe", "x"):
            pass
        readonly = False
    except OSError as error:
        readonly = error.errno == 30
    record("read-only-rootfs", readonly, "EROFS required")
    status = open("/proc/self/status").read().splitlines()
    fields = dict(line.split(":", 1) for line in status if ":" in line)
    record("no-effective-caps", int(fields["CapEff"].strip(), 16) == 0, fields["CapEff"].strip())
    record("no-new-privileges", fields["NoNewPrivs"].strip() == "1", fields["NoNewPrivs"].strip())
    print(json.dumps({"kind": "topology-fixture", "cases": cases, "passed": sum(c["status"] == "passed" for c in cases), "total": len(cases)}))
    return all(c["status"] == "passed" for c in cases)


if __name__ == "__main__":
    if len(sys.argv) != 4:
        raise SystemExit("refused: require three inspected owned-project addresses")
    raise SystemExit(0 if run(*(private_address(x) for x in sys.argv[1:])) else 1)
