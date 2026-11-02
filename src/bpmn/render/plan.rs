//! Prefer primary flows first; compare connector orders and source order by
//! crossings before bend/length cost. Keep results indexed by source edge.
use super::orthogonal::{self, Prior};
use super::ports::Ports;
use super::{Document, EdgeKind, Geometry, edges, labels};
type Planned = (String, Vec<Prior>);
type Scored = ((usize, f64), Planned);

fn one(
    doc: &Document,
    geom: &Geometry,
    ports: &Ports,
    data_first: Option<bool>,
) -> Result<Planned, String> {
    let mut indices: Vec<_> = (0..doc.edges.len()).collect();
    if let Some(data_first) = data_first {
        indices.sort_by_key(|&i| match doc.edges[i].kind {
            EdgeKind::Sequence => 0,
            EdgeKind::Message => 1,
            EdgeKind::Data => {
                if data_first {
                    2
                } else {
                    3
                }
            }
            EdgeKind::Association => {
                if data_first {
                    3
                } else {
                    2
                }
            }
        });
    }
    let mut previous = Vec::new();
    let mut paths = String::new();
    let mut by_index = vec![None; doc.edges.len()];
    for index in indices {
        edges::edge(
            &mut paths,
            doc,
            geom,
            &doc.edges[index],
            index,
            ports,
            &mut previous,
        )?;
        by_index[index] = previous.last().cloned();
    }
    Ok((paths, by_index.into_iter().map(Option::unwrap).collect()))
}
fn quality(routes: &[Prior]) -> (usize, f64) {
    let mut crossings = 0;
    let mut cost = 0.0;
    for (i, route) in routes.iter().enumerate() {
        for other in routes.iter().skip(i + 1) {
            crossings += orthogonal::bridges(&route.points, std::slice::from_ref(other)).len();
        }
        cost += route
            .points
            .windows(2)
            .map(|leg| (leg[0].0 - leg[1].0).abs() + (leg[0].1 - leg[1].1).abs())
            .sum::<f64>();
        cost += route.points.len().saturating_sub(2) as f64 * 100.0;
    }
    (crossings, cost)
}
pub(super) fn all(
    doc: &Document,
    geom: &Geometry,
    ports: &Ports,
) -> Result<(String, Vec<Prior>), String> {
    let mut best: Option<Scored> = None;
    let mut error = None;
    for data_first in [Some(false), Some(true), None] {
        match one(doc, geom, ports, data_first) {
            Ok(result) => {
                if let Err(reason) = labels::render(&mut String::new(), doc, geom, &result.1) {
                    error = Some(reason);
                    continue;
                }
                let score = quality(&result.1);
                if best.as_ref().is_none_or(|(previous, _)| {
                    score.0 < previous.0 || (score.0 == previous.0 && score.1 < previous.1)
                }) {
                    best = Some((score, result));
                }
            }
            Err(reason) => error = Some(reason),
        }
    }
    best.map(|(_, result)| result)
        .ok_or_else(|| error.unwrap_or_else(|| "no valid BPMN routing order".into()))
}
