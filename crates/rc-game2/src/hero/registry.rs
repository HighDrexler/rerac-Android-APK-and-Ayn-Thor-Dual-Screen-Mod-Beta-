//! Going Commando's hero states: every id SetState (L00 `0x2c4560`) has a case for, the movement group its entry
//! writes to `0x18c0bc` (−1: the entry sets none, or sets it conditionally), the port module that owns it and
//! whether the port has it. A state without `ported` freezes the hero ([`super::HeroTick::Unported`]).
//!
//! The ids R&C1 also has keep R&C1's meaning where the code shows it (idle 0, walk 2, stop 3, crouch 4, fall 6, jump 7,
//! run jump 9, double jump 0xe; 0xb, R&C1's long jump, is R&C2's crouch flip); the rest are named by their group or left unnamed until ported.

/// The module that owns a state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Module {
    /// `ground.rs`: idle 0, stop 3, crouch 4.
    Ground,
    /// `walk.rs`: walk / run 2.
    Walk,
    /// `air.rs`: fall 6.
    Air,
    /// `jump.rs`: the jump group (group 4; ported: 7, 9, the flip 0xb, the double jump 0xe).
    Jump,
    /// Not ported yet.
    Unported,
}

/// One state of the machine.
#[derive(Clone, Copy, Debug)]
pub struct StateInfo {
    pub id: i32,
    /// The group SetState writes (−1: none or conditional).
    pub group: i32,
    pub module: Module,
    pub ported: bool,
}

const fn s(id: i32, group: i32, module: Module, ported: bool) -> StateInfo { StateInfo { id, group, module, ported } }
use Module::*;

/// Every state id with a SetState case on L00, in id order.
pub const STATES: &[StateInfo] = &[
    s(0x00, 0, Ground, true), s(0x01, 0, Unported, false), s(0x02, 1, Walk, true), s(0x03, 1, Ground, true),
    s(0x04, 0xc, Ground, true), s(0x06, 2, Air, true), s(0x07, 4, Jump, true), s(0x08, 5, Unported, false),
    s(0x09, 4, Jump, true), s(0x0a, 4, Unported, false), s(0x0b, 4, Jump, true), s(0x0c, 4, Unported, false),
    s(0x0d, 4, Unported, false), s(0x0e, 4, Jump, true), s(0x0f, 4, Unported, false), s(0x10, 4, Unported, false),
    s(0x11, 4, Unported, false), s(0x12, 4, Unported, false), s(0x13, -1, Unported, false), s(0x14, -1, Unported, false),
    s(0x15, -1, Unported, false), s(0x16, 7, Unported, false), s(0x17, 3, Unported, false), s(0x18, 3, Unported, false),
    s(0x19, 3, Unported, false), s(0x1a, 3, Unported, false), s(0x1b, 4, Unported, false), s(0x1c, 9, Unported, false),
    s(0x1d, 0, Unported, false), s(0x1e, 9, Unported, false), s(0x1f, -1, Unported, false), s(0x22, -1, Unported, false),
    s(0x23, 0xd, Unported, false), s(0x24, 0xd, Unported, false), s(0x25, 0xd, Unported, false), s(0x26, 8, Unported, false),
    s(0x27, 0xf, Unported, false), s(0x28, 0xf, Unported, false), s(0x29, 0xf, Unported, false), s(0x2a, 0xf, Unported, false),
    s(0x2b, 0xe, Unported, false), s(0x2c, 2, Unported, false), s(0x2e, 1, Unported, false), s(0x2f, 8, Unported, false),
    s(0x30, 0x10, Unported, false), s(0x31, 9, Unported, false), s(0x32, 0x11, Unported, false), s(0x33, 0x11, Unported, false),
    s(0x34, 0x11, Unported, false), s(0x35, 0x12, Unported, false), s(0x36, 0x12, Unported, false), s(0x37, 9, Unported, false),
    s(0x38, 4, Unported, false), s(0x39, 0x14, Unported, false), s(0x3d, -1, Unported, false), s(0x3e, 0xf, Unported, false),
    s(0x3f, 0, Unported, false), s(0x40, 1, Unported, false), s(0x41, 2, Unported, false), s(0x42, 7, Unported, false),
    s(0x43, 0x14, Unported, false), s(0x44, 2, Unported, false), s(0x45, 4, Unported, false), s(0x46, 3, Unported, false),
    s(0x47, 3, Unported, false), s(0x48, 4, Unported, false), s(0x49, 3, Unported, false), s(0x4a, 3, Unported, false),
    s(0x4b, 5, Unported, false), s(0x4d, -1, Unported, false), s(0x4e, 2, Unported, false), s(0x5b, 4, Unported, false),
    s(0x60, -1, Unported, false), s(0x61, 0x18, Unported, false), s(0x62, 1, Unported, false), s(0x63, 1, Unported, false),
    s(0x64, 0, Unported, false), s(0x65, 0x19, Unported, false), s(0x66, 0x19, Unported, false), s(0x67, 0x14, Unported, false),
    s(0x6d, 6, Unported, false), s(0x6e, 4, Unported, false), s(0x6f, 9, Unported, false), s(0x70, 1, Unported, false),
    s(0x71, 0x1a, Unported, false), s(0x72, -1, Unported, false), s(0x73, 7, Unported, false), s(0x74, 0x14, Unported, false),
    s(0x75, 9, Unported, false), s(0x76, 2, Unported, false), s(0x77, 10, Unported, false), s(0x78, 0x19, Unported, false),
    s(0x79, 0x14, Unported, false), s(0x7a, 0x14, Unported, false), s(0x7b, 0x1c, Unported, false), s(0x7c, 0x14, Unported, false),
    s(0x7d, 0x14, Unported, false), s(0x7f, 0x14, Unported, false), s(0x81, 4, Unported, false), s(0x82, 0x1e, Unported, false),
    s(0x83, -1, Unported, false), s(0x85, 0x20, Unported, false), s(0x86, 0x20, Unported, false), s(0x89, 4, Unported, false),
    s(0x8a, 0x21, Unported, false), s(0x8b, 0x21, Unported, false), s(0x8c, 7, Unported, false), s(0x92, 4, Unported, false),
    s(0x93, 6, Unported, false),
];

pub fn info(id: i32) -> Option<&'static StateInfo> { STATES.iter().find(|s| s.id == id) }
pub fn module_of(id: i32) -> Module { info(id).map_or(Unported, |s| s.module) }
pub fn implemented(id: i32) -> bool { info(id).is_some_and(|s| s.ported) }
