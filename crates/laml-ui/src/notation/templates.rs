//! Small, self-contained examples for explicitly created editor documents.
use super::Notation;

impl Notation {
    pub fn new_file(self, dark: bool, include_example: bool) -> String {
        let theme = if dark { "modern-dark" } else { "modern" };
        let extra = match self {
            Self::C4Context | Self::C4Container => "    scope: Shop\n",
            Self::C4Component => "    scope: API\n",
            Self::C4Code => "    scope: Orders\n",
            Self::Communication | Self::InteractionOverview | Self::Timing => {
                "    interaction: Example\n"
            }
            _ => "",
        };
        let (semantics, layout) = if include_example {
            self.example_parts()
        } else {
            ("", "")
        };
        let layout = if layout.is_empty() {
            String::new()
        } else {
            format!("layout {{\n{layout}}}\n")
        };
        let kind = if self == Self::Gne {
            String::new()
        } else {
            format!("    type: {}\n", self.key())
        };
        format!(
            "settings {{\n{kind}    theme: {theme}\n    background: transparent\n{extra}}}\nsemantics {{\n{semantics}}}\n{layout}"
        )
    }

    fn example_parts(self) -> (&'static str, &'static str) {
        match self {
            Self::Gne => ("    Browser\n    API\n    Browser -> API : request\n", ""),
            Self::Class => (
                "    class Customer {\n        name: String\n    }\n    class Order {\n        number: String\n    }\n    Customer -- Order : places\n",
                "",
            ),
            Self::Object => (
                "    customer(of=Customer) {\n        name = \"Ada\"\n    }\n    order(of=Order) {\n        number = \"B-17\"\n    }\n    customer -- order\n",
                "",
            ),
            Self::Package => (
                "    package Orders\n    package Payments\n    Orders ..> Payments\n",
                "",
            ),
            Self::Component => (
                "    component Web\n    component Service\n    Web ..> Service : requests\n",
                "",
            ),
            Self::Deployment => (
                "    device Server {\n        executionEnvironment Runtime {\n            artifact Application\n        }\n    }\n    device Database\n    Server -- Database : SQL\n",
                "",
            ),
            Self::Composite => (
                "    interface API\n    class Client {\n        port Out(requires=API)\n    }\n    class Service {\n        port In(provides=API)\n    }\n    structure System {\n        part client(of=Client)\n        part service(of=Service)\n        client::Out == service::In\n    }\n",
                "    System.client < System.service : gap=130\n",
            ),
            Self::Profile => (
                "    profile Audit {\n        stereotype Tracked\n    }\n    metaclass Class\n    Audit.Tracked --^> Class\n",
                "    Audit < Class : gap=170\n",
            ),
            Self::UseCase => (
                "    actor Customer\n    system Shop {\n        usecase Order(label=\"Place order\")\n    }\n    Customer -- Shop.Order\n",
                "",
            ),
            Self::State => (
                "    initial Start\n    state Idle\n    state Active\n    final Done\n    Start --> Idle\n    Idle --> Active : start\n    Active --> Done : finish\n",
                "",
            ),
            Self::Activity => (
                "    initial Start\n    action Review\n    activityFinal Done\n    control Start --> Review\n    control Review --> Done\n",
                "",
            ),
            Self::Communication => (
                "    object Customer\n    object Shop\n    Customer -- Shop\n    message 1 Customer --> Shop : placeOrder()\n    message 2 Shop --> Customer : confirmation()\n",
                "    Customer < Shop : gap=180\n",
            ),
            Self::InteractionOverview => (
                "    initial Start\n    interaction-use SignIn(ref=Authenticate)\n    final Done\n    Start --> SignIn\n    SignIn --> Done\n",
                "",
            ),
            Self::Sequence => (
                "    Client\n    Service\n    Client -> Service+ : request\n    <-- : response\n",
                "",
            ),
            Self::Timing => (
                "    lifeline Service : Idle\n    state Service at 2 : Active\n    state Service at 5 : Idle\n",
                "    end: 8\n    tick 0\n    tick 2\n    tick 5\n    tick 7\n",
            ),
            Self::Bpmn => (
                "    startEvent Start\n    endEvent Done\n    Start --> Review(label=\"Review order\") --> Done\n",
                "",
            ),
            Self::C4Context => (
                "    person Customer(description=\"Places orders\")\n    system Shop(description=\"Handles orders\")\n    Customer -> Shop : Places order\n",
                "",
            ),
            Self::C4Container => (
                "    system Shop(description=\"Handles orders\") {\n        container Web(technology=\"Browser\"; description=\"Captures orders\")\n        container API(technology=\"Rust\"; description=\"Processes orders\")\n    }\n    Shop.Web -> Shop.API : Submits order\n",
                "",
            ),
            Self::C4Component => (
                "    container API(technology=\"Rust\"; description=\"Processes orders\") {\n        component Handler(technology=\"Rust\"; description=\"Accepts requests\")\n        component Orders(technology=\"Rust\"; description=\"Coordinates orders\")\n    }\n    API.Handler -> API.Orders : Places order\n",
                "",
            ),
            Self::C4Code => (
                "    component Orders(technology=\"Rust\"; description=\"Coordinates orders\") {\n        class OrderService {\n            +place(order: Order)\n        }\n        class Order {\n            +number: String\n        }\n        OrderService --> Order : validates\n    }\n",
                "",
            ),
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    #[test]
    fn every_new_file_example_builds_in_both_themes() {
        for ty in Notation::ALL {
            for dark in [false, true] {
                let source = ty.new_file(dark, true);
                let (_, nodes, relations) = super::super::inspect(&source)
                    .unwrap_or_else(|e| panic!("{}: {e}\n{source}", ty.key()));
                assert!(nodes > 0, "{} must show an example", ty.key());
                assert!(
                    relations > 0,
                    "{} must demonstrate a relationship or event",
                    ty.key()
                );
                let preview = crate::preview::build(&source, std::path::Path::new("new.laml"))
                    .unwrap_or_else(|e| panic!("{}: {e}\n{source}", ty.key()));
                assert_eq!(preview.notation, ty);
            }
        }
    }
    #[test]
    fn empty_files_retain_the_selected_type_without_example_elements() {
        for ty in Notation::ALL {
            let source = ty.new_file(false, false);
            assert_eq!(Notation::of(&source), ty);
            assert!(source.contains("semantics {\n}"));
            assert!(source.ends_with("semantics {\n}\n"));
            assert!(!source.contains("layout {"));
        }
    }
}
