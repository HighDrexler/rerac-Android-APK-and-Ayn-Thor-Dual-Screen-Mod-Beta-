package org.rerac.android;

/** Native metadata inspection. Does not load the game engine during launcher startup. */
final class NativeCore {
    private NativeCore() {}
    static native String buildInfo();
    static native boolean menuChoice(long generation,long page,long widget,int index);
    static native String companionInputStatus();
}
