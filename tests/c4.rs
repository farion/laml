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
        let context = include_str!("../examples/c4-context.laml").replace("\r\n", "\n");
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
    fn banking_examples_render_classic_c4_notation() {
        use laml::{
            geometry::place,
            route::route,
            styles::load,
            svg::{TextMode, c4::render},
        };
        use std::path::Path;
        for view in ["container", "component"] {
            let path = format!("examples/c4-banking-{view}.laml");
            let source = std::fs::read_to_string(&path).unwrap();
            let graph = parser::parse(&source).unwrap();
            let styles = load(&graph, Path::new(&path)).unwrap();
            let boxes = place(&graph, &styles).unwrap();
            let routes = route(&graph, &boxes, &styles).unwrap();
            let svg = render(
                &boxes,
                &routes,
                TextMode::System,
                graph.settings.theme,
                &graph,
            )
            .unwrap();
            assert!(svg.contains("View:"));
            assert!(svg.contains("Key:"));
            assert!(svg.contains("stroke-dasharray"));
            assert!(svg.contains("<ellipse"));
            assert!(svg.contains("[Container:"));
            if view == "component" {
                assert!(svg.contains("Component View: Internet Banking System - Backend"));
                let outer = boxes.iter().find(|n| n.id == "Banking").unwrap();
                let inner = boxes.iter().find(|n| n.id == "Banking.Backend").unwrap();
                assert!(outer.c4_boundary && inner.c4_boundary);
                assert!(inner.x > outer.x && inner.x + inner.width < outer.x + outer.width);
                for name in ["signin", "notify"] {
                    let i = graph
                        .relations
                        .iter()
                        .position(|r| r.name.as_deref() == Some(name))
                        .unwrap();
                    assert_eq!(routes[i].points.len(), 2, "{name} should route directly");
                }
            }
        }
    }

    #[test]
    fn all_four_banking_levels_render_without_truncated_text() {
        use laml::{
            geometry::place,
            route::route,
            styles::load,
            svg::{TextMode, c4::render},
        };
        use std::path::Path;
        for view in ["context", "container", "component", "code"] {
            let path = format!("examples/c4-banking-{view}.laml");
            let graph = parser::parse(&std::fs::read_to_string(&path).unwrap()).unwrap();
            let styles = load(&graph, Path::new(&path)).unwrap();
            let boxes = place(&graph, &styles).unwrap();
            let routes = route(&graph, &boxes, &styles).unwrap();
            let svg = render(
                &boxes,
                &routes,
                TextMode::System,
                graph.settings.theme,
                &graph,
            )
            .unwrap();
            assert!(svg.contains("View: "), "{view}");
            assert!(svg.contains("Internet Banking System") || view == "code");
        }
    }

    #[test]
    fn c4_descriptions_are_not_silently_ellipsized() {
        use laml::{geometry::place, styles::load};
        use std::path::Path;
        let source = format!(
            "{}\nstyling {{\n    node[Banking] {{\n        width: 270\n    }}\n}}\n",
            include_str!("../examples/c4-banking-context.laml").replace(
                "Offers online banking",
                "Offers secure online banking for customers with many accounts"
            )
        );
        let graph = parser::parse(&source).unwrap();
        let styles = load(&graph, Path::new("examples/c4-banking-context.laml")).unwrap();
        let error = place(&graph, &styles).unwrap_err();
        assert!(error.message.contains("truncated"), "{error:?}");
    }

    #[test]
    fn rejects_bad_nested_system_scope() {
        let source = include_str!("../examples/c4-banking-component.laml");
        assert!(parser::parse(&source.replace("scope: Backend", "scope: Missing")).is_err());
        assert!(
            parser::parse(&source.replace("system Banking(", "external system Banking(")).is_err()
        );
        assert!(
            parser::parse(&source.replace(
                "        container Backend(",
                "        external system Backend("
            ))
            .is_err()
        );
    }

    #[test]
    fn rejects_invalid_c4_appearances() {
        let source = include_str!("../examples/c4-banking-container.laml");
        assert!(
            parser::parse(&source.replace("appearance=\"browser\"", "appearance=\"wheel\""))
                .is_err()
        );
        assert!(
            parser::parse(&source.replace(
                "appearance=\"browser\"",
                "appearance=\"browser\"; appearance=\"folder\""
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
