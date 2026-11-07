use super::*;

impl ModelLedger {
    pub(super) fn propose_reservation(
        &mut self,
        request: ModelRequest,
        relations: &[ModelNamespaceRelation],
        cost: ModelCost,
        now: u64,
    ) -> Result<ModelReservationOutcome, ModelError> {
        self.admitted_history(now)?;
        if request.epoch != self.epoch {
            return Err(ModelError::StaleEpoch);
        }
        if [
            request.attempt.id,
            request.attempt.generation,
            request.owner,
            request.anchor.volume,
            request.selector,
            request.policy,
            request.namespace,
        ]
        .contains(&0)
            || request.anchor.file == 0
            || request.nonce == 0
            || cost.bytes == 0
            || cost.pending_operations == 0
            || !(request.original.start_tick < request.original.run_until
                && request.original.run_until < request.original.cleanup_until)
            || now < request.original.start_tick
        {
            return Err(ModelError::Invalid);
        }
        if self.records.iter().any(|record| {
            record.request.attempt.id == request.attempt.id
                && record.request.attempt != request.attempt
        }) {
            return Err(ModelError::Conflict);
        }
        if let Ok(index) = self.record_index(request.attempt) {
            let old = &self.records[index];
            if old.request != request || old.cost != cost || old.phase == ModelPhase::Terminal {
                return Err(ModelError::Conflict);
            }
            if now >= old.request.original.run_until {
                return Err(ModelError::Expired);
            }
            // Pure observation: no clocks, indexes or charges change on replay.
            return Ok(ModelReservationOutcome::ModelExistingSameRecord);
        }
        if now >= request.original.run_until {
            return Err(ModelError::Expired);
        }
        if relations.len() != self.namespaces.len() {
            return Err(ModelError::Conflict);
        }
        for slot in &self.namespaces {
            let matches: Vec<_> = relations
                .iter()
                .filter(|row| row.existing == slot.attempt)
                .collect();
            if matches.len() != 1
                || slot.token == request.namespace
                || matches[0].relation != ModelRelation::Disjoint
            {
                return Err(ModelError::Conflict);
            }
        }
        let attempts = self
            .records
            .len()
            .checked_add(1)
            .ok_or(ModelError::Overflow)?;
        let namespaces = self
            .namespaces
            .len()
            .checked_add(1)
            .ok_or(ModelError::Overflow)?;
        let bytes = self
            .used_bytes
            .checked_add(cost.bytes)
            .ok_or(ModelError::Overflow)?;
        let pending = self
            .pending_operations
            .checked_add(cost.pending_operations)
            .ok_or(ModelError::Overflow)?;
        // Every admitted record reserves its eventual replay tombstone slot.
        if attempts > self.caps.attempts
            || namespaces > self.caps.namespaces
            || attempts > self.caps.tombstones
            || bytes > self.caps.bytes
            || pending > self.caps.pending_operations
        {
            return Err(ModelError::Capacity);
        }
        self.records.push(ModelRecord {
            request,
            phase: ModelPhase::Reserved,
            cost,
        });
        self.namespaces.push(ModelNamespace {
            attempt: request.attempt,
            token: request.namespace,
        });
        self.used_bytes = bytes;
        self.pending_operations = pending;
        self.last_tick = now;
        Ok(ModelReservationOutcome::ModelReserved)
    }

    pub(super) fn mark_create_pending(
        &mut self,
        attempt: ModelAttempt,
        now: u64,
    ) -> Result<(), ModelError> {
        self.admitted_history(now)?;
        let index = self.record_index(attempt)?;
        let record = &self.records[index];
        if record.request.epoch != self.epoch {
            return Err(ModelError::StaleEpoch);
        }
        if record.phase != ModelPhase::Reserved {
            return Err(ModelError::Conflict);
        }
        if now >= record.request.original.run_until {
            return Err(ModelError::Expired);
        }
        self.records[index].phase = ModelPhase::CreatePending;
        self.last_tick = now;
        Ok(())
    }

    pub(super) fn mark_unknown(
        &mut self,
        attempt: ModelAttempt,
        now: u64,
    ) -> Result<(), ModelError> {
        self.check_tick(now)?;
        let index = self.record_index(attempt)?;
        if self.records[index].phase == ModelPhase::Terminal {
            return Err(ModelError::Conflict);
        }
        // Recording uncertainty is allowed after expiry; it never enables work.
        self.records[index].phase = ModelPhase::Unknown;
        self.last_tick = now;
        Ok(())
    }

    pub(super) fn observe_not_created(
        &mut self,
        attempt: ModelAttempt,
        epoch: u64,
        now: u64,
    ) -> Result<(), ModelError> {
        self.admitted_history(now)?;
        let index = self.record_index(attempt)?;
        let record = &self.records[index];
        if epoch != self.epoch || record.request.epoch != epoch {
            return Err(ModelError::StaleEpoch);
        }
        if !matches!(
            record.phase,
            ModelPhase::Reserved | ModelPhase::CreatePending
        ) {
            return Err(ModelError::Conflict);
        }
        self.propose_terminal(
            ModelTerminalObservation {
                attempt,
                owner: record.request.owner,
                anchor: record.request.anchor,
                namespace: record.request.namespace,
                epoch,
            },
            now,
        )
    }

    pub(super) fn propose_terminal(
        &mut self,
        observation: ModelTerminalObservation,
        now: u64,
    ) -> Result<(), ModelError> {
        self.admitted_history(now)?;
        let index = self.record_index(observation.attempt)?;
        let record = &self.records[index];
        if observation.epoch != self.epoch {
            return Err(ModelError::StaleEpoch);
        }
        if observation.owner != record.request.owner
            || observation.anchor != record.request.anchor
            || observation.namespace != record.request.namespace
            || record.phase == ModelPhase::Terminal
        {
            return Err(ModelError::Conflict);
        }
        if now >= record.request.original.cleanup_until {
            return Err(ModelError::Expired);
        }
        let pending = self
            .pending_operations
            .checked_sub(record.cost.pending_operations)
            .ok_or(ModelError::UnknownHistory)?;
        let tombstones = self
            .tombstones
            .len()
            .checked_add(1)
            .ok_or(ModelError::Overflow)?;
        if tombstones > self.caps.tombstones {
            return Err(ModelError::Capacity);
        }
        let request = record.request;
        self.tombstones.push(request);
        self.records[index].phase = ModelPhase::Terminal;
        self.namespaces
            .retain(|slot| slot.attempt != observation.attempt);
        self.pending_operations = pending;
        self.last_tick = now;
        Ok(())
    }

    pub(super) fn propose_handoff(
        &mut self,
        next: u64,
        history: ModelHistoryState,
        now: u64,
    ) -> Result<(), ModelError> {
        self.check_tick(now)?;
        let expected = self.epoch.checked_add(1).ok_or(ModelError::Overflow)?;
        if next != expected {
            return Err(ModelError::StaleEpoch);
        }
        if !self.history_known {
            return Err(ModelError::UnknownHistory);
        }
        if history == ModelHistoryState::KnownFixtureHistory {
            self.check_indexes()?;
        }
        self.epoch = next;
        self.last_tick = now;
        if history == ModelHistoryState::UnknownFixtureHistory {
            self.history_known = false;
            for record in &mut self.records {
                if record.phase != ModelPhase::Terminal {
                    record.phase = ModelPhase::Unknown;
                }
            }
        }
        Ok(())
    }
}
