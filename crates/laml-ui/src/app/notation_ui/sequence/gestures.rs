//! A single gesture owns the pointer from threshold crossing to release/cancel.
use super::*;
#[derive(Clone)]
pub(super) struct Gesture {
    pub hit: Hit,
    pub destinations: Vec<Destination>,
    pub hot: Option<usize>,
}
impl Gesture {
    fn nearest(&self, pointer: egui::Pos2) -> Option<usize> {
        self.destinations
            .iter()
            .enumerate()
            .filter_map(|(i, d)| {
                let distance = if matches!(self.hit, Hit::Participant(_)) {
                    (pointer.x - d.at.x).abs()
                } else if matches!(
                    self.hit,
                    Hit::Resize(..) | Hit::Event(_) | Hit::Destruction(_)
                ) {
                    (pointer.y - d.at.y).abs()
                } else {
                    d.hit_rect
                        .map_or_else(|| pointer.distance(d.at), |r| r.distance_to_pos(pointer))
                };
                (distance <= 16.0).then_some((i, distance, pointer.distance(d.at)))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1).then(a.2.total_cmp(&b.2)))
            .map(|(i, _, _)| i)
    }
}
impl Editor {
    pub(super) fn sequence_gesture(&mut self, ui: &egui::Ui, scene: &Scene) {
        let Some(pointer) = ui.input(|i| i.pointer.interact_pos()) else {
            self.notation_ui.sequence.gesture = None;
            self.notation_ui.sequence.press = None;
            self.notation_ui.sequence.pending_targets = None;
            self.notation_ui.sequence.released = None;
            return;
        };
        if self.notation_ui.sequence.gesture.is_none()
            && let Some((hit, origin)) = self.notation_ui.sequence.press.clone()
            && pointer.distance(origin) >= 5.0
            && ui.input(|i| i.pointer.primary_down() || i.pointer.primary_released())
        {
            let destinations = if matches!(hit, Hit::Origin(_) | Hit::Endpoint { .. }) {
                match message_targets::start(self, scene, &hit) {
                    Ok(pending) => self.notation_ui.sequence.pending_targets = pending,
                    Err(error) => {
                        self.command_error = Some(error);
                        self.notation_ui.sequence.press = None;
                        return;
                    }
                }
                Vec::new()
            } else {
                proposals::destinations(self.document.source(), scene, &hit, &self.graph.operator)
            };
            self.notation_ui.sequence.released = None;
            self.notation_ui.sequence.gesture = Some(Gesture {
                hit,
                destinations,
                hot: None,
            });
            self.notation_ui.sequence.press = None;
        }
        let Some(mut gesture) = self.notation_ui.sequence.gesture.take() else {
            return;
        };
        let pointer = self.notation_ui.sequence.released.unwrap_or(pointer);
        if let Some(pending) = self.notation_ui.sequence.pending_targets.as_mut() {
            if let Some(result) = pending.poll(scene) {
                self.notation_ui.sequence.pending_targets = None;
                match result {
                    Ok(destinations) => gesture.destinations = destinations,
                    Err(error) => {
                        self.command_error = Some(error);
                        self.notation_ui.sequence.released = None;
                        return;
                    }
                }
            } else {
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_millis(16));
            }
        }
        gesture.hot = ui
            .clip_rect()
            .contains(pointer)
            .then(|| gesture.nearest(pointer))
            .flatten();
        painting::gesture(ui, scene, &gesture, pointer);
        if let Some(preview) = &self.preview {
            message_paint::ghost(
                ui,
                scene,
                preview,
                &gesture,
                pointer,
                self.document.source(),
                &self.graph.operator,
            );
        }
        let pending = self.notation_ui.sequence.pending_targets.is_some();
        ui.ctx()
            .set_cursor_icon(if gesture.hot.is_some() || pending {
                egui::CursorIcon::Grabbing
            } else {
                egui::CursorIcon::NotAllowed
            });
        if ui.input(|i| i.pointer.primary_released())
            || self.notation_ui.sequence.released.is_some()
        {
            if pending {
                self.notation_ui.sequence.released = Some(pointer);
                self.notation_ui.sequence.gesture = Some(gesture);
                return;
            }
            self.notation_ui.sequence.released = None;
            if let Some(index) = gesture.hot {
                let d = gesture.destinations.swap_remove(index);
                if d.alternatives.is_empty() {
                    self.sequence_apply(d.proposal);
                } else {
                    self.notation_ui.sequence.alternatives = Some(d.alternatives);
                }
            }
        } else if ui.input(|i| i.pointer.primary_down()) {
            self.notation_ui.sequence.gesture = Some(gesture);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn message_drops_require_proximity_not_a_distant_lifeline_fallback() {
        let hit = Hit::Origin(Point {
            dock: Dock {
                owner: "A".into(),
                before: 0,
                boundary: None,
            },
            at: egui::Pos2::ZERO,
            same_row: false,
            straighten: false,
        });
        let g = Gesture {
            hit,
            destinations: vec![Destination {
                at: egui::pos2(100.0, 100.0),
                proposal: Proposal {
                    edits: vec![],
                    line: None,
                    label: "Test",
                },
                alternatives: vec![],
                dock: None,
                hit_rect: None,
                preview: None,
            }],
            hot: None,
        };
        assert_eq!(g.nearest(egui::pos2(100.0, 110.0)), Some(0));
        assert_eq!(g.nearest(egui::pos2(100.0, 140.0)), None);
    }
    #[test]
    fn the_entire_activation_edge_is_a_target_not_only_its_corner() {
        let g = Gesture {
            hit: Hit::Origin(Point {
                dock: Dock {
                    owner: "A".into(),
                    before: 0,
                    boundary: None,
                },
                at: egui::Pos2::ZERO,
                same_row: false,
                straighten: false,
            }),
            destinations: vec![Destination {
                at: egui::pos2(100.0, 100.0),
                proposal: Proposal {
                    edits: vec![],
                    line: None,
                    label: "Test",
                },
                alternatives: vec![],
                dock: None,
                hit_rect: Some(egui::Rect::from_min_max(
                    egui::pos2(100.0, 100.0),
                    egui::pos2(300.0, 100.0),
                )),
                preview: None,
            }],
            hot: None,
        };
        assert_eq!(g.nearest(egui::pos2(200.0, 100.0)), Some(0));
        assert_eq!(g.nearest(egui::pos2(300.0, 108.0)), Some(0));
        assert_eq!(g.nearest(egui::pos2(200.0, 120.0)), None);
    }
}
