//! Composition root for the Rust-only Pages verifier and pinned upstream builder.
use std::path::PathBuf;

fn run() -> Result<(), String> {
    let mut arguments = std::env::args().skip(1);
    let mode = arguments
        .next()
        .ok_or("expected source, built or validate")?;
    if matches!(mode.as_str(), "snapshot" | "review-delta") {
        if arguments.next().as_deref() != Some("--root") {
            return Err(format!("{mode} requires --root"));
        }
        let root = PathBuf::from(arguments.next().ok_or("missing snapshot root")?);
        if arguments.next().is_some() {
            return Err("unexpected snapshot argument".into());
        }
        if mode == "review-delta" {
            println!(
                "{}",
                serde_json::to_string_pretty(&humanymous_pages::review::delta(&root)?)
                    .map_err(|e| e.to_string())?
            );
            return Ok(());
        }
        println!(
            "{}",
            serde_json::to_string_pretty(&humanymous_pages::snapshot::capture(&root)?)
                .map_err(|e| e.to_string())?
        );
        return Ok(());
    }
    if matches!(
        mode.as_str(),
        "vectors" | "vectors-export" | "vectors-i18n-export"
    ) {
        let flag = if mode == "vectors" {
            "--fixtures"
        } else {
            "--output"
        };
        if arguments.next().as_deref() != Some(flag) {
            return Err(format!("{mode} requires {flag}"));
        }
        let path = PathBuf::from(arguments.next().ok_or("missing vector path")?);
        if arguments.next().is_some() {
            return Err("unexpected vector argument".into());
        }
        if mode == "vectors-export" {
            return humanymous_pages::vectors::export(&path);
        }
        if mode == "vectors-i18n-export" {
            return humanymous_pages::vectors::export_i18n(&path);
        }
        let result = humanymous_pages::vectors::check(&path)?;
        println!(
            "{}",
            serde_json::to_string_pretty(&result).map_err(|e| e.to_string())?
        );
        return if result.errors.is_empty() {
            Ok(())
        } else {
            Err("experimental vectors failed".into())
        };
    }
    let (mut source, mut destination, mut site, mut baseurl) = (None, None, None, None);
    while let Some(flag) = arguments.next() {
        let slot = match flag.as_str() {
            "--source" => &mut source,
            "--destination" => &mut destination,
            "--site" => &mut site,
            "--baseurl" => &mut baseurl,
            _ => return Err(format!("unknown argument {flag}")),
        };
        if slot.is_some() {
            return Err(format!("duplicate argument {flag}"));
        }
        *slot = Some(
            arguments
                .next()
                .ok_or_else(|| format!("missing value for {flag}"))?,
        );
    }
    let source = PathBuf::from(source.unwrap_or_else(|| "docs".into()));
    let baseurl = baseurl.unwrap_or_default();
    let result = match mode.as_str() {
        "source" if destination.is_none() && site.is_none() && baseurl.is_empty() => {
            humanymous_pages::check_source(&source)?
        }
        "built" if destination.is_none() => humanymous_pages::check_output(
            &source,
            &PathBuf::from(site.ok_or("built requires --site")?),
            &baseurl,
        )?,
        "validate" if site.is_none() => humanymous_pages::validate(
            &source,
            &PathBuf::from(destination.ok_or("validate requires --destination")?),
            &baseurl,
        )?,
        _ => return Err("invalid command or incompatible options".into()),
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&result).map_err(|e| e.to_string())?
    );
    if result.errors.is_empty() {
        Ok(())
    } else {
        Err("Pages validation failed".into())
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
