# G6 origin-audit-binding Cap topology image.
# Cap=0 Cap-ready candidate. Cap=1 after Cap realloc + live TRANSFER only.
# G5/CR Cap green != origin_audit_product_binding product. Detection freeze. CARGO_BUILD_JOBS=1.
# Cap pilot: emit honesty measure receipt scaffold; refuse product invent.
# network_mode=none; never claims origin/audit product / Cap free invent.
FROM alpine:3.20
COPY deployments/lab/origin/g6-origin-audit-binding-contract.json /opt/lab/g6-origin-audit-binding-contract.json
COPY deployments/lab/origin/g6-origin-audit-binding-measure-receipt.scaffold.json /opt/lab/measure-receipt.json
COPY deployments/lab/origin/g6-origin-audit-binding-cap-pilot.sh /opt/lab/cap-pilot.sh
RUN chmod 0555 /opt/lab/cap-pilot.sh
USER 65532:65532
ENTRYPOINT ["/bin/sh", "/opt/lab/cap-pilot.sh"]
