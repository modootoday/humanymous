//! Real TCP/shared Session controls using the SAME data as the actual Lab pilot.
//! These native controls do not execute BrowserSession or an ordinary browser.
mod harness;
use super::*;
use harness::{Harness, check};

#[tokio::test]
async fn shared_lab_bootstrap_consumed_ack_and_required_collector_refusal() {
    let fixture = Harness::start().await;
    let result = async {
        let mut client = fixture.bootstrap().await?;
        let mut effect = fixture.request("effect", b"effect".to_vec());
        signed(
            &mut client,
            &mut effect,
            crate::authority::now_ms().unwrap(),
        );
        let response = fixture.call(&effect, false).await?;
        check(response.status == 403, "before-report status")?;
        check(
            serde_json::from_slice::<Value>(&response.body).map_err(|_| "verdict JSON")?["verdict"]
                == "CHALLENGE",
            "before-report verdict",
        )?;
        fixture.complete(&mut client, &response)?;
        let mut collect = fixture.request("collect", client_report(""));
        signed(
            &mut client,
            &mut collect,
            crate::authority::now_ms().unwrap(),
        );
        let response = fixture.call(&collect, false).await?;
        check(response.status == 200, "collect status")?;
        fixture.complete(&mut client, &response)?;
        let mut required = fixture.request("required_collector", b"effect".to_vec());
        let counter = signed(
            &mut client,
            &mut required,
            crate::authority::now_ms().unwrap(),
        );
        let response = fixture.call(&required, false).await?;
        check(response.status == 422, "required-collector status")?;
        fixture.refuse_without_ack(&mut client, counter, &response)?;
        check(
            fixture.audit.records.lock().unwrap().len() == 2,
            "preconsume refusal added audit",
        )
    }
    .await;
    let cleanup = fixture.finish().await;
    cleanup.unwrap();
    result.unwrap();
}

#[tokio::test]
async fn shared_lab_request_limit_413_has_no_ack_and_closes_client_epoch() {
    let fixture = Harness::start().await;
    let result = async {
        let mut client = fixture.bootstrap().await?;
        let mut exact = fixture.request("effect", vec![b'x'; crate::gateway::http::MAX_BODY]);
        signed(&mut client, &mut exact, crate::authority::now_ms().unwrap());
        let response = fixture.call(&exact, false).await?;
        check(
            response.status == 403,
            "65536-byte request must reach frozen policy",
        )?;
        fixture.complete(&mut client, &response)?;
        check(!client.is_closed(), "consumed policy response closed epoch")?;
        let mut oversized =
            fixture.request("effect", vec![b'x'; crate::gateway::http::MAX_BODY + 1]);
        let counter = signed(
            &mut client,
            &mut oversized,
            crate::authority::now_ms().unwrap(),
        );
        // Send the signed original Content-Length but no body: the parser must
        // return413 from its header boundary without waiting for entity bytes.
        let response = fixture.call(&oversized, true).await?;
        check(
            response.status == 413,
            "65537-byte declaration must return413",
        )?;
        fixture.refuse_without_ack(&mut client, counter, &response)?;
        check(
            fixture.audit.records.lock().unwrap().len() == 1,
            "oversize request reached decision audit",
        )
    }
    .await;
    let cleanup = fixture.finish().await;
    cleanup.unwrap();
    result.unwrap();
}
