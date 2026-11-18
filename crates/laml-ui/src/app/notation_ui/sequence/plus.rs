//! Place insertion controls away from real rendered arrows and their endpoint grips.
use super::*;
fn box_at(at: egui::Pos2) -> egui::Rect {
    egui::Rect::from_center_size(at, egui::Vec2::splat(20.0))
}
fn crosses(rect: egui::Rect, a: egui::Pos2, b: egui::Pos2) -> bool {
    let mut near = 0.0_f32;
    let mut far = 1.0_f32;
    for (start, delta, lo, hi) in [
        (a.x, b.x - a.x, rect.left(), rect.right()),
        (a.y, b.y - a.y, rect.top(), rect.bottom()),
    ] {
        if delta.abs() < 0.0001 {
            if start < lo || start > hi {
                return false;
            }
        } else {
            let t0 = (lo - start) / delta;
            let t1 = (hi - start) / delta;
            near = near.max(t0.min(t1));
            far = far.min(t0.max(t1));
            if near > far {
                return false;
            }
        }
    }
    true
}
pub(super) fn arrange(scene: &mut Scene) {
    let fixed = scene
        .areas
        .iter()
        .filter(|a| !matches!(a.hit, Hit::Insert(_)))
        .map(|a| a.rect)
        .collect::<Vec<_>>();
    let paths = scene
        .regions
        .iter()
        .filter(|r| r.kind == "message")
        .map(|r| {
            r.path
                .iter()
                .map(|p| scene.point(p.0, p.1))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let mut placed = Vec::<egui::Rect>::new();
    for index in 0..scene.areas.len() {
        let Hit::Insert(point) = &scene.areas[index].hit else {
            continue;
        };
        let bar = scene.bar_for(point).or_else(|| {
            scene
                .bars
                .iter()
                .filter(|b| {
                    b.owner == point.dock.owner
                        && point.at.y >= b.rect.top()
                        && point.at.y <= b.rect.bottom()
                })
                .max_by(|a, b| a.region.x.total_cmp(&b.region.x))
        });
        let (left, right) = bar.map_or((point.at.x, point.at.x), |b| {
            (b.rect.left(), b.rect.right())
        });
        let xs = [right + 22.0, left - 22.0];
        let clear = |rect: egui::Rect| {
            !fixed.iter().any(|r| r.expand(1.0).intersects(rect))
                && !placed.iter().any(|r| r.expand(1.0).intersects(rect))
                && !scene.labels.iter().any(|r| r.expand(3.0).intersects(rect))
                && !paths.iter().any(|path| {
                    path.windows(2)
                        .any(|p| crosses(rect.expand(2.0), p[0], p[1]))
                })
        };
        // Prefer the existing row and the right side, then the free left side.
        let mut choice = xs
            .into_iter()
            .map(|x| box_at(egui::pos2(x, point.at.y)))
            .find(|r| clear(*r));
        if choice.is_none() {
            // Both sides can be occupied at a shared height. Search clear bands
            // from actual obstacle bounds, not an arbitrary fixed candidate cap.
            let mut ys = Vec::new();
            for r in fixed.iter().chain(&placed).chain(&scene.labels) {
                ys.extend([r.top() - 15.0, r.bottom() + 15.0]);
            }
            for path in &paths {
                for p in path {
                    ys.extend([p.y - 15.0, p.y + 15.0]);
                }
            }
            ys.sort_by(|a, b| {
                (a - point.at.y)
                    .abs()
                    .total_cmp(&(b - point.at.y).abs())
                    .then(a.total_cmp(b))
            });
            choice = ys
                .into_iter()
                .flat_map(|y| xs.map(|x| box_at(egui::pos2(x, y))))
                .find(|r| clear(*r));
        }
        if let Some(rect) = choice {
            scene.areas[index].rect = rect;
            placed.push(rect);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(senders: &[&str], zoom: f32) -> Scene {
        let mut source =
            "settings {\n type: sequence\n}\nsemantics {\n A\n B\n C\n work: B+\n".to_string();
        for (i, from) in senders.iter().enumerate() {
            source.push_str(&format!(" m{i}: {from} ->> B\n"));
        }
        source.push_str(" B-\n}\nlayout {\n");
        for i in 0..senders.len() {
            if senders.len() == 1 {
                continue;
            }
            for end in ["send", "receive"] {
                source.push_str(&format!(
                    " m{i}.{end} ? B.work+ : align=center-y; reference=second\n"
                ));
            }
        }
        source.push_str("}\n");
        let preview =
            laml_ui::preview::build(&source, std::path::Path::new("plus-layout.laml")).unwrap();
        let image = egui::Rect::from_min_size(
            egui::pos2(71.0, 93.0),
            egui::vec2(
                (preview.bounds.2 - preview.bounds.0) as f32 * zoom,
                (preview.bounds.3 - preview.bounds.1) as f32 * zoom,
            ),
        );
        Scene::new(&preview, image)
    }
    fn near_message(scene: &Scene) -> &scene::Area {
        let receive = scene
            .regions
            .iter()
            .find(|r| r.kind == "sequence-receive" && r.id == "B")
            .unwrap();
        let y = scene.point(receive.x, receive.y).y;
        scene
            .areas
            .iter()
            .filter(|a| matches!(&a.hit,Hit::Insert(p) if p.dock.owner=="B" && p.at.y>=y-0.01))
            .min_by(|a, b| {
                let at = |a: &scene::Area| match &a.hit {
                    Hit::Insert(p) => p.at.y,
                    _ => unreachable!(),
                };
                (at(a) - y).abs().total_cmp(&(at(b) - y).abs())
            })
            .expect("Insertion slot at/next to a message occurrence")
    }
    #[test]
    fn plus_uses_the_side_without_the_existing_arrow_grip() {
        for zoom in [0.5, 0.6] {
            for (sender, left) in [("C", true), ("A", false)] {
                let scene = fixture(&[sender], zoom);
                let plus = near_message(&scene);
                let Hit::Insert(point) = &plus.hit else {
                    unreachable!()
                };
                assert_eq!(
                    plus.rect.center().x < point.at.x,
                    left,
                    "Use the side opposite {sender}'s existing arrow"
                );
                assert!(
                    (plus.rect.center().y - point.at.y).abs() < 0.01,
                    "A free side must retain the event row"
                );
                assert!(matches!(
                    scene.hit(plus.rect.center()),
                    Some(Hit::Insert(_))
                ));
                for handle in scene
                    .areas
                    .iter()
                    .filter(|a| matches!(a.hit, Hit::Endpoint { .. }))
                {
                    assert!(!plus.rect.intersects(handle.rect));
                    assert!(
                        matches!(scene.hit(handle.rect.center()), Some(Hit::Endpoint { .. })),
                        "Arrow grip stays independently reachable"
                    );
                }
            }
        }
    }
    #[test]
    fn a_self_call_return_near_an_insertion_slot_moves_plus_to_the_left() {
        let scene = fixture(&["B"], 1.2);
        let plus = near_message(&scene);
        let Hit::Insert(point) = &plus.hit else {
            unreachable!()
        };
        assert!(plus.rect.center().x < point.at.x);
        assert!((plus.rect.center().y - point.at.y).abs() < 0.01);
        let receive = scene
            .regions
            .iter()
            .find(|r| r.kind == "sequence-receive" && r.id == "B")
            .unwrap();
        assert!((point.at.y - scene.point(receive.x, receive.y).y).abs() < 16.0);
        let old_plus = box_at(point.at + egui::vec2(22.0, 0.0));
        assert!(
            scene
                .areas
                .iter()
                .any(|a| matches!(a.hit, Hit::Endpoint { .. }) && old_plus.intersects(a.rect)),
            "Reproduce the actual prior overlap before testing the fix"
        );
        for a in scene
            .areas
            .iter()
            .filter(|a| matches!(a.hit, Hit::Endpoint { .. }))
        {
            assert!(!plus.rect.intersects(a.rect));
        }
    }
    #[test]
    fn both_arrow_sides_at_one_height_get_a_clear_offset_not_a_hidden_grip() {
        let scene = fixture(&["A", "C"], 0.2);
        let plus = near_message(&scene);
        let Hit::Insert(point) = &plus.hit else {
            unreachable!()
        };
        assert!((plus.rect.center().y - point.at.y).abs() >= 16.0);
        for a in scene
            .areas
            .iter()
            .filter(|a| !matches!(a.hit, Hit::Insert(_)))
        {
            assert!(!plus.rect.intersects(a.rect));
        }
        for r in scene.regions.iter().filter(|r| r.kind == "message") {
            for p in r.path.windows(2) {
                assert!(!crosses(
                    plus.rect,
                    scene.point(p[0].0, p[0].1),
                    scene.point(p[1].0, p[1].1)
                ));
            }
        }
        assert!(matches!(
            scene.hit(plus.rect.center()),
            Some(Hit::Insert(_))
        ));
    }
}
