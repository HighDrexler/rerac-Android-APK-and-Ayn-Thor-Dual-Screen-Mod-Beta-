package org.rerac.android;

import static org.junit.Assert.*;
import java.io.File;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import org.junit.Rule;
import org.junit.Test;
import org.junit.rules.TemporaryFolder;

public class ResolutionFileTest {
    @Rule public TemporaryFolder temporary = new TemporaryFolder();
    private File settings() { return new File(temporary.getRoot(), "settings.toml"); }
    private void put(String text) throws IOException { Files.write(settings().toPath(), text.getBytes(StandardCharsets.UTF_8)); }
    private String get() throws IOException { return new String(Files.readAllBytes(settings().toPath()), StandardCharsets.UTF_8); }

    @Test public void newInstallUses720pAndSecondPrepareDoesNotWrite() throws Exception {
        assertTrue(ResolutionFile.defaultIfUnset(settings()));
        assertEquals("resolution = 720p\n", get());
        settings().setLastModified(100000);
        long timestamp = settings().lastModified();
        assertFalse(ResolutionFile.defaultIfUnset(settings()));
        assertEquals(timestamp, settings().lastModified());
        assertEquals("720p", ResolutionFile.effective(settings()));
    }

    @Test public void savedNativeChoiceIsNeverMigrated() throws Exception {
        String before = "# player settings\r\naspect = 16:9\r\nresolution = \"window\" # intentional\r\n";
        put(before);
        assertFalse(ResolutionFile.defaultIfUnset(settings()));
        assertEquals(before, get());
        assertEquals("window", ResolutionFile.effective(settings()));
    }

    @Test public void everyExplicitResolutionAndInvalidValueIsPreserved() throws Exception {
        for (String value : new String[]{"416", "720p", "1080", "1440p", "2160p", "", "future-option"}) {
            String before = "resolution = " + value + "\naspect = 4:3\n";
            put(before);
            assertFalse(value, ResolutionFile.defaultIfUnset(settings()));
            assertEquals(before, get());
        }
    }

    @Test public void absentResolutionAddsOnlyOneKeyAndPreservesOtherBytes() throws Exception {
        String before = "# resolution = window\r\nanti_aliasing = original\r\nunknown = \"keep=me\"";
        put(before);
        assertTrue(ResolutionFile.defaultIfUnset(settings()));
        assertEquals(before + "\nresolution = 720p\n", get());
    }

    @Test public void lastMatchingKeyWinsLikeNativeParser() {
        assertEquals("window", ResolutionFile.readKey("resolution=416\nresolution=\"window\" # last\n"));
        assertEquals("", ResolutionFile.readKey("resolution=720p\nresolution= # last empty\n"));
        assertNull(ResolutionFile.readKey("# resolution=2160\nother_resolution=1080\n"));
        assertEquals("720p", ResolutionFile.readKey("\u00a0resolution\u2003 = \"\"720p\"\"\u0085 # comment\n"));
    }

    @Test public void nativeResolutionSyntaxNormalizesWithoutBroadeningSupportedModes() {
        assertEquals("720p", ResolutionFile.normalize(" +0720p "));
        assertEquals("1080p", ResolutionFile.normalize("1080"));
        assertEquals("window", ResolutionFile.normalize("Window"));
        assertEquals("window", ResolutionFile.normalize("900p"));
        assertEquals("window", ResolutionFile.normalize("720P"));
        assertEquals("window", ResolutionFile.normalize("-720"));
        assertEquals("window", ResolutionFile.normalize("\u0667\u0662\u0660"));
    }

    @Test public void selectingOneResolutionPreservesAspectQualityAndUnknownKeys() throws Exception {
        put("# c\r\nresolution=720\r\naspect=16:10\r\nshadows=1\r\nunknown = keep\r\nresolution=1080p\r\n");
        ResolutionFile.select(settings(), "416p");
        assertEquals("# c\r\nresolution = 416p\r\naspect=16:10\r\nshadows=1\r\nunknown = keep\r\nresolution = 416p\r\n", get());
        assertEquals("416p", ResolutionFile.effective(settings()));
    }

    @Test public void failedAtomicReplacementLeavesPreviousFileAndCleansTemporary() throws Exception {
        put("resolution = window\naspect = 4:3\n");
        byte[] original = Files.readAllBytes(settings().toPath());
        try {
            ResolutionFile.select(settings(), "720p", new ResolutionFile.Replacer() {
                @Override public void replace(File staged, File destination) throws IOException {
                    assertEquals(destination.getAbsoluteFile().getParentFile(), staged.getParentFile());
                    assertTrue(staged.length() > 0);
                    throw new IOException("simulated atomic rename failure");
                }
            });
            fail("write should fail");
        } catch (IOException expected) { assertTrue(expected.getMessage().contains("simulated")); }
        assertArrayEquals(original, Files.readAllBytes(settings().toPath()));
        assertArrayEquals(new String[]{"settings.toml"}, temporary.getRoot().list());
    }

    @Test public void invalidUtf8SettingsAreNeverRewritten() throws Exception {
        byte[] invalid = {(byte) 0xff, (byte) 0xc0};
        Files.write(settings().toPath(), invalid);
        try { ResolutionFile.defaultIfUnset(settings()); fail("invalid UTF-8"); }
        catch (IOException expected) { assertTrue(expected.getMessage().contains("UTF-8")); }
        assertArrayEquals(invalid, Files.readAllBytes(settings().toPath()));
    }

    @Test public void settingsDirectoryCannotBeMistakenForMissingFile() throws Exception {
        assertTrue(settings().mkdir());
        try { ResolutionFile.defaultIfUnset(settings()); fail("directory is not settings"); }
        catch (IOException expected) { assertTrue(expected.getMessage().contains("not a file")); }
        assertTrue(settings().isDirectory());
    }

    @Test public void portOptionsChangesAreReadWithoutPreferenceCaching() throws Exception {
        ResolutionFile.defaultIfUnset(settings());
        put("resolution = 1080p\naspect=16:9\n");
        assertEquals("1080p", ResolutionFile.effective(settings()));
        assertFalse(ResolutionFile.defaultIfUnset(settings()));
        ResolutionFile.select(settings(), "window");
        assertEquals("window", ResolutionFile.effective(settings()));
        put("resolution = 416p\naspect=16:9\n");
        assertEquals("416p", ResolutionFile.effective(settings()));
    }

    @Test public void unsupportedSelectionDoesNotWriteSettings() throws Exception {
        put("resolution=window\n");
        try { ResolutionFile.select(settings(), "900p"); fail("unsupported mode"); }
        catch (IllegalArgumentException expected) { /* expected */ }
        assertEquals("resolution=window\n", get());
    }
}
