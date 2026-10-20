#!/usr/bin/env python3
"""Assert the 0.3.6 native/Java dual-screen contract survives touch repair."""
import json
import re
import unittest
from pathlib import Path

ROOT=Path(__file__).resolve().parents[3]

class CompanionParity(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.adapter=(ROOT/"crates/rc-engine/src/android.rs").read_text()
        cls.companion=(ROOT/"crates/rc-engine/src/android/companion.rs").read_text()
        cls.policy=(ROOT/"crates/rc-engine/src/android/companion_policy.rs").read_text()
        cls.launcher=(ROOT/"android/launcher/android/src/org/rerac/android/ModStore.java").read_text()
        cls.packager=(ROOT/"android/native/package_core.py").read_text()
        cls.manifest=json.loads((ROOT/"android/launcher/android/assets/mods/dual-screen/mod.json").read_text())

    def test_companion_jni_v16_exports_still_available(self):
        prefix="Java_org_rerac_android_GameActivity_"
        required=(
            "nativeInput","nativeTouchInput","nativeTouchMode","nativeMenuMode",
            "nativeSetDual","nativeDualEnabled","nativeSecondary","nativeWheel",
            "nativeCompanion","nativeIcon","nativeSelect","nativeMenuButton",
            "nativeMenuTouch","nativeMenuGesture","nativeMapVisible","nativeMapView",
            "nativePlanetIcon","nativePause","nativeDisplayRate"
        )
        for method in required:
            with self.subTest(method=method):
                self.assertEqual(len(re.findall(r"fn\s+"+prefix+method+r"\b",self.adapter)),1)

    def test_dual_screen_preference_and_second_display_survive(self):
        for token in ("dual-screen.setting","AtomicFile","secondDisplay(c)",
                      "setDual(c,value)","dualEnabled(c)"):
            with self.subTest(token=token):
                self.assertIn(token,self.launcher)
        self.assertIn("static SECONDARY",self.adapter)
        self.assertIn("fn secondary()",self.adapter)

    def test_dual_screen_mod_and_native_core_upgrade_contract(self):
        self.assertEqual(self.manifest["id"],"dual-screen")
        self.assertEqual(self.manifest["kind"],"built-in")
        self.assertEqual(self.manifest["schema"],1)
        self.assertEqual(self.manifest["name"],"AYN Thor Dual-Screen Mod")
        self.assertIn("mod['core_versions']=[core]",self.packager)
        self.assertIn("mod['version']='1.1'",self.packager)
        self.assertIn("replacements['classes2.dex']=patch_touch_wheel_triangle",self.packager)

    def test_companion_during_gameplay_dialogue_cinematics_and_native_menus(self):
        for token in (
            'display_mode(mode.raw(),frontend)',
            'TOUCH_MODE.store(touch_mode',
            'Command::Select','Command::Touch','Command::Gesture',
            'native_frontend_primitive',
            'skills','goldFound','mapHero','quick_select','map.composed'
        ):
            with self.subTest(token=token):
                self.assertTrue(token in self.companion or token in self.policy,
                                f"Missing companion feature {token}")

    def test_native_android_touches_do_not_replace_companion_snapshots(self):
        self.assertIn("PostUpdate, companion::publish",self.adapter)
        self.assertIn("FixedUpdate, (companion::apply_commands, sample_tick)",self.adapter)
        self.assertIn("nativeSelect",self.adapter)
        self.assertIn("nativeMenuTouch",self.adapter)
        self.assertIn("nativePlanetIcon",self.adapter)

if __name__=="__main__":
    unittest.main()
