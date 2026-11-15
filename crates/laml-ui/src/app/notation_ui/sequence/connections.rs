//! Reconnect either message endpoint and expose actual lifeline/execution drop docks.
use super::*;
use super::{canvas::point, targets::{Baseline, Target}};
use laml::{authoring::Region, sequence::{Event, Message, MessageKind}};

fn near_path(path: &[(f64,f64)], x: f64,y: f64,tolerance: f64) -> bool {
    path.windows(2).any(|p| {
        let (dx,dy)=(p[1].0-p[0].0,p[1].1-p[0].1);
        let len=dx*dx+dy*dy;
        let t=if len<1e-9 { 0.0 } else { ((x-p[0].0)*dx+(y-p[0].1)*dy)/len }.clamp(0.0,1.0);
        (x-p[0].0-t*dx).hypot(y-p[0].1-t*dy)<=tolerance
    })
}
fn local(rect: egui::Rect,bounds:(f64,f64,f64,f64),p:egui::Pos2)->(f64,f64) {
    let scale=f64::from(rect.width())/(bounds.2-bounds.0);
    (bounds.0+f64::from(p.x-rect.left())/scale,bounds.1+f64::from(p.y-rect.top())/scale)
}
fn positions(regions: &[Region], fixed_owner: &str, fixed:(f64,f64), hot:&Target, moving_source:bool, original_self:Option<bool>)->((f64,f64),(f64,f64)) {
    let self_message=hot.owner==fixed_owner;
    let preserve=hot.before.is_none() && original_self.is_none_or(|old|old==self_message);
    let fixed_y=if preserve { fixed.1 } else { hot.row_y+if self_message && moving_source {24.0} else {0.0} };
    let fixed_x=targets::dock(regions,fixed_owner,fixed_y,hot.x,self_message).map_or(fixed.0,|d| d.0);
    if moving_source { ((hot.x,hot.y),(fixed_x,fixed_y)) } else { ((fixed_x,fixed_y),(hot.x,hot.y)) }
}
impl Editor {
    pub(super) fn sequence_connections(&mut self,ui:&mut egui::Ui,rect:egui::Rect)->bool {
        if let Some((anchor,action))=self.notation_ui.sequence.connect.clone() {
            self.sequence_new_message(ui,rect,anchor,action); return true;
        }
        if let Some(gesture)=self.notation_ui.sequence.reconnect.clone() {
            self.sequence_reconnect(ui,rect,gesture); return true;
        }
        let Some(preview)=&self.preview else {return false;};
        let regions=&preview.regions;
        let scale=f64::from(rect.width())/(preview.bounds.2-preview.bounds.0);
        let pointer=ui.input(|i| i.pointer.hover_pos()).filter(|p|ui.clip_rect().contains(*p)).map(|p|local(rect,preview.bounds,p));
        let Ok(doc)=laml::sequence::parse(self.document.source()) else {return false;};
        let mut owned=false;
        let mut select=None;
        let mut reconnect=None;
        for message in doc.events.iter().filter_map(|e|if let Event::Message(m)=e {Some(m)} else {None}) {
            let Some(path)=regions.iter().find(|r|r.kind=="message" && r.line==message.line && !r.path.is_empty()) else {continue;};
            let selected=self.notation_ui.selected.as_ref().is_some_and(|s|s.0==message.line && s.2==self.document.revision());
            let hovered=pointer.is_some_and(|(x,y)|near_path(&path.path,x,y,10.0/scale));
            if !selected && !hovered {continue;}
            let send=path.path[0];
            let receive=*path.path.last().unwrap();
            for (moving_source,at) in [(true,send),(false,receive)] {
                let center=point(rect,preview.bounds,at.0,at.1);
                let response=ui.interact(egui::Rect::from_center_size(center,egui::Vec2::splat(22.0)),ui.scope_id().with(("sequence-end",self.id,message.line,moving_source)),egui::Sense::click_and_drag());
                let hot=response.hovered() || response.is_pointer_button_down_on();
                let color=appearance::color(ui.ctx(),appearance::ACCENT);
                ui.painter().circle_filled(center,5.5,appearance::color(ui.ctx(),appearance::PANEL));
                ui.painter().circle_stroke(center,5.5,egui::Stroke::new(if hot {2.0}else{1.5},color));
                if hot { ui.ctx().set_cursor_icon(egui::CursorIcon::Grab); owned=true; }
                if response.is_pointer_button_down_on() || response.clicked() {select=Some((message.line,path.id.clone()));}
                if response.drag_started_by(egui::PointerButton::Primary) {
                    reconnect=Some(Reconnect{line:message.line,moving_source,message:message.clone(),send,receive});
                    select=Some((message.line,path.id.clone())); owned=true;
                }
            }
        }
        if let Some((line,id))=select {self.notation_select(line,Some(id));}
        if let Some(reconnect)=reconnect {
            self.notation_ui.sequence.reconnect=Some(reconnect.clone());self.notation_ui.drag=None;
            self.sequence_reconnect(ui,rect,reconnect);ui.ctx().request_repaint();
        }
        owned
    }
    fn sequence_reconnect(&mut self,ui:&egui::Ui,rect:egui::Rect,g:Reconnect) {
        self.notation_ui.drag=None;
        let Some(preview)=&self.preview else {return;};
        let Some(pointer)=ui.input(|i|i.pointer.interact_pos()) else {return;};
        let fixed_owner=if g.moving_source {&g.message.to}else{&g.message.from};
        let fixed=if g.moving_source {g.receive}else{g.send};
        let moving=if g.moving_source {g.send}else{g.receive};
        let moving_owner=if g.moving_source {&g.message.from}else{&g.message.to};
        let docks=targets::all(&preview.regions,fixed_owner,fixed.0,g.moving_source,Baseline{before:None,send_y:g.send.1,moving_y:moving.1,moving_owner:Some(moving_owner)});
        let (x,y)=local(rect,preview.bounds,pointer);
        let tolerance=14.0/(f64::from(rect.width())/(preview.bounds.2-preview.bounds.0));
        let hot=ui.clip_rect().contains(pointer).then(||targets::hit(&docks,&preview.regions,x,y,tolerance)).flatten();
        let candidate=hot.map(|t|edit::sequence_reconnect::reconnect(self.document.source(),g.line,g.moving_source,&t.owner,t.before,t.activation_line));
        let valid=candidate.as_ref().is_some_and(|r|r.is_ok());
        paint::drops(ui,preview,rect,&docks,hot,valid);
        let (send,receive)=hot.map_or_else(||if g.moving_source {((x,y),fixed)}else{(fixed,(x,y))},|t|positions(&preview.regions,fixed_owner,fixed,t,g.moving_source,Some(g.message.from==g.message.to)));
        let self_message=hot.map_or(g.message.from==g.message.to,|t|t.owner==*fixed_owner);
        let mut moving_message=g.message.clone();
        if let Some(target)=hot {if g.moving_source {moving_message.from=target.owner.clone();}else{moving_message.to=target.owner.clone();}}
        paint::message(ui,preview,rect,&moving_message,send,receive,self_message,laml::sequence::parse(self.document.source()).is_ok_and(|d|d.allow_diagonal));
        ui.ctx().set_cursor_icon(if hot.is_some() && !valid {egui::CursorIcon::NotAllowed}else{egui::CursorIcon::Grabbing});
        if ui.input(|i|i.pointer.button_released(egui::PointerButton::Primary)) {
            self.notation_ui.sequence.reconnect=None;
            match candidate {
                Some(Ok((edits,line))) if !edits.is_empty()=>{
                    let id=g.message.name.clone().unwrap_or_else(||format!("line:{line}"));
                    self.submit_command(Command{edits,label:"Reconnect sequence message".into(),expectation:worker::Expectation::Notation{line:Some(line),id:Some(id)}});
                }
                Some(Err(error))=>self.command_error=Some(error), _=>{}
            }
        }
    }
    fn sequence_new_message(&mut self,ui:&egui::Ui,rect:egui::Rect,anchor:Anchor,action:Action) {
        self.notation_ui.drag=None;
        let Some(preview)=&self.preview else {return;};
        let Some(pointer)=ui.input(|i|i.pointer.hover_pos()) else {return;};
        let (x,y)=local(rect,preview.bounds,pointer);
        let docks=targets::all(&preview.regions,&anchor.owner,anchor.x,false,Baseline{before:Some(anchor.before),send_y:anchor.y,moving_y:anchor.y,moving_owner:None});
        let tolerance=14.0/(f64::from(rect.width())/(preview.bounds.2-preview.bounds.0));
        let hot=ui.clip_rect().contains(pointer).then(||targets::hit(&docks,&preview.regions,x,y,tolerance)).flatten();
        let mut insertion=anchor.clone();
        if let Some(target)=hot {insertion.before=target.before.unwrap_or(anchor.before);insertion.activation_line=target.activation_line;}
        let candidate=hot.map(|t|self.sequence_candidate(&insertion,action,&t.owner,"","Message"));
        let valid=candidate.as_ref().is_some_and(|r|r.is_ok());
        paint::drops(ui,preview,rect,&docks,hot,valid);
        let self_message=hot.is_some_and(|t|t.owner==anchor.owner);
        let (send,mut receive)=hot.map_or_else(||{
            let sx=targets::dock(&preview.regions,&anchor.owner,y,x,false).map_or(anchor.x,|d|d.0);
            ((sx,y),(x,y))
        },|t|positions(&preview.regions,&anchor.owner,(anchor.x,anchor.y),t,false,None));
        if valid && action==Action::SyncActivate && insertion.activation_line.is_none() && let Some(target)=hot {
            receive=paint::starting_bar(ui,preview,rect,self.document.source(),target,send.0,self_message,receive.1);
        }
        let message=Message{name:None,classes:vec![],from:anchor.owner.clone(),to:hot.map_or_else(||"Target".into(),|t|t.owner.clone()),label:None,
            kind:match action {Action::Async=>MessageKind::Async,Action::Reply=>MessageKind::Reply,_=>MessageKind::Sync},filled_reply:false,line:0};
        paint::message(ui,preview,rect,&message,send,receive,self_message,laml::sequence::parse(self.document.source()).is_ok_and(|d|d.allow_diagonal));
        ui.ctx().set_cursor_icon(if hot.is_some()&&!valid {egui::CursorIcon::NotAllowed}else{egui::CursorIcon::Crosshair});
        if ui.input(|i|i.pointer.button_released(egui::PointerButton::Primary)) && let Some(target)=hot {
            let owner=target.owner.clone();
            self.notation_ui.sequence.connect=None;
            match candidate {
                Some(Ok(_))=>self.sequence_form(insertion,action,owner),
                Some(Err(error))=>self.command_error=Some(error), _=>{}
            }
        }
    }
}
