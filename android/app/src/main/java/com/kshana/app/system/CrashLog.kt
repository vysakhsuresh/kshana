package com.kshana.app.system

import android.content.Context
import android.content.Intent
import android.os.Build
import java.io.File
import java.io.PrintWriter
import java.io.StringWriter
import java.time.ZonedDateTime

/**
 * A crash, kept on this phone only. The last uncaught exception is written to
 * filesDir/crash/last.txt with the app version, the device and the Android version;
 * nothing is sent anywhere. The person can read it in Settings and choose to share it.
 */
object CrashLog {
    private fun file(context: Context): File = File(context.filesDir, "crash/last.txt")

    /** Install once, from the Application. Keeps the system's handler so the app still closes normally. */
    fun install(context: Context) {
        val app = context.applicationContext
        val previous = Thread.getDefaultUncaughtExceptionHandler()
        Thread.setDefaultUncaughtExceptionHandler { thread, error ->
            runCatching { write(app, thread, error) }
            previous?.uncaughtException(thread, error)
        }
    }

    private fun write(context: Context, thread: Thread, error: Throwable) {
        val trace = StringWriter().also { error.printStackTrace(PrintWriter(it)) }.toString()
        val version = runCatching {
            val p = context.packageManager.getPackageInfo(context.packageName, 0)
            "${p.versionName} (${if (Build.VERSION.SDK_INT >= 28) p.longVersionCode else @Suppress("DEPRECATION") p.versionCode.toLong()})"
        }.getOrDefault("unknown")
        val text = buildString {
            appendLine("Kshana crash report")
            appendLine("When: ${ZonedDateTime.now()}")
            appendLine("App: $version")
            appendLine("Device: ${Build.MANUFACTURER} ${Build.MODEL}")
            appendLine("Android: ${Build.VERSION.RELEASE} (API ${Build.VERSION.SDK_INT})")
            appendLine("Thread: ${thread.name}")
            appendLine()
            append(trace)
        }
        val f = file(context)
        f.parentFile?.mkdirs()
        f.writeText(text)
    }

    /** The last crash, or null when there is none. */
    fun last(context: Context): String? = file(context).takeIf { it.exists() }?.readText()

    /** Forget the last crash. */
    fun clear(context: Context) {
        file(context).delete()
    }

    /** A plain-text share of the last crash, for the person to send if they choose; null when there is none. */
    fun shareIntent(context: Context): Intent? {
        val text = last(context) ?: return null
        val send = Intent(Intent.ACTION_SEND)
            .setType("text/plain")
            .putExtra(Intent.EXTRA_SUBJECT, "Kshana crash report")
            .putExtra(Intent.EXTRA_TEXT, text)
        return Intent.createChooser(send, "Share the crash report")
    }
}
