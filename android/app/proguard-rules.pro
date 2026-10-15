# UniFFI bindings reach the Rust library through JNA; keep both intact under R8.
-keep class uniffi.kshana_ffi.** { *; }
-keep class com.sun.jna.** { *; }
-keep class * implements com.sun.jna.** { *; }
-dontwarn java.awt.**
