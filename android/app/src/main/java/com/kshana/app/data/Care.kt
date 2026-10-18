package com.kshana.app.data

import android.content.Context
import org.json.JSONArray

/**
 * Notices when someone keeps returning to a painful question (will they come back,
 * will I recover) so the app can answer gently and point to people who can help.
 * Counts are kept on the phone for seven days only.
 */
object Care {
    private const val KEY = "care.asks"
    private const val WEEK = 7 * 86_400_000L
    private val HEAVY = setOf("will_it_return", "will_it_recover", "will_it_happen", "when")

    fun note(context: Context, life: String, question: String) {
        val now = System.currentTimeMillis()
        val a = JSONArray(ProfileStore.prefs(context).getString(KEY, "[]"))
        val keep = JSONArray()
        for (i in 0 until a.length()) {
            val s = a.getString(i)
            val t = s.substringBefore('|').toLongOrNull() ?: continue
            if (now - t < WEEK) keep.put(s)
        }
        keep.put("$now|$life|$question")
        ProfileStore.prefs(context).edit().putString(KEY, keep.toString()).apply()
    }

    /** True when the same painful question about love or health was asked three or more times this week, or a health answer reads very low. */
    fun heavy(context: Context, life: String, question: String, score: Double): Boolean {
        if (life == "Health" && score < 32) return true
        if (life !in setOf("Love", "Health", "Children", "Family") || question !in HEAVY) return false
        val a = JSONArray(ProfileStore.prefs(context).getString(KEY, "[]"))
        var n = 0
        for (i in 0 until a.length()) {
            val p = a.getString(i).split('|')
            if (p.size == 3 && p[1] == life && p[2] in HEAVY) n++
        }
        return n >= 3
    }
}
