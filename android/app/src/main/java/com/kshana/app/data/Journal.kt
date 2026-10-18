package com.kshana.app.data

import android.content.Context
import org.json.JSONArray
import org.json.JSONObject
import java.time.LocalDate

/** Something that happened in the person's life, in their own words. */
data class LifeEvent(
    val id: Long,
    val date: LocalDate,
    val life: String,
    val good: Boolean,
    val note: String,
)

/**
 * The person's own record of their life: a job, a quarrel, an illness, a move. Each
 * entry is compared with what the app read for that day, sharpens the birth time, and
 * grades the app honestly. Stored on the phone, carried in the backup file.
 */
object Journal {
    private const val KEY = "journal.json"

    fun all(context: Context): List<LifeEvent> {
        val raw = ProfileStore.prefs(context).getString(KEY, null) ?: return emptyList()
        return runCatching {
            val a = JSONArray(raw)
            (0 until a.length()).map { i ->
                val o = a.getJSONObject(i)
                LifeEvent(o.getLong("id"), LocalDate.parse(o.getString("date")), o.getString("life"), o.getBoolean("good"), o.optString("note"))
            }.sortedByDescending { it.date }
        }.getOrDefault(emptyList())
    }

    private fun save(context: Context, list: List<LifeEvent>) {
        val a = JSONArray()
        for (e in list) a.put(JSONObject().put("id", e.id).put("date", e.date.toString()).put("life", e.life).put("good", e.good).put("note", e.note))
        ProfileStore.prefs(context).edit().putString(KEY, a.toString()).apply()
    }

    fun add(context: Context, date: LocalDate, life: String, good: Boolean, note: String) {
        save(context, all(context) + LifeEvent(System.currentTimeMillis(), date, life, good, note.trim()))
    }

    fun remove(context: Context, id: Long) {
        save(context, all(context).filter { it.id != id })
    }
}
