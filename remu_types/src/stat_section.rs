//! Display payload for `stat_print`: one section per platform-declared group
//! (e.g. a stats-schema region), rendered as a titled block of `(name, value)`
//! rows.

/// One group of statistics, ready for display. The platform decides how many
/// sections exist, what they are called and in which order they come; the
/// tracer renders them verbatim.
#[derive(Debug, Clone)]
pub struct StatSection {
    /// Section name (e.g. a region name); empty when the platform declares no
    /// groups, in which case the tracer prints the rows without a title.
    pub name: String,
    /// One-line help for the section (e.g. a region doc); may be empty.
    pub doc: String,
    /// `(name, value)` rows, already formatted, in platform order.
    pub rows: Vec<(String, String)>,
}
