//! Native management-window pilot; never launches a target browser.
#![forbid(unsafe_code)]

fn main() -> Result<(), String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let seconds = match args.as_slice() {
        [] => None,
        [flag, value] if flag == "--pilot-seconds" => {
            let seconds = value.parse::<u64>().map_err(|_| "invalid pilot duration")?;
            if !(1..=600).contains(&seconds) {
                return Err("pilot duration must be 1..600".into());
            }
            Some(seconds)
        }
        _ => return Err("usage: humanymous-browser [--pilot-seconds 1..600]".into()),
    };
    meari_native::infrastructure::window::show_with_options(
        humanymous_browser::ui::document(1100, 720)?,
        seconds,
        meari_native::application::WindowOptions {
            title: "humanymous | synthetic browser UI | CEF NOT READY".into(),
        },
    )
}
