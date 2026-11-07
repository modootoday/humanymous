use super::*;
use js_sys::Promise;

pub(super) async fn collect(
    global: &JsValue,
    navigator: &JsValue,
    report: &mut Advanced,
) -> Result<(), JsValue> {
    if let Some(devices) = optional(navigator, "mediaDevices")
        && let Some(values) = completion(global, call(&devices, "enumerateDevices", &[])).await?
    {
        report.media_device_count = count(&values)?;
        for i in 0..report.media_device_count {
            let device = get(&values, &i.to_string())?;
            let kind = text(&get(&device, "kind")?)?;
            report.has_audio_input |= kind == "audioinput";
            report.has_video_input |= kind == "videoinput";
        }
    }
    if let Some(speech) = optional(global, "speechSynthesis")
        && let Ok(voices) = call(&speech, "getVoices", &[])
    {
        report.voice_count = count(&voices)?;
        if report.voice_count == 0 {
            wait_voice(global, &speech).await?;
            if let Ok(voices) = call(&speech, "getVoices", &[]) {
                report.voice_count = count(&voices)?;
            }
        }
    }
    Ok(())
}

struct VoiceListener {
    target: JsValue,
    callback: JsValue,
}
impl VoiceListener {
    fn remove(&self) -> Result<(), JsValue> {
        call(
            &self.target,
            "removeEventListener",
            &["voiceschanged".into(), self.callback.clone()],
        )
        .map(|_| ())
    }
}
impl Drop for VoiceListener {
    fn drop(&mut self) {
        let _ = self.remove();
    }
}
async fn wait_voice(global: &JsValue, target: &JsValue) -> Result<(), JsValue> {
    let mut callback = JsValue::UNDEFINED;
    let promise = Promise::new(&mut |resolve, _| {
        callback = resolve.into();
    });
    let listener = VoiceListener {
        target: target.clone(),
        callback,
    };
    let registered = call(
        target,
        "addEventListener",
        &["voiceschanged".into(), listener.callback.clone()],
    );
    let waited = if registered.is_ok() {
        settle(global, &promise, 300).await.map(|_| ())
    } else {
        registered.map(|_| ())
    };
    let cleanup = listener.remove();
    waited.and(cleanup)
}
