-- PLAN-08 R6 — durable audit projection table (SoT-32 Tier 2). The gate projects each
-- tamper-evident record here as JSONEachRow. record_hash is the RFC 6962 Merkle leaf,
-- so a row can be cross-checked against the signed tree. Ordered by (node_id, seq) so
-- insert order is irrelevant and retention drops whole partitions.
CREATE TABLE IF NOT EXISTS audit_log
(
    node_id        String,
    seq            UInt64,
    event_id       String,
    ts             String,
    event_type     String,
    verdict        String,
    correlation_id String,
    record_hash    String
)
ENGINE = MergeTree
ORDER BY (node_id, seq);
