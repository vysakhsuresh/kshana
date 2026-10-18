package com.kshana.app.data

import android.content.ContentValues
import android.content.Context
import android.net.Uri
import android.os.Build
import android.provider.MediaStore
import org.json.JSONArray
import org.json.JSONObject

/**
 * Everything the person has entered, in one file they own: their birth, the people they
 * added, their record of what happened, their journal, remedy streaks and settings.
 * Nothing leaves the phone unless the person shares the file. A copy is also kept in
 * Downloads/Kshana (Android 10 and later) so an uninstall or a cleared storage never
 * loses it; the first screen offers to restore it.
 */
object Backup {
    private val STORES = listOf("kshana", "kshana.calibration")
    /** Keys that describe this install, not the person; never carried across. */
    private val SKIP = setOf("sunrise.next", "sunset.next", "alerts.json", "widget.line1", "widget.line2", "widget.line3", "widget.line4", "widget.score", "care.asks")
    const val FILE_NAME = "kshana-backup.json"

    fun toJson(context: Context): String {
        val root = JSONObject().put("app", "kshana").put("version", 1).put("saved", System.currentTimeMillis())
        for (name in STORES) {
            val prefs = context.applicationContext.getSharedPreferences(name, Context.MODE_PRIVATE)
            val o = JSONObject()
            for ((k, v) in prefs.all) {
                if (k in SKIP || v == null) continue
                val e = JSONObject()
                when (v) {
                    is String -> e.put("t", "s").put("v", v)
                    is Boolean -> e.put("t", "b").put("v", v)
                    is Int -> e.put("t", "i").put("v", v)
                    is Long -> e.put("t", "l").put("v", v)
                    is Float -> e.put("t", "f").put("v", v.toDouble())
                    is Set<*> -> e.put("t", "ss").put("v", JSONArray(v.map { it.toString() }))
                    else -> continue
                }
                o.put(k, e)
            }
            root.put(name, o)
        }
        return root.toString(2)
    }

    /** Replace everything with the file's contents. Returns the person's name, or null if the file is not a Kshana backup. */
    fun fromJson(context: Context, text: String): String? {
        val root = runCatching { JSONObject(text) }.getOrNull() ?: return null
        if (root.optString("app") != "kshana") return null
        for (name in STORES) {
            val o = root.optJSONObject(name) ?: continue
            val ed = context.applicationContext.getSharedPreferences(name, Context.MODE_PRIVATE).edit()
            ed.clear()
            for (k in o.keys()) {
                val e = o.getJSONObject(k)
                when (e.optString("t")) {
                    "s" -> ed.putString(k, e.getString("v"))
                    "b" -> ed.putBoolean(k, e.getBoolean("v"))
                    "i" -> ed.putInt(k, e.getInt("v"))
                    "l" -> ed.putLong(k, e.getLong("v"))
                    "f" -> ed.putFloat(k, e.getDouble("v").toFloat())
                    "ss" -> {
                        val a = e.getJSONArray("v")
                        ed.putStringSet(k, (0 until a.length()).map { a.getString(it) }.toSet())
                    }
                }
            }
            ed.commit()
        }
        Lang.load(context)
        return ProfileStore.prefs(context).getString("profile.name", "") ?: ""
    }

    fun write(context: Context, uri: Uri): Boolean = runCatching {
        context.contentResolver.openOutputStream(uri, "wt")?.use { it.write(toJson(context).toByteArray()) } != null
    }.getOrDefault(false)

    fun read(context: Context, uri: Uri): String? = runCatching {
        val text = context.contentResolver.openInputStream(uri)?.use { it.readBytes().toString(Charsets.UTF_8) } ?: return null
        fromJson(context, text)
    }.getOrNull()

    /** Keep a copy in Downloads/Kshana, overwriting the last one. Quietly does nothing before Android 10. */
    fun autoSave(context: Context) {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.Q) return
        if (!ProfileStore.prefs(context).getBoolean("profile.has", false)) return
        if (!ProfileStore.prefs(context).getBoolean(K_AUTO, true)) return
        runCatching {
            val resolver = context.contentResolver
            val collection = MediaStore.Downloads.EXTERNAL_CONTENT_URI
            val rel = "Download/Kshana/"
            var uri: Uri? = null
            resolver.query(
                collection, arrayOf(MediaStore.MediaColumns._ID),
                "${MediaStore.MediaColumns.DISPLAY_NAME}=? AND ${MediaStore.MediaColumns.RELATIVE_PATH}=?",
                arrayOf(FILE_NAME, rel), null,
            )?.use { c -> if (c.moveToFirst()) uri = android.content.ContentUris.withAppendedId(collection, c.getLong(0)) }
            if (uri == null) {
                val v = ContentValues().apply {
                    put(MediaStore.MediaColumns.DISPLAY_NAME, FILE_NAME)
                    put(MediaStore.MediaColumns.MIME_TYPE, "application/json")
                    put(MediaStore.MediaColumns.RELATIVE_PATH, rel)
                }
                uri = resolver.insert(collection, v)
            }
            uri?.let { u -> resolver.openOutputStream(u, "wt")?.use { it.write(toJson(context).toByteArray()) } }
            ProfileStore.prefs(context).edit().putLong(K_LAST, System.currentTimeMillis()).apply()
        }
    }

    private const val K_AUTO = "backup.auto"
    private const val K_LAST = "backup.last"
    fun autoOn(context: Context) = ProfileStore.prefs(context).getBoolean(K_AUTO, true)
    fun setAuto(context: Context, on: Boolean) = ProfileStore.prefs(context).edit().putBoolean(K_AUTO, on).apply()
    fun lastSaved(context: Context): Long = ProfileStore.prefs(context).getLong(K_LAST, 0L)
}

/**
 * The file picker for a restore, opening straight in Downloads › Kshana where the
 * automatic copy lives, so the person does not have to hunt for it.
 */
class OpenBackup : androidx.activity.result.contract.ActivityResultContract<Unit, android.net.Uri?>() {
    override fun createIntent(context: android.content.Context, input: Unit): android.content.Intent {
        val i = android.content.Intent(android.content.Intent.ACTION_OPEN_DOCUMENT)
            .addCategory(android.content.Intent.CATEGORY_OPENABLE)
            .setType("*/*")
            .putExtra(android.content.Intent.EXTRA_MIME_TYPES, arrayOf("application/json", "text/plain", "application/octet-stream"))
        if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.O) {
            val start = android.provider.DocumentsContract.buildDocumentUri("com.android.externalstorage.documents", "primary:Download/Kshana")
            i.putExtra(android.provider.DocumentsContract.EXTRA_INITIAL_URI, start)
        }
        return i
    }

    override fun parseResult(resultCode: Int, intent: android.content.Intent?): android.net.Uri? =
        if (resultCode == android.app.Activity.RESULT_OK) intent?.data else null
}
