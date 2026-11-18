use super::*;

fn signature(result: Result<Vec<Route>, RouteError>) -> Result<Vec<u8>, (usize, String)> {
    result
        .map(|routes| serde_json::to_vec(&routes).unwrap())
        .map_err(|e| (e.line, e.message))
}

#[test]
fn cached_prefixes_preserve_routes_labels_bridges_and_search_modes() {
    let diagram = crate::parser::parse(
        "semantics {\n A\n B\n C\n @ab A -> B : request\n @ac A -> C\n @bc B -> C\n}\n",
    )
    .unwrap();
    let sheet = StyleSheet::default();
    let boxes = crate::geometry::place(&diagram, &sheet).unwrap();
    let mut cached = PassCache::default();
    let mut uncached = PassCache {
        disabled: true,
        ..Default::default()
    };
    for rescue in [false, true] {
        for order in [vec![0, 1, 2], vec![0, 2, 1], vec![2, 1, 0]] {
            for cutoff in [None, Some(usize::MAX), Some(0), Some(1), Some(usize::MAX)] {
                assert_eq!(
                    signature(cached.run(&diagram, &boxes, &sheet, cutoff, &order, rescue)),
                    signature(uncached.run(&diagram, &boxes, &sheet, cutoff, &order, rescue)),
                    "order={order:?}, cutoff={cutoff:?}, rescue={rescue}"
                );
            }
        }
    }
    assert!(cached.hits > 0);
}

#[test]
fn failed_crowded_passes_reuse_prefixes_without_changing_diagnostics() {
    let source = include_str!("../../tests/fixtures/gne-routing-prefix-performance.laml");
    let diagram = crate::parser::parse(source).unwrap();
    let sheet = crate::styles::load(&diagram, std::path::Path::new("fixture.laml")).unwrap();
    let boxes = crate::geometry::place(&diagram, &sheet).unwrap();
    let mut cached = PassCache::default();
    let mut uncached = PassCache {
        disabled: true,
        ..Default::default()
    };
    for cutoff in [None, Some(usize::MAX), Some(0), Some(2)] {
        let actual = signature(cached.pass(&diagram, &boxes, &sheet, cutoff));
        let expected = signature(uncached.pass(&diagram, &boxes, &sheet, cutoff));
        assert!(actual.is_err(), "original spacing must reproduce the retry");
        assert_eq!(actual, expected);
    }
    assert!(cached.hits > 0);
}

#[test]
fn crowded_diagram_still_expands_spacing_and_is_history_independent() {
    let source = include_str!("../../tests/fixtures/gne-routing-prefix-performance.laml");
    let diagram = crate::parser::parse(source).unwrap();
    let sheet = crate::styles::load(&diagram, std::path::Path::new("fixture.laml")).unwrap();
    let (boxes, first) = super::super::place_and_route(&diagram, &sheet).unwrap();
    assert_eq!(first.len(), 8);
    let again = super::super::route(&diagram, &boxes, &sheet).unwrap();
    assert_eq!(signature(Ok(first)), signature(Ok(again)));
}
