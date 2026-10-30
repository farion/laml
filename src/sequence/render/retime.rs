//! Expand downstream rows when a mandatory coregion gap needs more height.

use super::{
    DrawnFrame,
    anchors::MessageAnchor,
    executions::{DrawnExecution, OpenExecution},
};
use crate::sequence::{AnnotationEnd, Continuation, Coregion, Note};

pub(super) struct Shiftables<'data, 'borrow> {
    pub anchors: &'borrow mut [MessageAnchor],
    pub frames: &'borrow mut [DrawnFrame],
    pub bars: &'borrow mut [DrawnExecution],
    pub open: &'borrow mut [OpenExecution],
    pub positions: &'borrow mut [(usize, f64)],
    pub deaths: &'borrow mut [(String, f64)],
    pub branches: &'borrow mut [f64],
    pub decompositions: &'borrow mut [(usize, f64)],
    pub notes: &'borrow mut [(&'data Note, f64)],
    pub continuations: &'borrow mut [(&'data Continuation, f64)],
    pub references: &'borrow mut [(&'data str, f64)],
    pub final_y: &'borrow mut f64,
}

fn shift(y: &mut f64, threshold: f64, delta: f64) {
    if *y > threshold + 0.001 {
        *y += delta;
    }
}

impl Shiftables<'_, '_> {
    pub(super) fn expand(&mut self, region: &Coregion, threshold: f64, delta: f64) {
        if delta < 0.001 {
            return;
        }
        for anchor in self.anchors.iter_mut() {
            if !region
                .ends
                .iter()
                .any(|end| end.message == anchor.id && end.end == AnnotationEnd::Send)
            {
                shift(&mut anchor.send.1, threshold, delta);
            }
            if !region
                .ends
                .iter()
                .any(|end| end.message == anchor.id && end.end == AnnotationEnd::Receive)
            {
                shift(&mut anchor.receive.1, threshold, delta);
            }
        }
        for frame in self.frames.iter_mut() {
            shift(&mut frame.top, threshold, delta);
            shift(&mut frame.bottom, threshold, delta);
            for (_, y) in &mut frame.separators {
                shift(y, threshold, delta);
            }
        }
        for bar in self.bars.iter_mut() {
            shift(&mut bar.opened.from, threshold, delta);
            shift(&mut bar.to, threshold, delta);
        }
        for bar in self.open.iter_mut() {
            shift(&mut bar.from, threshold, delta);
        }
        for (_, y) in self.positions.iter_mut() {
            shift(y, threshold, delta);
        }
        for (_, y) in self.deaths.iter_mut() {
            shift(y, threshold, delta);
        }
        for y in self.branches.iter_mut() {
            shift(y, threshold, delta);
        }
        for (_, y) in self.decompositions.iter_mut() {
            shift(y, threshold, delta);
        }
        for (_, y) in self.notes.iter_mut() {
            shift(y, threshold, delta);
        }
        for (_, y) in self.continuations.iter_mut() {
            shift(y, threshold, delta);
        }
        for (_, y) in self.references.iter_mut() {
            shift(y, threshold, delta);
        }
        shift(self.final_y, threshold, delta);
    }
}
