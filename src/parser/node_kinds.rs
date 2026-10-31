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
    } else if let Some(name) = source.strip_prefix("actor ") {
        (NodeKind::UseCaseActor, name)
    } else if let Some(name) = source.strip_prefix("usecase ") {
        (NodeKind::UseCase, name)
    } else if let Some(name) = source.strip_prefix("system ") {
        (NodeKind::UseCaseSystem, name)
    } else if let Some(name) = source.strip_prefix("action ") {
        (NodeKind::ActivityAction, name)
    } else if let Some(name) = source.strip_prefix("objectNode ") {
        (NodeKind::ActivityObject, name)
    } else if let Some(name) = source.strip_prefix("activityFinal ") {
        (NodeKind::ActivityFinal, name)
    } else if let Some(name) = source.strip_prefix("flowFinal ") {
        (NodeKind::ActivityFlowFinal, name)
    } else if let Some(name) = source.strip_prefix("decision ") {
        (NodeKind::ActivityDecision, name)
    } else if let Some(name) = source.strip_prefix("merge ") {
        (NodeKind::ActivityMerge, name)
    } else if let Some(name) = source.strip_prefix("fork ") {
        (NodeKind::ActivityFork, name)
    } else if let Some(name) = source.strip_prefix("join ") {
        (NodeKind::ActivityJoin, name)
    } else if diagram_type == DiagramType::Activity && source.starts_with("initial ") {
        (NodeKind::ActivityInitial, &source["initial ".len()..])
    } else if let Some(name) = source.strip_prefix("state ") {
        (NodeKind::State, name)
    } else if let Some(name) = source.strip_prefix("initial ") {
        (NodeKind::StateInitial, name)
    } else if let Some(name) = source.strip_prefix("final ") {
        (NodeKind::StateFinal, name)
    } else if let Some(name) = source.strip_prefix("choice ") {
        (NodeKind::StateChoice, name)
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
            DiagramType::Deployment
            | DiagramType::State
            | DiagramType::Activity
            | DiagramType::UseCase => (NodeKind::Generic, source),
        }
    }
}

pub(super) fn allowed(kind: NodeKind, ty: DiagramType) -> bool {
    let deployment = matches!(
        kind,
        NodeKind::DeploymentNode
            | NodeKind::Device
            | NodeKind::ExecutionEnvironment
            | NodeKind::Artifact
    );
    let state = matches!(
        kind,
        NodeKind::State | NodeKind::StateInitial | NodeKind::StateFinal | NodeKind::StateChoice
    );
    let usecase = matches!(
        kind,
        NodeKind::UseCaseActor | NodeKind::UseCase | NodeKind::UseCaseSystem
    );
    let activity = matches!(
        kind,
        NodeKind::ActivityAction
            | NodeKind::ActivityObject
            | NodeKind::ActivityInitial
            | NodeKind::ActivityFinal
            | NodeKind::ActivityFlowFinal
            | NodeKind::ActivityDecision
            | NodeKind::ActivityMerge
            | NodeKind::ActivityFork
            | NodeKind::ActivityJoin
    );
    match ty {
        DiagramType::Activity => activity || kind == NodeKind::Note,
        DiagramType::UseCase => usecase || kind == NodeKind::Note,
        DiagramType::Object => matches!(kind, NodeKind::Object | NodeKind::Note),
        DiagramType::Component => matches!(
            kind,
            NodeKind::Component
                | NodeKind::ComponentPart
                | NodeKind::Interface
                | NodeKind::Package
                | NodeKind::Note
        ),
        DiagramType::Deployment => {
            deployment
                || matches!(
                    kind,
                    NodeKind::Component | NodeKind::Package | NodeKind::Note
                )
        }
        DiagramType::State => state || kind == NodeKind::Note,
        DiagramType::Class => {
            !matches!(
                kind,
                NodeKind::Object
                    | NodeKind::Junction
                    | NodeKind::Component
                    | NodeKind::ComponentPart
            ) && !deployment
                && !state
                && !activity
                && !usecase
        }
        DiagramType::Generic => {
            !matches!(kind, NodeKind::Component | NodeKind::ComponentPart)
                && !deployment
                && !state
                && !activity
                && !usecase
        }
    }
}
