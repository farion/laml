use laml::{
    export,
    geometry::place,
    parser::parse,
    route::route,
    styles::{StyleSheet, load},
    svg::{TextMode, render, render_with_mode},
};

const SOURCE: &str = "semantics {\nService {\n+run()\n-id: Int\n}\nClient\nService --> Client\n}\nstyling {\n* {\nsymbol-rendering: icons\nicon-set: phosphor\n}\nnode[Service] {\nicon: \"folder\"\n}\nmember[Service::id] {\nsymbol-rendering: text\n}\n}\n";
const COPYRIGHT: &str = "Copyright (c) 2023 Phosphor Icons";

#[test]
fn phosphor_notice_is_conditional_in_svg_png_and_pdf_metadata() {
    let diagram = parse(SOURCE).unwrap();
    let styles = load(&diagram, std::path::Path::new("examples/architecture.laml")).unwrap();
    let boxes = place(&diagram, &styles).unwrap();
    let routes = route(&diagram, &boxes, &styles).unwrap();
    let svg = render(&boxes, &routes).unwrap();
    assert!(svg.contains("data-phosphor-icon=\"folder\""));
    assert!(svg.contains("data-phosphor-icon=\"globe\""));
    assert!(!svg.contains("data-phosphor-icon=\"lock\""));
    assert!(svg.contains("-id: Int"));
    assert!(svg.contains("<metadata id=\"phosphor-license\">"));
    assert!(svg.contains(COPYRIGHT));
    let outlined = render_with_mode(&boxes, &routes, TextMode::Paths).unwrap();
    assert!(outlined.contains("<metadata id=\"phosphor-license\">"));
    assert!(outlined.contains(COPYRIGHT));
    let png = export::png(&svg).unwrap();
    assert!(
        png.windows(COPYRIGHT.len())
            .any(|window| window == COPYRIGHT.as_bytes())
    );
    let pdf = export::pdf(&svg).unwrap();
    let document = lopdf::Document::load_mem(&pdf).unwrap();
    let info = document
        .trailer
        .get(b"Info")
        .unwrap()
        .as_reference()
        .unwrap();
    let dictionary = document.get_object(info).unwrap().as_dict().unwrap();
    let subject = dictionary.get(b"Subject").unwrap().as_str().unwrap();
    assert!(
        subject
            .windows(COPYRIGHT.len())
            .any(|window| window == COPYRIGHT.as_bytes())
    );
    assert!(!String::from_utf8_lossy(&pdf).contains("<metadata id"));
    let plain = "semantics {\nA\nB\nA --> B\n}\n";
    let diagram = parse(plain).unwrap();
    let boxes = place(&diagram, &StyleSheet::default()).unwrap();
    let routes = route(&diagram, &boxes, &StyleSheet::default()).unwrap();
    let svg = render(&boxes, &routes).unwrap();
    assert!(!svg.contains(COPYRIGHT));
    let png = export::png(&svg).unwrap();
    assert!(
        !png.windows(COPYRIGHT.len())
            .any(|window| window == COPYRIGHT.as_bytes())
    );
    let pdf = export::pdf(&svg).unwrap();
    assert!(
        !pdf.windows(COPYRIGHT.len())
            .any(|window| window == COPYRIGHT.as_bytes())
    );
}

#[test]
fn icon_properties_reject_unknown_assets_and_uml_shape_pollution() {
    let bad = SOURCE.replace("\"folder\"", "\"made-up\"");
    let diagram = parse(&bad).unwrap();
    assert!(load(&diagram, std::path::Path::new("examples/architecture.laml")).is_err());
    let uml = SOURCE
        .replacen("semantics {", "settings {\ntype: class\n}\nsemantics {", 1)
        .replace("+run()\n-id: Int", "-id: Int\n--\n+run()");
    let diagram = parse(&uml).unwrap();
    let sheet = load(&diagram, std::path::Path::new("examples/architecture.laml")).unwrap();
    assert!(
        place(&diagram, &sheet)
            .unwrap_err()
            .message
            .contains("additional icons")
    );
    let missing = SOURCE.replace("icon-set: phosphor\n", "");
    let diagram = parse(&missing).unwrap();
    let sheet = load(&diagram, std::path::Path::new("examples/architecture.laml")).unwrap();
    assert!(
        place(&diagram, &sheet)
            .unwrap_err()
            .message
            .contains("icon-set")
    );
}
