#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AccountSummaryStatus {
    Ok,
    ConversionUnavailable,
}

impl AccountSummaryStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::ConversionUnavailable => "conversion_unavailable",
        }
    }
}
