#[derive(Copy, Clone)]
pub enum ReportFreq {
    /// One update per boot cycle
    Boot,
    /// One update per hour, or on each boot
    Hourly,
    /// One update per day, or on each boot
    Daily,
}
