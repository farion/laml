//! One screen-space hit map. Every gesture starts from one explicit area.
use super::*;
use laml::authoring::Region;

#[derive(Clone, Debug)]
pub(super) struct Point {
    pub dock: Dock,
    pub at: egui::Pos2,
    pub same_row: bool,
    pub straighten: bool,
}
#[derive(Clone, Debug)]
pub(super) struct Bar {
    pub owner: String,
    pub region: Region,
    pub rect: egui::Rect,
}
#[derive(Clone, Debug)]
pub(super) enum Hit {
    Participant(Region),
    Activation(Bar),
    ParticipantPlus { neighbor: String, left: bool },
    Insert(Point),
    Origin(Point),
    Resize(Bar, bool),
    Endpoint { message: Region, source: bool },
    Event(Region),
    Destroy { owner: String, before: usize },
    Destruction(Region),
}
#[derive(Clone)]
pub(super) struct Area {
    pub rect: egui::Rect,
    pub hit: Hit,
}
pub(super) struct Scene {
    pub areas: Vec<Area>,
    pub docks: Vec<Point>,
    pub bars: Vec<Bar>,
    pub heads: Vec<(Region, egui::Rect)>,
    pub regions: Vec<Region>,
    pub bounds: (f64, f64, f64, f64),
    pub image: egui::Rect,
    pub labels: Vec<egui::Rect>,
}
impl Scene {
    pub fn new(preview: &Preview, image: egui::Rect) -> Self {
        let mut scene = Self {
            areas: vec![],
            docks: vec![],
            bars: vec![],
            heads: vec![],
            regions: preview.regions.clone(),
            bounds: preview.bounds,
            image,
            labels: vec![],
        };
        scene.labels = preview
            .texts
            .iter()
            .map(|t| {
                egui::Rect::from_min_max(
                    scene.point(t.x, t.y),
                    scene.point(t.x + t.width, t.y + t.height),
                )
            })
            .collect();
        for r in &preview.regions {
            if r.kind == "participant" {
                let shape = scene.shape(r);
                scene.heads.push((r.clone(), shape));
            } else if r.kind == "sequence-execution" {
                scene.bars.push(Bar {
                    owner: r.id.clone(),
                    region: r.clone(),
                    rect: scene.shape(r),
                });
            }
        }
        for r in &mut scene.regions {
            if r.kind == "sequence-slot" && r.id.ends_with(".slot:end") {
                r.line = 0;
            }
        }
        scene.build_docks();
        scene.build_areas();
        super::plus::arrange(&mut scene);
        scene
    }
    pub fn point(&self, x: f64, y: f64) -> egui::Pos2 {
        let scale = f64::from(self.image.width()) / (self.bounds.2 - self.bounds.0);
        self.image.min
            + egui::vec2(
                ((x - self.bounds.0) * scale) as f32,
                ((y - self.bounds.1) * scale) as f32,
            )
    }
    pub(super) fn bar_for(&self, point: &Point) -> Option<&Bar> {
        self.bars
            .iter()
            .filter(|bar| {
                bar.owner == point.dock.owner
                    && match &point.dock.boundary {
                        Some(Boundary::Start(start) | Boundary::End { start, .. }) => {
                            bar.region.line == *start
                        }
                        None => point.at.y > bar.rect.top() && point.at.y < bar.rect.bottom(),
                    }
            })
            .min_by(|a, b| {
                if point.dock.boundary.is_some() {
                    (a.rect.center().x - point.at.x)
                        .abs()
                        .total_cmp(&(b.rect.center().x - point.at.x).abs())
                } else {
                    b.region.x.total_cmp(&a.region.x)
                }
            })
    }
    pub fn shape(&self, r: &Region) -> egui::Rect {
        egui::Rect::from_min_max(
            self.point(r.x, r.y),
            self.point(r.x + r.width, r.y + r.height),
        )
    }
    pub fn hit(&self, pos: egui::Pos2) -> Option<Hit> {
        self.hit_filtered(pos, |_| true)
    }
    pub fn hit_filtered(&self, pos: egui::Pos2, visible: impl Fn(&Area) -> bool) -> Option<Hit> {
        let scale = f64::from(self.image.width()) / (self.bounds.2 - self.bounds.0);
        let x = self.bounds.0 + f64::from(pos.x - self.image.left()) / scale;
        let y = self.bounds.1 + f64::from(pos.y - self.image.top()) / scale;
        // Actual bracket strokes win over nearby controls' generous hit margins.
        if let Some(region) = self.regions.iter().find(|r| {
            r.kind == "coregion"
                && super::super::canvas::hit(std::slice::from_ref(r), x, y, 2.0 / scale).is_some()
        }) {
            return Some(Hit::Event(region.clone()));
        }
        self.areas
            .iter()
            .find(|a| a.rect.contains(pos) && visible(a))
            .map(|a| a.hit.clone())
            .or_else(|| {
                let r = super::super::canvas::hit(&self.regions, x, y, 5.0 / scale)?;
                // Frame interiors belong to their contained objects, not to a drag zone.
                if matches!(r.kind.as_str(), "frame" | "decomposition")
                    && self.shape(r).shrink(8.0).contains(pos)
                    && pos.y > self.shape(r).top() + 24.0
                {
                    return None;
                }
                if r.kind == "execution"
                    && let Some(bar) = self
                        .bars
                        .iter()
                        .find(|b| b.region.line == r.line && (b.region.x - r.x).abs() < 0.1)
                {
                    return Some(Hit::Activation(bar.clone()));
                }
                Some(if r.kind == "destroyed-end" {
                    Hit::Destruction(r.clone())
                } else {
                    Hit::Event(r.clone())
                })
            })
    }
    fn add(&mut self, rect: egui::Rect, hit: Hit) {
        self.areas.push(Area { rect, hit });
    }
    fn push_dock(&mut self, dock: Dock, at: egui::Pos2, same_row: bool) {
        if !self
            .docks
            .iter()
            .any(|p| p.dock == dock && p.same_row == same_row && p.at.distance(at) < 0.1)
        {
            self.docks.push(Point {
                dock,
                at,
                same_row,
                straighten: false,
            });
        }
    }
    fn build_docks(&mut self) {
        let lines = self
            .regions
            .iter()
            .filter(|r| r.kind == "lifeline")
            .cloned()
            .collect::<Vec<_>>();
        let slots = self
            .regions
            .iter()
            .filter(|r| r.kind == "sequence-slot")
            .cloned()
            .collect::<Vec<_>>();
        for line in lines {
            for slot in &slots {
                if slot.y < line.y || slot.y > line.y + line.height {
                    continue;
                }
                // Decomposition slots are scoped by their compiler-qualified ID.
                let scope = |id: &str| id.rsplit_once('.').map(|(scope, _)| scope.to_owned());
                if scope(&line.id) != scope(&slot.id) {
                    continue;
                }
                let bar = self
                    .bars
                    .iter()
                    .filter(|b| {
                        b.owner == line.id
                            && slot.y > b.region.y
                            && slot.y < b.region.y + b.region.height
                    })
                    .max_by(|a, b| a.region.x.total_cmp(&b.region.x));
                let x = bar.map_or(line.x, |b| b.region.x + b.region.width / 2.0);
                let dock = Dock {
                    owner: line.id.clone(),
                    before: slot.line,
                    boundary: None,
                };
                self.push_dock(dock, self.point(x, slot.y), false);
            }
        }
        for bar in self.bars.clone() {
            for (boundary, before, at) in [
                (
                    Boundary::Start(bar.region.line),
                    bar.region.line,
                    egui::pos2(bar.rect.center().x, bar.rect.top()),
                ),
                (
                    Boundary::End {
                        start: bar.region.line,
                        end: bar.region.end_line,
                    },
                    bar.region.end_line.unwrap_or(0),
                    egui::pos2(bar.rect.center().x, bar.rect.bottom()),
                ),
            ] {
                self.push_dock(
                    Dock {
                        owner: bar.owner.clone(),
                        before,
                        boundary: Some(boundary),
                    },
                    at,
                    false,
                );
            }
        }
    }
    fn build_areas(&mut self) {
        // Priority is intentional, but independent controls occupy disjoint areas.
        for (head, shape) in self.heads.clone() {
            for left in [true, false] {
                let at = egui::pos2(
                    if left {
                        shape.left() - 17.0
                    } else {
                        shape.right() + 17.0
                    },
                    shape.center().y,
                );
                self.add(
                    egui::Rect::from_center_size(at, egui::Vec2::splat(22.0)),
                    Hit::ParticipantPlus {
                        neighbor: head.id.clone(),
                        left,
                    },
                );
            }
        }
        for bar in self.bars.clone() {
            for lower in [false, true] {
                let y = if lower {
                    bar.rect.bottom()
                } else {
                    bar.rect.top()
                };
                self.add(
                    egui::Rect::from_center_size(
                        egui::pos2(bar.rect.center().x, y),
                        egui::vec2(bar.rect.width().max(12.0), 6.0),
                    ),
                    Hit::Resize(bar.clone(), lower),
                );
            }
        }
        let messages = self
            .regions
            .iter()
            .filter(|r| r.kind == "message" && !r.path.is_empty())
            .cloned()
            .collect::<Vec<_>>();
        for message in messages {
            let points = &message.path;
            for source in [true, false] {
                let (end, next) = if source {
                    (points[0], points[1])
                } else {
                    (points[points.len() - 1], points[points.len() - 2])
                };
                let at = self.point(end.0, end.1);
                let toward = self.point(next.0, next.1) - at;
                let grip = at + toward.normalized() * toward.length().min(18.0);
                self.add(
                    egui::Rect::from_center_size(grip, egui::Vec2::splat(12.0)),
                    Hit::Endpoint {
                        message: message.clone(),
                        source,
                    },
                );
            }
        }
        for r in self
            .regions
            .clone()
            .iter()
            .filter(|r| matches!(r.kind.as_str(), "lifeline-end" | "destroyed-end"))
        {
            let rect = self.shape(r).expand(6.0);
            let hit = if r.kind == "lifeline-end" {
                // Qualified end slots carry the actual local closing boundary.
                let scope = r.id.rsplit_once('.').map(|(s, _)| s);
                let before = self
                    .regions
                    .iter()
                    .find(|s| {
                        s.kind == "sequence-slot"
                            && s.id.ends_with("slot:end")
                            && s.id.rsplit_once('.').map(|(s, _)| s) == scope
                    })
                    .map_or(0, |s| s.line);
                Hit::Destroy {
                    owner: r.id.clone(),
                    before,
                }
            } else {
                Hit::Destruction(r.clone())
            };
            self.add(rect, hit);
        }
        for p in self.docks.clone() {
            if p.dock.boundary.is_some() {
                continue;
            }
            let plus = p.at + egui::vec2(22.0, 0.0);
            self.add(
                egui::Rect::from_center_size(plus, egui::Vec2::splat(20.0)),
                Hit::Insert(p.clone()),
            );
            let mut origin = p;
            let mut height = 20.0;
            if let Some(bar) = self.bar_for(&origin) {
                if bar.rect.height() <= 20.0 {
                    continue;
                }
                origin.at.x = bar.rect.center().x;
                height = (bar.rect.height() - 16.0).min(height);
                if bar.rect.height() >= 60.0
                    && self.labels.iter().any(|t| {
                        t.intersects(egui::Rect::from_center_size(
                            origin.at,
                            egui::Vec2::splat(20.0),
                        ))
                    })
                {
                    continue;
                }
            }
            self.add(
                egui::Rect::from_center_size(origin.at, egui::vec2(20.0, height)),
                Hit::Origin(origin),
            );
        }
        // Larger, centered origins stay clear of edge resizing. Short bars get
        // one center grip instead of overlapping top/bottom grips.
        for bar in self.bars.clone() {
            if bar.rect.height() <= 20.0 {
                continue;
            }
            for lower in [false, true] {
                let short = bar.rect.height() < 60.0;
                if short && lower {
                    continue;
                }
                let mut at = egui::pos2(
                    bar.rect.center().x,
                    if short {
                        bar.rect.center().y
                    } else if lower {
                        bar.rect.bottom() - 18.0
                    } else {
                        bar.rect.top() + 18.0
                    },
                );
                let mut clear = at;
                for label in &self.labels {
                    if label.expand(12.0).contains(clear) {
                        clear.y = if lower {
                            label.top() - 12.0
                        } else {
                            label.bottom() + 12.0
                        };
                    }
                }
                if clear.y >= bar.rect.top() + 18.0 && clear.y <= bar.rect.bottom() - 18.0 {
                    at = clear;
                }
                let boundary = if lower {
                    Boundary::End {
                        start: bar.region.line,
                        end: bar.region.end_line,
                    }
                } else {
                    Boundary::Start(bar.region.line)
                };
                let dock = Dock {
                    owner: bar.owner.clone(),
                    before: if lower {
                        bar.region.end_line.unwrap_or(0)
                    } else {
                        bar.region.line
                    },
                    boundary: Some(boundary),
                };
                let edge = if lower {
                    bar.region.end_line
                } else {
                    Some(bar.region.line)
                };
                let bound = edge.is_some_and(|line| {
                    self.regions
                        .iter()
                        .any(|r| r.kind == "message" && r.line == line)
                });
                let dock = if bound {
                    let Some(point) = self
                        .docks
                        .iter()
                        .filter(|p| {
                            p.dock.owner == bar.owner
                                && p.dock.boundary.is_none()
                                && p.at.y > bar.rect.top()
                                && p.at.y < bar.rect.bottom()
                        })
                        .min_by(|a, b| (a.at.y - at.y).abs().total_cmp(&(b.at.y - at.y).abs()))
                    else {
                        continue;
                    };
                    point.dock.clone()
                } else {
                    dock
                };
                self.areas.retain(|area| !matches!(&area.hit, Hit::Origin(p)
                    if p.dock.owner == bar.owner && (p.at.x-at.x).abs() < 0.1 && (p.at.y-at.y).abs() < 22.0));
                self.add(
                    egui::Rect::from_center_size(
                        at,
                        egui::vec2(20.0, (bar.rect.height() - 16.0).min(20.0)),
                    ),
                    Hit::Origin(Point {
                        dock,
                        at,
                        same_row: false,
                        straighten: false,
                    }),
                );
            }
        }
        for (head, shape) in self.heads.clone() {
            self.add(shape, Hit::Participant(head));
        }
    }
}

#[cfg(test)]
#[path = "scene_tests.rs"]
mod tests;
