//! Trusted private-pipe broker for separately mounted, distinct-UID witnesses.
//! Audit owns framing, signatures, proofs, persistent head and client semantics.
use super::Lab;
use crate::{docker::Result, snapshot};
use humanymous_runtime::audit::checkpoint::service::{read_frame, write_frame};
use serde_json::{Value, json};
use std::{
    io::{BufReader, Write},
    sync::mpsc,
    time::Duration,
};

pub(super) fn frame_reader(
    output: std::process::ChildStdout,
) -> mpsc::Receiver<std::io::Result<Vec<u8>>> {
    let (send, receive) = mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let mut reader = BufReader::new(output);
        loop {
            match read_frame(&mut reader) {
                Ok(Some(frame)) => {
                    if send.send(Ok(frame)).is_err() {
                        break;
                    }
                }
                Ok(None) => break,
                Err(error) => {
                    let _ = send.send(Err(error));
                    break;
                }
            }
        }
    });
    receive
}

pub(super) struct Broker {
    pub(super) passphrase: Vec<u8>,
    pub(super) public: Option<Value>,
    pub(super) writer_public: Option<Value>,
    pub(super) head: Option<Value>,
    pub(super) transactions: usize,
    pub(super) inspections: Vec<Value>,
}
impl Broker {
    pub(super) fn transaction(
        &mut self,
        lab: &mut Lab,
        writer: &Value,
        create: bool,
        request: &Value,
    ) -> Result<Vec<Value>> {
        let bootstrap = match &self.head {
            Some(head) => json!({"mode":"pinned","minimum":head,"consistency_proof":[]}),
            None => json!({"mode":"create_new"}),
        };
        let private = json!({"create_identity":create,"config":{"key_directory":"/witness/keys","state_directory":"/witness/head","owner_sid":null,"passphrase":self.passphrase,"writer_public":writer,"node_id":"isolated-node","bootstrap":bootstrap}});
        let mut input = Vec::new();
        write_frame(&mut input, &private)?;
        write_frame(&mut input, request)?;
        write_frame(&mut input, &json!({"operation":"shutdown"}))?;
        lab.compose(&["create", "--force-recreate", "witness-transaction"], true)?;
        let actor = lab.inspect("witness-transaction")?;
        lab.case(
            "witness-network-none",
            actor["HostConfig"]["NetworkMode"] == "none",
            json!("private inherited pipes; no public listener"),
        )?;
        self.inspections.push(actor.clone());
        let id = actor["Id"].as_str().ok_or("witness actor ID")?;
        let outcome = (|| -> Result<Vec<Value>> {
            let (mut guard, mut pipe, output) =
                lab.docker
                    .duplex(&["start".into(), "-a".into(), "-i".into(), id.into()])?;
            let receive = frame_reader(output);
            pipe.write_all(&input)?;
            pipe.flush()?;
            let mut frames = Vec::new();
            for _ in 0..4 {
                let bytes = receive.recv_timeout(Duration::from_secs(15))??;
                frames.push(serde_json::from_slice(&bytes)?);
            }
            // Keep stdin open until the actual typed shutdown acknowledgement;
            // Docker start may detach early if its input closes before attach.
            drop(pipe);
            let status = guard.wait(Duration::from_secs(15))?;
            if !status.success() {
                return Err("actual witness transaction process failed".into());
            }
            lab.docker.transcript.push(json!({"actor":id,"reply_frames":frames,"exit":status.code(),"private_input_recorded":false}));
            Ok(frames)
        })();
        if outcome.is_err() {
            let _ = lab.docker.invoke(
                &["logs".into(), "--tail".into(), "32".into(), id.into()],
                &[],
                false,
            );
        }
        lab.docker.call(&["rm", "--force", id])?;
        let frames = outcome?;
        if frames.len() != 4
            || frames[0]["role"] != "witness_identity"
            || frames[1]["status"] != "ready"
            || frames[3]["status"] != "stopped"
        {
            return Err("witness framing/lifecycle mismatch".into());
        }
        let public = frames[0]["witness_public"].clone();
        if frames[1]["witness_public"] != public
            || self
                .public
                .as_ref()
                .is_some_and(|expected| expected != &public)
        {
            return Err("witness identity changed".into());
        }
        self.public = Some(public);
        if frames[2]["status"] == "signed" {
            self.head = Some(frames[2]["checkpoint"].clone());
        }
        self.transactions += 1;
        Ok(frames)
    }
    fn writer(&mut self, lab: &mut Lab, create: bool, outage: bool) -> Result<Value> {
        lab.compose(&["create", "--force-recreate", "witness-writer"], true)?;
        let actor = lab.inspect("witness-writer")?;
        let id = actor["Id"].as_str().ok_or("writer actor ID")?.to_owned();
        lab.case(
            "writer-network-none",
            actor["HostConfig"]["NetworkMode"] == "none",
            json!("writer sees only its custody/WAL mounts"),
        )?;
        self.inspections.push(actor);
        let (mut guard, input, output) =
            lab.docker
                .duplex(&["start".into(), "-a".into(), "-i".into(), id.clone()])?;
        let mut input = Some(input);
        // Stable synthetic writer password is independent of the random witness
        // password. It never grants witness key-volume access.
        write_frame(
            input.as_mut().ok_or("writer pipe")?,
            &json!({"create_identity":create,"passphrase":vec![31;32],"trusted_head":self.head}),
        )?;
        let receive = frame_reader(output);
        let first = receive.recv_timeout(Duration::from_secs(15))??;
        let identity: Value = serde_json::from_slice(&first)?;
        if identity["role"] != "writer_identity" {
            return Err("writer identity frame missing".into());
        }
        let public = identity["writer_public"].clone();
        if self
            .writer_public
            .as_ref()
            .is_some_and(|expected| expected != &public)
        {
            return Err("writer identity changed across recreation".into());
        }
        self.writer_public = Some(public.clone());
        let initial = self.transaction(lab, &public, create, &json!({"operation":"state"}))?;
        write_frame(
            input.as_mut().ok_or("writer private pipe")?,
            &json!({"witness_public":self.public}),
        )?;
        write_frame(input.as_mut().ok_or("writer private pipe")?, &initial[1])?;
        let mut report = None;
        for _ in 0..8 {
            let bytes = receive.recv_timeout(Duration::from_secs(30))??;
            let value: Value = serde_json::from_slice(&bytes)?;
            if value["role"] == "writer_result" {
                report = Some(value);
                break;
            }
            if value["operation"] != "counter_sign" {
                return Err("unexpected writer broker operation".into());
            }
            if outage {
                // The real witness transaction has acknowledged shutdown and
                // exited. Inject loss of its trusted provisioning/control path;
                // the broker reports unavailable, never invents a signature.
                let active = lab.compose(
                    &["ps", "--status", "running", "-q", "witness-transaction"],
                    true,
                )?;
                lab.case(
                    "witness-absent-after-readiness",
                    active.stdout.trim().is_empty(),
                    json!("actual witness process/container stopped; broker refuses restart"),
                )?;
                write_frame(
                    input.as_mut().ok_or("writer private pipe")?,
                    &json!({"status":"refused","reason":"unavailable"}),
                )?;
                continue;
            }
            let signed = self.transaction(lab, &public, false, &value)?;
            write_frame(input.as_mut().ok_or("writer private pipe")?, &signed[2])?;
        }
        drop(input.take());
        let status = guard.wait(Duration::from_secs(15))?;
        lab.docker.call(&["rm", "--force", &id])?;
        if !status.success() {
            return Err("writer process failed".into());
        }
        let report = report.ok_or("writer result missing")?;
        if outage {
            lab.case(
                "witness-lost-before-effect",
                report["status"] == "closed" && report["origin_effects"] == 0,
                json!("required witness absent after readiness; injected broker unavailability; record may be durable, no retry"),
            )?;
        } else {
            lab.case(
                "witness-before-origin-effect",
                report["status"] == "passed"
                    && report["origin_effects"] == 1
                    && self.head.as_ref() == Some(&report["checkpoint"]),
                json!("shared Log waits for separately persisted signed receipt"),
            )?;
        }
        Ok(report)
    }
}
pub(super) fn run(lab: &mut Lab) -> Result<()> {
    eprintln!("lab: isolated witness UID/key/head-volume and emit-before-effect pilot");
    let mut passphrase = vec![0; 32];
    getrandom::fill(&mut passphrase)?;
    let mut broker = Broker {
        passphrase,
        public: None,
        writer_public: None,
        head: None,
        transactions: 0,
        inspections: Vec::new(),
    };
    let first = broker.writer(lab, true, false)?;
    let first_head = broker
        .head
        .clone()
        .ok_or("first independent head missing")?;
    let second = broker.writer(lab, false, false)?;
    lab.case("witness-container-recreation",first["audit_sequence"]==1&&second["audit_sequence"]==2,json!("actual fresh writer/service containers reuse only their separate named volumes and external pin"))?;
    let writer_public = broker
        .inspections
        .iter()
        .find(|v| v["Config"]["User"] == "65532:65532")
        .ok_or("writer inspection absent")?;
    let witness_actor = broker
        .inspections
        .iter()
        .find(|v| v["Config"]["User"] == "65533:65533")
        .ok_or("witness inspection absent")?;
    let mount_names = |v: &Value| {
        v["Mounts"]
            .as_array()
            .map(|ms| {
                ms.iter()
                    .filter_map(|m| m["Name"].as_str().map(str::to_owned))
                    .collect::<std::collections::BTreeSet<_>>()
            })
            .unwrap_or_default()
    };
    lab.case(
        "independent-witness-key-state-mounts",
        mount_names(writer_public).is_disjoint(&mount_names(witness_actor)),
        json!("different UIDs; writer has neither witness key nor witness head volume"),
    )?;
    lab.compose(&["create", "witness-key-denied"], true)?;
    let denied = lab.inspect("witness-key-denied")?;
    lab.case(
        "witness-key-negative-readonly-mount",
        denied["Mounts"][0]["RW"] == false,
        json!("wrong-UID access is tested against actual populated key volume"),
    )?;
    let negative = lab.docker.invoke(
        &[
            "start".into(),
            "-a".into(),
            denied["Id"].as_str().ok_or("negative actor ID")?.into(),
        ],
        &[],
        false,
    )?;
    lab.case(
        "different-uid-witness-key-denied",
        negative.success,
        json!("actual kernel EACCES, not merely missing path"),
    )?;
    // Replay a real, previously co-signed older head against the latest pin.
    let public = broker
        .writer_public
        .clone()
        .ok_or("writer public identity absent")?;
    let replay = broker.transaction(
        lab,
        &public,
        false,
        &json!({"operation":"counter_sign","checkpoint":first_head,"consistency_proof":[]}),
    )?;
    lab.case(
        "witness-old-signed-head-refused",
        replay[2]["status"] == "refused",
        json!("old authentic checkpoint cannot roll back latest independent head"),
    )?;
    let failed = broker.writer(lab, false, true)?;
    let report = json!({"schema":"humanymous.independent-witness-lab/1","status":"passed","results":[first,second,failed],"transactions":broker.transactions,"pinned_checkpoint":broker.head,"witness_public":broker.public,"actual_separate_uids":true,"actual_separate_key_head_volumes":true,"runtime_binary_integrated":false,"power_loss_qualified":false,"external_pin_supervisor_persistent":false});
    snapshot::write(&lab.evidence, "witness-report.json", &report)?;
    snapshot::write(
        &lab.evidence,
        "witness-containers.json",
        &json!(broker.inspections),
    )?;
    lab.components["independent_witness"] = report;
    Ok(())
}
