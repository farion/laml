//! Classify source node headings; UML node kinds remain explicit and diagram-scoped.
use crate::syntax::{DiagramType, NodeKind};

pub(super) fn classify<'a>(
    source: &'a str,
    note: Option<&'a str>,
    anonymous: bool,
    diagram_type: DiagramType,
) -> (NodeKind, &'a str) {
    if let Some(name) = note {
        (NodeKind::Note, name)
    } else if let Some(name) = source.strip_prefix("association class ") {
        (NodeKind::AssociationClass, name)
    } else if source == "association" {
        (NodeKind::Association, "")
    } else if source == "junction" {
        (NodeKind::Junction, "")
    } else if let Some(name) = source.strip_prefix("association ") {
        (NodeKind::Association, name)
    } else if let Some(name) = source.strip_prefix("junction ") {
        (NodeKind::Junction, name)
    } else if let Some(name) = source.strip_prefix("package ") {
        (NodeKind::Package, name)
    } else if let Some(name) = source.strip_prefix("class ") {
        (NodeKind::Class, name)
    } else if let Some(name) = source.strip_prefix("component ") {
        (NodeKind::Component, name)
    } else if let Some(name) = source.strip_prefix("part ") {
        (NodeKind::ComponentPart, name)
    } else if let Some(name) = source.strip_prefix("interface ") {
        (NodeKind::Interface, name)
    } else if let Some(name) = source.strip_prefix("enum ") {
        (NodeKind::Enumeration, name)
    } else if let Some(name) = source.strip_prefix("datatype ") {
        (NodeKind::DataType, name)
    } else if let Some(name) = source.strip_prefix("device ") {
        (NodeKind::Device, name)
    } else if let Some(name) = source.strip_prefix("executionEnvironment ") {
        (NodeKind::ExecutionEnvironment, name)
    } else if let Some(name) = source.strip_prefix("node ") {
        (NodeKind::DeploymentNode, name)
    } else if let Some(name) = source.strip_prefix("artifact ") {
        (NodeKind::Artifact, name)
    } else if anonymous && source.starts_with("object(") {
        (NodeKind::Object, &source["object".len()..])
    } else if let Some(name) = source.strip_prefix("object ") {
        (NodeKind::Object, name)
    } else {
        match diagram_type {
            DiagramType::Generic => (NodeKind::Generic, source),
            DiagramType::Class => (NodeKind::Class, source),
            DiagramType::Object => (NodeKind::Object, source),
            DiagramType::Component => (NodeKind::Component, source),
            DiagramType::Deployment => (NodeKind::Generic, source),
        }
    }
}
