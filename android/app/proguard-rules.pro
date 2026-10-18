# UniFFI bindings reach the Rust library through JNA, which finds classes, fields and
# methods by reflection. Keep both intact under R8.
-keep class uniffi.kshana_ffi.** { *; }
-keep class com.sun.jna.** { *; }
-keep class * implements com.sun.jna.** { *; }
-keepclassmembers class * extends com.sun.jna.Structure { public *; }
-keepclassmembers class * implements com.sun.jna.Callback { *; }
-keepattributes Signature, InnerClasses, EnclosingMethod, *Annotation*
# JNA references desktop-only classes it never uses on Android.
-dontwarn java.awt.**
-dontwarn com.sun.jna.**

# Crash reports should name real classes and lines.
-keepattributes SourceFile, LineNumberTable
-renamesourcefileattribute SourceFile
