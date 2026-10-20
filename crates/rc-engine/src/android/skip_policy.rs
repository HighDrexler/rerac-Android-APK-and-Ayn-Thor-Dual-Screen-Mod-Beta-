//! Policy for the Android-exclusive one-tap cinematic skip.
//! Other platforms retain ReRAC's original Start+four-shoulders rule.
pub fn shoulder_chord(held:u32,pressed:u32,start:u32,shoulders:u32)->u32 {
    if pressed&start!=0 {held|shoulders} else {held}
}
#[cfg(test)]
mod tests {
    use super::*;
    // rc_game::pad::button's native SCE layout; 0x8 is R1, not Start.
    const START:u32=0x800;
    const SHOULDER:u32=15;
    #[test] fn android_start_can_skip_movie_and_scene(){
        assert_eq!(shoulder_chord(0,START,START,SHOULDER),15);
        assert_eq!(shoulder_chord(5,START,START,SHOULDER),15);
    }
    #[test] fn existing_chord_and_unrelated_buttons_unchanged(){
        assert_eq!(shoulder_chord(15,0,START,SHOULDER),15);
        assert_eq!(shoulder_chord(3,0,START,SHOULDER),3);
        assert_eq!(shoulder_chord(0,4,START,SHOULDER),0);
    }

    #[test]
    fn each_shoulder_and_face_button_alone_cannot_request_skip() {
        for pressed in [1, 2, 4, 8, 0x10, 0x20, 0x40, 0x80, 0x100, 0x200, 0x400] {
            assert_eq!(shoulder_chord(pressed, pressed, START, SHOULDER), pressed);
        }
    }

    #[test]
    fn start_edge_keeps_start_and_other_buttons_held() {
        let held = START | 0x40 | 0x2000;
        assert_eq!(
            shoulder_chord(held, START | 0x40, START, SHOULDER),
            held | SHOULDER,
        );
        // A held Start on the next tick must not synthesize another chord.
        assert_eq!(shoulder_chord(held, 0, START, SHOULDER), held);
    }

    #[test]
    fn releasing_start_does_not_latch_the_synthetic_shoulders() {
        assert_eq!(shoulder_chord(START, START, START, SHOULDER), START | SHOULDER);
        assert_eq!(shoulder_chord(0, 0, START, SHOULDER), 0);
        assert_eq!(shoulder_chord(8, 8, START, SHOULDER), 8);
    }
}
