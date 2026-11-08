# G5 witness-transport Cap topology image.
# Cap=0 Cap-ready candidate. Cap=1 after Cap realloc + live TRANSFER only.
# Custody/witness Cap green != authenticated_witness_transport product. Detection freeze. CARGO_BUILD_JOBS=1.
# Cap pilot: emit honesty measure receipt scaffold; refuse product invent.
# network_mode=none; never claims authenticated transport / Cap free invent.
FROM alpine:3.20
COPY deployments/lab/witness/g5-witness-transport-contract.json /opt/lab/g5-witness-transport-contract.json
COPY deployments/lab/witness/g5-witness-transport-measure-receipt.scaffold.json /opt/lab/measure-receipt.json
COPY deployments/lab/witness/g5-witness-transport-cap-pilot.sh /opt/lab/cap-pilot.sh
RUN chmod 0555 /opt/lab/cap-pilot.sh
USER 65532:65532
ENTRYPOINT ["/bin/sh", "/opt/lab/cap-pilot.sh"]
