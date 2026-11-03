#[allow(unused_imports)]
mod tests {
    use laml::{parser, syntax::DiagramType};

    #[test]
    fn four_levels_parse_with_correct_scope_and_visible_details() {
        for (level, source) in [
            ("context", include_str!("../examples/c4-context.laml")),
            ("container", include_str!("../examples/c4-container.laml")),
            ("component", include_str!("../examples/c4-component.laml")),
            ("code", include_str!("../examples/c4-code.laml")),
        ] {
            let graph = parser::parse(source).unwrap_or_else(|e| panic!("{level}: {e:?}"));
            assert!(graph.nodes.len() >= 3);
            if level == "code" {
                assert_eq!(graph.settings.diagram_type, DiagramType::Class);
            }
        }
        let graph = parser::parse(include_str!("../examples/c4-container.laml")).unwrap();
        assert!(
            graph
                .nodes
                .iter()
                .any(|n| n.label.as_deref().is_some_and(|s| s.contains("PostgreSQL")))
        );
        assert!(
            graph
                .relations
                .iter()
                .any(|r| r.label.as_deref() == Some("Sends order (HTTPS)"))
        );
    }

    #[test]
    fn rejects_missing_scope_and_wrong_zoom_level() {
        let context = include_str!("../examples/c4-context.laml");
        assert!(parser::parse(&context.replace("    scope: Shop\n", "")).is_err());
        assert!(parser::parse(&context.replace("scope: Shop", "scope: Unknown")).is_err());
        assert!(parser::parse(&context.replace("system Shop", "container Shop")).is_err());
        assert!(
            parser::parse(&context.replace(
                "    Customer -> Shop : Places orders",
                "    Customer -- Shop : Places orders"
            ))
            .is_err()
        );
        assert!(
            parser::parse(&context.replace(
                "    Customer -> Shop : Places orders",
                "    Customer -> Shop"
            ))
            .is_err()
        );
        assert!(
            parser::parse(&context.replace(
                "    Shop -> Payment : Requests payment authorization",
                "    Customer -> Payment : Requests payment authorization"
            ))
            .is_err()
        );
        assert!(
            parser::parse(&context.replace(
                "    Customer -> Shop : Places orders",
                "    UnknownNode\n    Customer -> Shop : Places orders"
            ))
            .is_err()
        );
    }

    #[test]
    fn rejects_missing_technology_and_unscoped_code() {
        let container = include_str!("../examples/c4-container.laml");
        assert!(parser::parse(&container.replace("technology=\"React\"; ", "")).is_err());
        let code = include_str!("../examples/c4-code.laml");
        assert!(
            parser::parse(&code.replace("    component Orders", "    component Elsewhere"))
                .is_err()
        );
    }
}
