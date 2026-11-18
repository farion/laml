use super::*;
use egui::Color32;

fn roles(source: &str) -> Vec<(String, Role)> {
    let mut context = classify::Context::new(source);
    let mut out = Vec::new();
    for line in source.split_inclusive('\n') {
        let tokens = lexer::lex(line);
        out.extend(
            tokens
                .iter()
                .zip(context.line(&tokens))
                .map(|(t, r)| (t.text.to_owned(), r)),
        );
    }
    out
}
pub(super) fn role(source: &str, text: &str) -> Role {
    roles(source)
        .into_iter()
        .find(|(t, _)| t == text)
        .unwrap_or_else(|| panic!("Missing {text:?} in {source}"))
        .1
}
pub(super) fn inside(section: &str, text: &str) -> String {
    format!("{section} {{\n{text}\n}}\n")
}
pub(super) fn member(text: &str) -> String {
    inside("semantics", &format!("class Example {{\n{text}\n}}"))
}

#[test]
fn settings_have_keys_constants_numbers_and_scope_references() {
    let s = inside(
        "settings",
        "type: sequence\ntheme: modern-dark\nallow-diagonal: false\nscope: Shop.API",
    );
    for name in ["type", "theme", "allow-diagonal", "scope"] {
        assert_eq!(role(&s, name), Role::Property);
    }
    for value in ["sequence", "modern-dark", "false"] {
        assert_eq!(role(&s, value), Role::Value);
    }
    for id in ["Shop", "API"] {
        assert_eq!(role(&s, id), Role::Id);
    }
    assert_eq!(role(&s, "settings"), Role::Section);
}

#[test]
fn explicit_ids_properties_and_unquoted_labels_have_distinct_roles() {
    let s = inside(
        "semantics",
        "@request(technology=\"HTTPS\"; style-classes=protocol,critical) Shop.Web -> Shop.API : class if 42",
    );
    assert_eq!(role(&s, "@request"), Role::ExplicitId);
    assert_eq!(role(&s, "technology"), Role::Property);
    assert_eq!(role(&s, "\"HTTPS\""), Role::Text);
    for class in ["protocol", "critical"] {
        assert_eq!(role(&s, class), Role::StyleClass);
    }
    for label in ["class", "if", "42"] {
        assert_eq!(role(&s, label), Role::Text);
    }
    assert_eq!(role(&s, "API"), Role::Id);
    assert_eq!(role(&s, "->"), Role::Operator);
}

#[test]
fn bare_and_quoted_labels_match_without_highlighting_keywords_inside_them() {
    for label in ["request", "\"request\""] {
        assert_eq!(
            role(&inside("semantics", &format!("A -> B : {label}")), label),
            Role::Text
        );
    }
    let s = inside(
        "semantics",
        "named: A -> B : settings\nwork: B+ : Validate\nat[named.send] : time-observation=\"t0\"",
    );
    assert_eq!(role(&s, "named"), Role::Id);
    assert_eq!(role(&s, "work"), Role::Id);
    assert_eq!(role(&s, "settings"), Role::Text);
    assert_eq!(role(&s, "Validate"), Role::Text);
    assert_eq!(role(&s, "time-observation"), Role::Property);
}

#[test]
fn qualified_member_addresses_are_not_mistaken_for_labels() {
    let s = inside("semantics", "A::out -> B::input : send");
    for id in ["A", "out", "B", "input"] {
        assert_eq!(role(&s, id), Role::Id);
    }
    assert_eq!(role(&s, "send"), Role::Text);
}

#[test]
fn inline_attributes_include_references_values_and_numbers() {
    let s = inside("semantics", "part store(of=Store; label=\"42\"; width=110)");
    assert_eq!(role(&s, "part"), Role::Keyword);
    assert_eq!(role(&s, "store"), Role::Id);
    assert_eq!(role(&s, "of"), Role::Property);
    assert_eq!(role(&s, "Store"), Role::Id);
    assert_eq!(role(&s, "width"), Role::Property);
    assert_eq!(role(&s, "110"), Role::Number);
}

