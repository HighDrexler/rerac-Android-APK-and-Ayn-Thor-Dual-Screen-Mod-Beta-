//! Policy for the Android-exclusive one-tap cinematic skip.
//! Other platforms retain ReRAC's original Start+four-shoulders rule.
pub fn shoulder_chord(held:u32,pressed:u32,start:u32,shoulders:u32)->u32 {
    if pressed&start!=0 {held|shoulders} else {held}
}
#[cfg(test)]
mod tests {
    use super::*;
    const START:u32=8;
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
}
