//! Resolve component parts to declared component types and copy their simple ports.
use super::{ParseError, error, references::resolve};
use crate::syntax::{Diagram, NodeKind, PortContract};

pub(super) fn expand(diagram: &mut Diagram) -> Result<(), ParseError> {
    let definitions = diagram.nodes.clone();
    for part in &mut diagram.nodes {
        if !matches!(
            part.kind,
            NodeKind::ComponentPart | NodeKind::CompositePart | NodeKind::CompositeRole
        ) {
            continue;
        }
        let owner = part
            .parent
            .as_deref()
            .and_then(|parent| definitions.iter().find(|n| n.id == parent));
        let composite = matches!(part.kind, NodeKind::CompositePart | NodeKind::CompositeRole);
        let expected_owner = if composite {
            NodeKind::CompositeStructure
        } else {
            NodeKind::Component
        };
        let expected_type = if composite {
            NodeKind::Class
        } else {
            NodeKind::Component
        };
        if !owner.is_some_and(|node| node.kind == expected_owner) {
            return Err(error(
                part.line,
                if composite {
                    "composite part or role must be directly owned by a structure"
                } else {
                    "part must be directly owned by a component"
                },
            ));
        }
        let raw = part.part_type.as_deref().ok_or_else(|| {
            error(
                part.line,
                if composite {
                    "part or role needs of=Class"
                } else {
                    "part needs of=Component"
                },
            )
        })?;
        let id = resolve(
            &definitions,
            owner.and_then(|node| node.parent.as_deref()),
            raw,
        );
        let ty = definitions
            .iter()
            .find(|node| node.id == id && node.kind == expected_type)
            .ok_or_else(|| {
                error(
                    part.line,
                    if composite {
                        format!(
                            "part or role '{}' needs a declared class type '{raw}'",
                            part.id
                        )
                    } else {
                        format!("part '{}' needs a declared component type '{raw}'", part.id)
                    },
                )
            })?;
        if ty.id == owner.unwrap().id || ty.parent.as_deref() == Some(part.id.as_str()) {
            return Err(error(
                part.line,
                if composite {
                    "structured classifier cannot contain a role typed by itself"
                } else {
                    "component cannot contain a part of itself"
                },
            ));
        }
        part.label = Some(format!(
            "{}:{}",
            part.label
                .as_deref()
                .unwrap_or_else(|| part.id.rsplit('.').next().unwrap_or(&part.id)),
            ty.label.as_deref().unwrap_or(&ty.id)
        ));
        part.part_type = Some(ty.id.clone());
        part.ports = ty
            .ports
            .iter()
            .map(|port| {
                let mut copied = port.clone();
                copied.id = port.label.clone();
                copied.explicit = false;
                copied.line = part.line;
                if let Some(contract) = &mut copied.contract {
                    let interface = match contract {
                        PortContract::Provides(id) | PortContract::Requires(id) => id,
                    };
                    *interface = resolve(&definitions, ty.parent.as_deref(), interface);
                }
                copied
            })
            .collect();
    }
    Ok(())
}
