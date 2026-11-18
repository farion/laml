//! Non-message constructs only. Fragment creation must not invent example calls.
use super::catalog;
use crate::notation::Notation;
pub fn recipes() -> Vec<catalog::Recipe> {
    catalog::recipes(Notation::Sequence)
        .into_iter()
        .filter_map(|mut r| {
            r.body = match r.title {
                "Alternative" => "alt {condition:q} {\n}\nelse {otherwise:q} {\n}",
                "Loop" => "loop(min={minimum}; max={maximum}; guard={condition:q}) {\n}",
                "Optional fragment" => "opt {condition:q} {\n}",
                "Parallel fragment" => "par {condition:q} {\n}\nand {otherwise:q} {\n}",
                "Critical fragment" => "critical {condition:q} {\n}",
                "Break fragment" => "break {condition:q} {\n}",
                "Negative fragment" => "neg {condition:q} {\n}",
                "Assertion fragment" => "assert {condition:q} {\n}",
                "Strict fragment" => "strict {condition:q} {\n}",
                "Weak sequencing fragment" => "seq {condition:q} {\n}",
                "Ignore fragment" => "ignore {messages:q} {\n}",
                "Consider fragment" => "consider {messages:q} {\n}",
                "Nested fragment shorthand" => "strict alt {condition:q} {\n}",
                "Named fragment" => "{id}: opt {condition:q} {\n}",
                // A decomposition needs inner participants but does not need messages.
                "Inline decomposition" => {
                    "decompose {participant} {label:q} {\n    {child}\n    {other}\n}"
                }
                "Interaction reference with gates" => "ref {label:q} {\n}",
                _ if r.body.contains("->") || r.body.contains("<--") => return None,
                _ => r.body,
            };
            Some(r)
        })
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn insertion_catalog_does_not_offer_messages_or_fill_fragments_with_calls() {
        for r in recipes().into_iter() {
            assert!(!r.body.contains("->"), "{}", r.title);
            assert!(!r.body.contains("<--"), "{}", r.title);
        }
    }
}
