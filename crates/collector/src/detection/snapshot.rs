use humanymous_signals::report::UaClientHints;

/// Presence/truthiness and native rendering of the webdriver getter.
#[derive(Default, Clone, Copy)]
pub enum Getter {
    /// No truthy getter exists; do not manufacture an integrity signal.
    #[default]
    Absent,
    /// Current Function.prototype.toString contains the frozen native marker.
    Native,
    /// Truthy non-native getter/function value.
    Patched,
}

/// Bounded browser observations. This type is not an authenticated browser proof.
#[derive(Default)]
pub struct Snapshot {
    /// Strict boolean navigator.webdriver, false for absent/nonboolean as before.
    pub webdriver: bool,
    /// Actual navigator.userAgent string.
    pub user_agent: String,
    /// Synchronous low-entropy client hints only.
    pub hints: UaClientHints,
    /// Plugin count when the plugins object is truthy.
    pub plugins: Option<i64>,
    /// Language count when the languages object is truthy.
    pub languages: Option<i64>,
    /// JavaScript truthiness of window.chrome, not property existence.
    pub chrome_truthy: bool,
    /// Notification.permission is a string equal to denied.
    pub notification_denied: bool,
    /// Required numeric outerWidth/innerWidth observations.
    pub widths: Option<(i64, i64)>,
    /// Existing externally produced CDP flags, without claiming they ran.
    pub cdp_flags: [bool; 2],
    /// Enumerable own document key names, never property values.
    pub document_keys: Vec<String>,
    /// Enumerable own window key names, never property values.
    pub window_keys: Vec<String>,
    /// Truthiness of the four frozen Playwright marker properties.
    pub playwright: [bool; 4],
    /// Truthiness of _phantom/callPhantom.
    pub phantom: [bool; 2],
    /// Actual WebGL debug vendor when exposed.
    pub webgl_vendor: String,
    /// Actual WebGL debug renderer when exposed.
    pub webgl_renderer: String,
    /// Numeric navigator.deviceMemory when present.
    pub memory: Option<f64>,
    /// Actual screen height/available height.
    pub screen: Option<(i64, i64)>,
    /// Native rendering of the webdriver getter.
    pub getter: Getter,
    /// Separately read top and fresh iframe webdriver values, when available.
    pub iframe: Option<(bool, bool)>,
}
