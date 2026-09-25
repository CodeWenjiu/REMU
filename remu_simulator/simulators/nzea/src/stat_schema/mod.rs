//! Stat schema: the RTL declares what it measures, remu reads the declaration,
//! evaluates it and renders it. No metric, region or format string is known to
//! remu any more (see the nzea handoff document, schema = 1).
//!
//! The schema file is `<verilog_dir>/<Design>.stats.toml`, emitted by nzea's
//! dump next to `filelist.f`: the `schema` module parses and validates it, the
//! `expr` module is the expression language behind its derived entries. The
//! grammar and its evaluation semantics follow `nzea_rtl/src/StatExpr.scala`
//! (the normative reference, pinned by the tests); the parser itself is built
//! from winnow combinators, as elsewhere in the repo. The one extension is that
//! unavailability carries a *reason* for display (`n/a (…)`), which the Scala
//! reference does not need.

remu_macro::mod_prv!(expr, schema);

// Crate-facing API of the schema. `Counter`/`Derived`/`Region` stay unnamed
// outside: consumers read the fields of the values they are handed.
pub(crate) use expr::Unavailable;
pub(crate) use schema::StatSchema;
