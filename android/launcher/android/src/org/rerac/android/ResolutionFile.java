package org.rerac.android;

import java.io.File;
import java.io.FileOutputStream;
import java.io.IOException;
import java.nio.ByteBuffer;
import java.nio.charset.CharacterCodingException;
import java.nio.charset.CodingErrorAction;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.StandardCopyOption;

/** Engine-compatible key/value persistence, without a second preference store. */
final class ResolutionFile {
    private static final int MAX_SETTINGS_BYTES = 1024 * 1024;
    private static final String KEY = "resolution";

    interface Replacer { void replace(File temporary, File destination) throws IOException; }
    private static final Replacer ATOMIC_REPLACER = new Replacer() {
        @Override public void replace(File temporary, File destination) throws IOException {
            Files.move(temporary.toPath(), destination.toPath(),
                    StandardCopyOption.ATOMIC_MOVE, StandardCopyOption.REPLACE_EXISTING);
        }
    };

    private ResolutionFile() {}

    static synchronized boolean defaultIfUnset(File file) throws IOException {
        String text = read(file);
        // Even an unusual or invalid explicit value is left intact. In particular,
        // a saved "window" may have been chosen by the player; never migrate it.
        if (readKey(text) != null) return false;
        write(file, append(text, "720p"), ATOMIC_REPLACER);
        return true;
    }

    static synchronized String effective(File file) throws IOException {
        return normalize(readKey(read(file)));
    }

    static synchronized void select(File file, String value) throws IOException {
        select(file, value, ATOMIC_REPLACER);
    }

    static synchronized void select(File file, String value, Replacer replacer) throws IOException {
        if (value == null || !normalize(value).equals(value))
            throw new IllegalArgumentException("Unsupported rendering resolution");
        String before = read(file);
        String after = replaceKey(before, value);
        if (!after.equals(before)) write(file, after, replacer);
    }

    private static String read(File file) throws IOException {
        if (!file.exists()) return "";
        if (!file.isFile()) throw new IOException("Settings path is not a file");
        if (file.length() > MAX_SETTINGS_BYTES) throw new IOException("Settings file is too large");
        byte[] bytes = Files.readAllBytes(file.toPath());
        if (bytes.length > MAX_SETTINGS_BYTES) throw new IOException("Settings file is too large");
        try {
            // Rust read_to_string rejects invalid UTF-8. Do not silently replace
            // malformed bytes and rewrite unrelated settings after a failed read.
            return StandardCharsets.UTF_8.newDecoder()
                    .onMalformedInput(CodingErrorAction.REPORT)
                    .onUnmappableCharacter(CodingErrorAction.REPORT)
                    .decode(ByteBuffer.wrap(bytes)).toString();
        } catch (CharacterCodingException failure) {
            throw new IOException("Settings file is not valid UTF-8", failure);
        }
    }

    /** Mirrors rc-engine/render_settings.rs: last matching key wins. */
    static String readKey(String text) {
        String result = null;
        int begin = 0;
        while (begin < text.length()) {
            int end = text.indexOf('\n', begin);
            if (end < 0) end = text.length();
            String line = text.substring(begin, end);
            int equals = line.indexOf('=');
            if (equals >= 0 && trim(line.substring(0, equals)).equals(KEY)) {
                String value = line.substring(equals + 1);
                int comment = value.indexOf('#');
                if (comment >= 0) value = value.substring(0, comment);
                value = trim(value);
                int first = 0, last = value.length();
                while (first < last && value.charAt(first) == '"') first++;
                while (last > first && value.charAt(last - 1) == '"') last--;
                result = value.substring(first, last);
            }
            begin = end + 1;
        }
        return result;
    }

    /** Mirrors display.rs Resolution::parse; an unsupported value falls back to Window. */
    static String normalize(String value) {
        if (value == null) return "window";
        value = trim(value);
        if (value.equals("window")) return "window";
        if (value.endsWith("p")) value = value.substring(0, value.length() - 1);
        // Rust u16 accepts a leading plus, but only ASCII digits.
        int start = value.startsWith("+") ? 1 : 0;
        if (start == value.length()) return "window";
        for (int i = start; i < value.length(); i++) {
            char c = value.charAt(i);
            if (c < '0' || c > '9') return "window";
        }
        final int height;
        try { height = Integer.parseInt(value); }
        catch (NumberFormatException unsupported) { return "window"; }
        switch (height) {
            case 416: case 720: case 1080: case 1440: case 2160: return height + "p";
            default: return "window";
        }
    }

    static String replaceKey(String text, String value) {
        StringBuilder out = new StringBuilder(text.length() + 32);
        boolean found = false;
        int begin = 0;
        while (begin < text.length()) {
            int newline = text.indexOf('\n', begin);
            int end = newline < 0 ? text.length() : newline;
            int contentEnd = end > begin && text.charAt(end - 1) == '\r' ? end - 1 : end;
            String line = text.substring(begin, contentEnd);
            int equals = line.indexOf('=');
            if (equals >= 0 && trim(line.substring(0, equals)).equals(KEY)) {
                out.append(KEY).append(" = ").append(value);
                found = true;
            } else out.append(line);
            out.append(text, contentEnd, end);
            if (newline >= 0) out.append('\n');
            begin = end + 1;
        }
        return found ? out.toString() : append(text, value);
    }

    private static String append(String text, String value) {
        return text + (text.isEmpty() || text.endsWith("\n") ? "" : "\n")
                + KEY + " = " + value + "\n";
    }

    private static void write(File file, String text, Replacer replacer) throws IOException {
        File parent = file.getAbsoluteFile().getParentFile();
        if (parent == null || (!parent.isDirectory() && !parent.mkdirs()))
            throw new IOException("Settings directory is unavailable");
        File temporary = File.createTempFile(".resolution-", ".tmp", parent);
        try {
            try (FileOutputStream output = new FileOutputStream(temporary)) {
                output.write(text.getBytes(StandardCharsets.UTF_8));
                output.getFD().sync();
            }
            // Same-directory atomic rename: failure leaves the previous file intact.
            replacer.replace(temporary, file);
        } finally {
            if (temporary.exists()) temporary.delete();
        }
    }

    private static String trim(String value) {
        int first = 0, last = value.length();
        while (first < last && whitespace(value.charAt(first))) first++;
        while (last > first && whitespace(value.charAt(last - 1))) last--;
        return value.substring(first, last);
    }

    // Unicode White_Space, the definition used by Rust str::trim.
    private static boolean whitespace(char c) {
        return (c >= '\t' && c <= '\r') || c == ' ' || c == '\u0085' || c == '\u00a0'
                || c == '\u1680' || (c >= '\u2000' && c <= '\u200a') || c == '\u2028'
                || c == '\u2029' || c == '\u202f' || c == '\u205f' || c == '\u3000';
    }
}
