//! Companion projection of live PageMenu records. Kept free of JNI/Bevy so the
//! very same selection adapter can be tested against PageMenu::tick on a host.
use rc_game::menus::{MenuAssets, pause::{Data, PageMenu, Widget, func}};

#[derive(Debug, PartialEq, Eq)]
pub struct Row {
    pub label:String, pub detail:String, pub enabled:bool, pub selected:bool,
    pub widget:u32, pub index:usize,
}
#[derive(Debug)]
pub struct Screen {
    pub title:String, pub root:bool, pub rows:Vec<Row>, pub descriptions:Vec<String>,
    pub supported:bool, pub can_resume:bool, pub busy:bool,
}
fn text(raw:&[u8])->String {
    // 0x01 is the game's line break; 0x08..0x19 are formatting/pad icons.
    if raw==b"Paradox! This message does not exist" {return String::new()}
    raw.iter().copied().take_while(|&c|c!=0).filter_map(|c|match c {
        1|10=>Some('\n'),32..=255=>Some(char::from(c)),_=>None,
    }).collect()
}
fn visible(menu:&PageMenu,slot:usize,w:&Widget)->bool {
    // PageMenu::draw skips frame slot 6 when Goodies is locked, on every page.
    w.dflags&4==0&&w.draw!=0&&(slot!=6||menu.goodies)
}
fn selectable(w:&Widget)->bool { w.update==func::LIST_UPDATE&&matches!(w.data,Data::List(_)) }

pub fn choose(menu:&mut PageMenu,page:u32,widget:u32,index:usize)->bool {
    if menu.current!=page||menu.trans!=0||menu.target!=0{return false}
    let Some(p)=menu.pages.get(&page) else{return false};
    let Some(slot)=p.widgets.iter().position(|&id|id==widget) else{return false};
    let Some(w)=menu.widgets.get(&widget) else{return false};
    if !visible(menu,slot,w)||!selectable(w){return false}
    let Data::List(list)=&w.data else{return false};
    if list.items.get(index).is_none_or(|i|i.label==0||i.action<=0||i.action==2){return false}
    let Data::List(list)=&mut menu.widgets.get_mut(&widget).unwrap().data else{unreachable!()};
    list.cursor=index as i32;
    let p=menu.pages.get_mut(&page).unwrap();p.focus=widget;p.pending=0;
    true
}

pub fn screen(menu:&PageMenu,a:&MenuAssets)->Option<Screen> {
    let page=menu.pages.get(&menu.current)?;
    let swapped=menu.text_swap==2;
    let mut s=Screen{title:if menu.current==menu.addrs.root{"Paused"}else{"Game menu"}.into(),root:menu.current==menu.addrs.root,rows:Vec::new(),descriptions:Vec::new(),supported:true,can_resume:!menu.no_close,busy:menu.trans!=0||menu.target!=0};
    for w in menu.widgets.values().filter(|w|selectable(w)) {
        let Data::List(l)=&w.data else{continue};
        for it in &l.items {if matches!(it.action,3|4|5)&&it.arg==menu.current {
            let label=text(a.msg_in(it.label as i32,swapped));if !label.is_empty(){s.title=label;}
        }}
    }
    for (slot,id) in page.widgets.iter().copied().enumerate() {
        let Some(w)=menu.widgets.get(&id) else{continue};
        if !visible(menu,slot,w){continue}
        match &w.data {
            Data::List(l) if selectable(w)=>{
                for (index,it) in l.items.iter().enumerate() {
                    // The native renderer substitutes "Unavailable" for action 2.
                    let label=text(a.msg_in(if it.action==2{20308}else{it.label as i32},swapped));
                    if label.is_empty(){s.supported=false;continue}
                    let detail=if it.sublabel==0{String::new()}else{text(a.msg_in(it.sublabel as i32,swapped))};
                    s.rows.push(Row{label,detail,enabled:it.action>0&&it.action!=2,selected:page.focus==id&&l.cursor==index as i32,widget:id,index});
                }
            },
            // A list-drawn hint panel has no update callback and no choices.
            // Confirm/sketch widgets also use Data::List, but their input is
            // not LIST_UPDATE: retain their native renderer and key behavior.
            Data::List(_) if w.update==0=>{},
            Data::Label(l)=>{
                if (l.flags&4!=0&&l.id==0)||(l.flags&0x1000!=0&&l.id==0xffff)||(l.flags&0x100!=0&&l.content<0){continue}
                // Complex ownership/skill/planet formatting stays in the
                // original label renderer, rather than inventing a second UI.
                if l.flags&0x200!=0||l.flags&0x800!=0||l.flags&0x11e4==0{s.supported=false;continue}
                let msg=if l.flags&(4|0x1000)!=0 {l.id} else {
                    let addr=l.id.wrapping_add((l.content as u32).wrapping_mul(l.stride)&!3).wrapping_add(l.variant as u32*4);
                    match (&l.table,menu.label_tables.get(&l.id)) {
                        (Some(t),_) if l.content>=0=>t.get(l.content as usize).copied().unwrap_or(0),
                        (None,Some(t)) if l.content>=0=>t.get(l.content as usize).copied().unwrap_or(0),
                        _=>a.overlay.u32(addr).unwrap_or(0),
                    }
                };
                let label=text(a.msg_in(msg as i32,swapped));
                if !label.is_empty()&&label!=s.title&&!s.descriptions.contains(&label){s.descriptions.push(label);}
            },
            Data::Other if w.update==0=>{},
            _=>s.supported=false,
        }
    }
    s.supported&=!s.rows.is_empty();Some(s)
}