#[test]
fn layout_distinguishes_addresses_operators_keys_values_and_numbers() {
    let s = inside(
        "layout",
        "Group.Service below Client : align=center-x; gap=110\nrequest.receive ? Service.work+ : align=center-y; reference=second",
    );
    for id in ["Group", "Service", "Client", "request", "receive", "work"] {
        assert_eq!(role(&s, id), Role::Id);
    }
    assert_eq!(role(&s, "below"), Role::Operator);
    assert_eq!(role(&s, "align"), Role::Property);
    assert_eq!(role(&s, "center-x"), Role::Value);
    assert_eq!(role(&s, "gap"), Role::Property);
    assert_eq!(role(&s, "110"), Role::Number);
    assert_eq!(role(&s, "second"), Role::Value);
}

#[test]
fn styling_selectors_classes_filters_and_properties_are_recognized() {
    let s = inside(
        "styling",
        "node[API].critical {\nfill: \"#128575\"\nfont-size: 14\n}\nmember[kind=field] {\ndisplay: none\n}\nimport \"shared.lamlstyle\"",
    );
    for keyword in ["node", "member", "import"] {
        assert_eq!(role(&s, keyword), Role::Keyword);
    }
    assert_eq!(role(&s, "API"), Role::Id);
    assert_eq!(role(&s, "critical"), Role::StyleClass);
    for key in ["fill", "font-size", "kind", "display"] {
        assert_eq!(role(&s, key), Role::Property);
    }
    assert_eq!(role(&s, "field"), Role::Value);
    assert_eq!(role(&s, "none"), Role::Value);
    assert_eq!(role(&s, "14"), Role::Number);
}

#[test]
fn uml_attributes_keep_names_types_visibility_and_default_literals_separate() {
    for (line, name, ty, literal, literal_role) in [
        (
            "@count +count: Integer = 42",
            "count",
            "Integer",
            "42",
            Role::Number,
        ),
        (
            "#name: String = \"Ada\"",
            "name",
            "String",
            "\"Ada\"",
            Role::Text,
        ),
        (
            "static int counter = 7",
            "counter",
            "int",
            "7",
            Role::Number,
        ),
        (
            "~enabled: Boolean = true",
            "enabled",
            "Boolean",
            "true",
            Role::Value,
        ),
    ] {
        let s = member(line);
        assert_eq!(role(&s, name), Role::Member, "{line}");
        assert_eq!(role(&s, ty), Role::Type, "{line}");
        assert_eq!(role(&s, literal), literal_role, "{line}");
    }
    assert_eq!(role(&member("#name: String"), "#"), Role::Operator);
    assert_eq!(role(&member("static int counter"), "static"), Role::Keyword);
    assert_eq!(
        role(&member("@count count: Integer"), "@count"),
        Role::ExplicitId
    );
}

#[test]
fn uml_operations_and_parameters_support_both_notations() {
    for line in [
        "+start(timeout: Integer, title: String): Result",
        "+Result start(Integer timeout, String title)",
    ] {
        let s = member(line);
        for name in ["start", "timeout", "title"] {
            assert_eq!(role(&s, name), Role::Member, "{line}");
        }
        for ty in ["Integer", "String", "Result"] {
            assert_eq!(role(&s, ty), Role::Type, "{line}");
        }
    }
}

#[test]
fn uml_parameter_defaults_do_not_recolor_later_parameters_or_return_types() {
    let s = member("+start(timeout: Integer = 42, title: String = \"Ada\"): Result");
    for name in ["start", "timeout", "title"] {
        assert_eq!(role(&s, name), Role::Member);
    }
    for ty in ["Integer", "String", "Result"] {
        assert_eq!(role(&s, ty), Role::Type);
    }
    assert_eq!(role(&s, "42"), Role::Number);
    assert_eq!(role(&s, "\"Ada\""), Role::Text);
}

#[test]
fn object_slots_retain_literal_roles() {
    let s = inside(
        "semantics",
        "order(of=Order) {\nname = \"Ada\"\ncount = 42\n}",
    );
    assert_eq!(role(&s, "name"), Role::Member);
    assert_eq!(role(&s, "count"), Role::Member);
    assert_eq!(role(&s, "42"), Role::Number);
}

#[test]
fn free_form_notes_do_not_change_context_or_invent_syntax() {
    let s = inside(
        "semantics",
        "note Reminder {\nclass if settings { https://example.org // text\n}\nclass Customer {\nname: String\n}",
    );
    let r = roles(&s);
    assert!(
        r.iter()
            .any(|(t, role)| t == "class" && *role == Role::Text)
    );
    assert_eq!(role(&s, "Customer"), Role::Id);
    assert_eq!(role(&s, "name"), Role::Member);
}

