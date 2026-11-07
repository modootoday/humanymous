use super::*;

pub(super) async fn collect(
    global: &JsValue,
    navigator: &JsValue,
    report: &mut Advanced,
) -> Result<(), JsValue> {
    if let Some(connection) = optional(navigator, "connection") {
        report.connection_present = true;
        report.connection_rtt = number(&get(&connection, "rtt")?)?;
    }
    if optional(navigator, "getBattery").is_some()
        && let Some(battery) = completion(global, call(navigator, "getBattery", &[])).await?
    {
        report.battery_present = true;
        report.battery_level = number(&get(&battery, "level")?)?;
        report.battery_charging = truthy(&get(&battery, "charging")?);
    }
    if let Some(intl) = optional(global, "Intl")
        && let Ok(formatter) = call(&intl, "DateTimeFormat", &[])
        && let Ok(options) = call(&formatter, "resolvedOptions", &[])
    {
        report.timezone_iana = text(&get(&options, "timeZone")?)?;
        if let Some(date) = optional(global, "Date")
            && let Ok(constructor) = date.dyn_into::<js_sys::Function>()
            && let Ok(date) = js_sys::Reflect::construct(&constructor, &js_sys::Array::new())
        {
            report.timezone_offset = int(&call(&date, "getTimezoneOffset", &[])?)?;
        }
    }
    if optional(global, "matchMedia").is_some() {
        let matches = |query: &str| -> Result<bool, JsValue> {
            Ok(truthy(&get(
                &call(global, "matchMedia", &[query.into()])?,
                "matches",
            )?))
        };
        report.color_gamut = if matches("(color-gamut: p3)")? {
            "p3"
        } else if matches("(color-gamut: srgb)")? {
            "srgb"
        } else {
            ""
        }
        .into();
        report.dynamic_range = if matches("(dynamic-range: high)")? {
            "high"
        } else {
            "standard"
        }
        .into();
        report.pointer_coarse = matches("(pointer: coarse)")?;
    }
    if let Some(performance) = optional(global, "performance")
        && optional(&performance, "measureUserAgentSpecificMemory").is_some()
    {
        match call(&performance, "measureUserAgentSpecificMemory", &[]) {
            Err(_) => report.measure_mem_throws = true,
            Ok(value) => {
                let promise = value
                    .dyn_into::<Promise>()
                    .map_err(|_| JsValue::from_str("memory Promise required"))?;
                report.measure_mem_throws =
                    matches!(settle(global, &promise, 800).await?, Settlement::Rejected);
            }
        }
    }
    Ok(())
}