#[test]
fn comments_urls_escaped_quotes_and_protected_visibility_are_safe() {
    let s = inside(
        "semantics",
        "A(label=\"Grün \\\" // ok\") // comment\nA -> B : https://example.org\n",
    );
    assert_eq!(role(&s, "// comment"), Role::Comment);
    assert!(
        !roles(&s)
            .iter()
            .any(|(t, r)| t == "//example.org" && *r == Role::Comment)
    );
    assert_eq!(role(&member("#title: String"), "title"), Role::Member);
}

#[test]
fn gateway_branch_labels_do_not_swallow_later_targets() {
    let s = inside("semantics", "A <X>@decision B:yes ; C:no");
    assert_eq!(role(&s, "@decision"), Role::ExplicitId);
    assert_eq!(role(&s, "B"), Role::Id);
    assert_eq!(role(&s, "C"), Role::Id);
    assert_eq!(role(&s, "yes"), Role::Text);
    assert_eq!(role(&s, "no"), Role::Text);
}

#[test]
fn nested_packages_restore_member_and_section_context() {
    let s = "semantics {\npackage Shop {\nclass Customer {\nname: String\n}\n}\n}\nsettings {\ntheme: modern\n}\n";
    assert_eq!(role(s, "name"), Role::Member);
    assert_eq!(role(s, "theme"), Role::Property);
    assert_eq!(role(s, "modern"), Role::Value);
}

#[test]
fn unfinished_strings_recover_at_the_next_line() {
    let s = inside("settings", "font-file: \"unfinished\ntheme: modern-dark");
    assert_eq!(role(&s, "theme"), Role::Property);
}

#[test]
fn layout_jobs_preserve_every_byte_of_unicode_crlf_and_incomplete_input() {
    for source in [
        "",
        "// comment",
        "semantics {\r\nA(label=\"Grün 😀\")\r\n}",
        "A(label=\"unfinished",
        "@ä A::b -> B : \"x\\\"y\"\n",
    ] {
        for dark in [false, true] {
            let job = highlight(source, dark, Color32::WHITE);
            assert_eq!(job.text, source);
            assert!(
                job.sections
                    .iter()
                    .all(|s| source.is_char_boundary(s.byte_range.start.0)
                        && source.is_char_boundary(s.byte_range.end.0))
            );
            assert_eq!(
                job.sections
                    .iter()
                    .map(|s| s.byte_range.end.0 - s.byte_range.start.0)
                    .sum::<usize>(),
                source.len()
            );
        }
    }
}

#[test]
fn palettes_keep_roles_stable_and_bold_faces_are_monospace() {
    let normal = Color32::WHITE;
    for dark in [false, true] {
        for role in [Role::Section, Role::ExplicitId] {
            assert_eq!(
                role.format(dark, normal).font_id.family,
                egui::FontFamily::Name("source-bold".into())
            );
        }
        assert_eq!(
            Role::Id.format(dark, normal).color,
            Role::ExplicitId.format(dark, normal).color
        );
        assert_ne!(
            Role::Member.format(dark, normal).color,
            Role::Type.format(dark, normal).color
        );
        assert_ne!(
            Role::Number.format(dark, normal).color,
            Role::Text.format(dark, normal).color
        );
    }
    let ctx = egui::Context::default();
    let mut definitions = egui::FontDefinitions::default();
    install_fonts(&mut definitions);
    ctx.set_fonts(definitions);
    let mut output = ctx.run_ui(Default::default(), |ui| {
        ui.fonts_mut(|fonts| {
            let regular = fonts.layout_no_wrap(
                "@request settings".into(),
                egui::FontId::monospace(14.0),
                normal,
            );
            let bold = fonts.layout_no_wrap(
                "@request settings".into(),
                Role::Section.format(true, normal).font_id,
                normal,
            );
            assert!((regular.size().x - bold.size().x).abs() < 0.01);
        });
    });
    output.textures_delta.clear();
}

#[test]
fn repository_examples_remain_lossless_without_compilation() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let mut count = 0;
    for entry in std::fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path
            .extension()
            .is_some_and(|e| e == "laml" || e == "lamlstyle")
        {
            let source = std::fs::read_to_string(path).unwrap();
            assert_eq!(highlight(&source, true, Color32::WHITE).text, source);
            count += 1;
        }
    }
    assert!(count > 20);
}
